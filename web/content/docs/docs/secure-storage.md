---
title: "Secure storage"
description: "Credential storage using OS keyring with database reference pattern"
---

## Overview

The `secure_storage` module (`src/secure_storage.rs`) wraps the `keyring` crate to store secrets in the operating system credential store. The application never writes secret values to SQLite or log files. Only opaque references (service name and key) are kept in the database.

## Platform credential stores

The `keyring` crate (version 3.6.3) delegates to the native credential manager on each platform:

| Platform | Backend |
|----------|---------|
| macOS | Keychain Services (via `apple-native` feature) |
| Linux | Secret Service API (GNOME Keyring, KDE Wallet) |
| Windows | Credential Manager |

No extra configuration is required. The crate selects the correct backend at compile time.

## Availability check

On startup, `secure_storage::initialize(cx)` probes the keyring by creating a test entry. If the probe fails, the module registers itself as degraded in the capability registry:

```rust
let available = keyring::Entry::new("gpui-starter", "availability-check").is_ok();
```

The global `SecureStorageSnapshot` tracks whether the keyring is reachable and records the last error. Check it at any time:

```rust
let snap = secure_storage::snapshot(cx);
if !snap.available {
    // keyring is unreachable, degrade gracefully
}
```

The capability registry entry (`"secure_storage"`) exposes `supported`, `enabled`, `degraded`, and `last_error` fields for the diagnostics view.

## API

All functions take `&mut App` so they can update the global error state.

### set_secret

```rust
pub fn set_secret(
    service: &str,
    key: &str,
    value: &str,
    cx: &mut App,
) -> Result<(), String>
```

Writes a secret under the given service and key. On success, clears `last_error`. On failure, logs the error and updates `last_error`.

### get_secret

```rust
pub fn get_secret(
    service: &str,
    key: &str,
    cx: &mut App,
) -> Result<Option<SharedString>, String>
```

Returns `Some(value)` if the entry exists, `None` if it does not (`keyring::Error::NoEntry`), or an error string on failure.

### delete_secret

```rust
pub fn delete_secret(
    service: &str,
    key: &str,
    cx: &mut App,
) -> Result<(), String>
```

Deletes the credential. Fails if the entry does not exist or the keyring is unreachable.

## Reference pattern: opaque IDs in SQLite

The architecture separates secret values from structured data. The database stores only references, while the keyring holds the actual values.

```sql
-- WRONG: secret value in the database
INSERT INTO environments (name, api_token) VALUES ('prod', 'sk-abc123');

-- CORRECT: opaque reference in the database
INSERT INTO secret_refs (id, service, key) VALUES (?, 'gpui-starter', 'prod-api-token');
-- actual value lives in the OS keyring
```

To read a secret at runtime, look up the reference row, then call `get_secret` with the stored service and key.

## Error handling

Every function logs through `tracing` with the target `gpui_starter::secure_storage`. Errors are returned as `Result<_, String>` so callers can display them in the UI. The pattern used in the settings view:

```rust
let message = match secure_storage::set_secret("gpui-starter", "demo-token", "demo-value", cx) {
    Ok(()) => "Secure value written".to_string(),
    Err(err) => format!("Write failed: {err}"),
};
window.push_notification(message, cx);
```

The `last_error` field in `SecureStorageSnapshot` is cleared on success and set on failure, giving the diagnostics page a live view of keyring health.

## Testing

The `testing` module provides `FakeSecureStorage` for unit tests that do not touch the OS keyring:

```rust
#[derive(Default)]
pub struct FakeSecureStorage {
    value: Option<String>,
}

impl FakeSecureStorage {
    pub fn set(&mut self, value: &str) { self.value = Some(value.to_string()); }
    pub fn get(&self) -> Option<String> { self.value.clone(); }
    pub fn delete(&mut self) { self.value = None; }
}
```

Usage in a test:

```rust
let mut storage = FakeSecureStorage::default();
storage.set("token");
assert_eq!(storage.get().as_deref(), Some("token"));
storage.delete();
assert_eq!(storage.get(), None);
```

## Export and import with redaction

When exporting application data (telemetry config, environment snapshots), secrets must never appear in the output. The telemetry module demonstrates the redaction pattern:

```rust
fn redact_endpoint(endpoint: &str) -> Option<String> {
    let host = endpoint
        .split("://")
        .nth(1)
        .unwrap_or(endpoint)
        .split('/')
        .next()
        .unwrap_or(endpoint);
    Some(format!("{host}/..."))
}
```

Rules for export/import flows:

- Replace secret values with placeholders or omit them entirely.
- Store only the service/key reference pair so the imported config can rebind secrets on the target machine.
- Never log raw secret values. The `tracing` calls in `secure_storage.rs` log the service and key, never the value.

## Function reference

| Function | Returns | Failure mode |
|----------|---------|--------------|
| `initialize(cx)` | Sets global `SecureStorageSnapshot` and capability | Marks degraded if keyring probe fails |
| `snapshot(cx)` | `SecureStorageSnapshot` | Returns default if not initialized |
| `set_secret(service, key, value, cx)` | `Result<(), String>` | Error on entry creation or write failure |
| `get_secret(service, key, cx)` | `Result<Option<SharedString>, String>` | `None` if no entry, error on read failure |
| `delete_secret(service, key, cx)` | `Result<(), String>` | Error if entry missing or keyring unreachable |
