---
question: "How are secrets and credentials stored?"
description: "Secrets are stored in the OS keyring (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux), never in the database."
category: "Features"
order: 13
---

Secrets like API keys, tokens, and passwords are never written to SQLite or any plain file on disk. Instead, gpui-starter delegates storage to the operating system's native credential store using the `keyring` crate.

## How it works

The `secure_storage` module provides a thin wrapper around the keyring. You store and retrieve values by key:

```rust
secure_storage::set("api-key", &secret_value)?;
let value = secure_storage::get("api-key")?;
```

Values are never logged, cached in memory beyond the current request, or written to the app's data directory. The database stores a reference flag indicating a secret exists, but not the secret itself.

## Cross-platform backends

The `keyring` crate maps to the correct native backend on each platform. On macOS it uses Keychain Services, on Windows it uses Credential Manager, and on Linux it uses Secret Service via D-Bus (GNOME Keyring and KDE Wallet).

No extra configuration is needed. The crate detects the platform at compile time and links the right backend. If no keyring is available on Linux (headless server, no D-Bus), the call returns an error rather than falling back to insecure storage.
