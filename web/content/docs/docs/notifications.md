---
title: "Notifications"
description: "Desktop notification system with native OS backends, in-app toasts, and persistent inbox"
---

## Module structure

```
src/notifications/
├── mod.rs              # Public API re-exports
├── service.rs          # NotificationService, dispatcher, globals
├── inbox.rs            # Persistent inbox store (NotificationInboxState)
└── backend/
    ├── mod.rs          # NotificationBackend trait
    ├── user_notify.rs  # Primary backend (macOS UserNotifications)
    └── notify_rust.rs  # Fallback backend (libnotify/DBus/Windows)
```

The `views/notifications.rs` file renders the inbox page in the app sidebar.

## NotificationBackend trait

All native backends implement a common async trait:

```rust
#[async_trait]
pub trait NotificationBackend: Send + Sync {
    fn kind(&self) -> NotificationBackendKind;
    fn capabilities(&self) -> NotificationCapabilities;
    async fn refresh_permission_state(&self) -> NotificationPermissionState;
    async fn request_permission(&self) -> NotificationPermissionState;
    async fn send(&self, request: &NotificationRequest) -> anyhow::Result<()>;
}
```

`NotificationService` holds an `Option<Arc<dyn NotificationBackend>>` for the primary backend and a required `Arc<dyn NotificationBackend>` for the fallback. On send, it tries the primary first. If the primary fails or is absent, it falls back to the secondary. If both fail, delivery degrades to in-app only.

## Backend selection

| Backend | Crate | When used | Interactive | Permission API |
|---------|-------|-----------|-------------|----------------|
| `UserNotifyBackend` | `user-notify` | Primary when available (macOS bundled app) | Yes (actions, reply) | Yes |
| `NotifyRustBackend` | `notify-rust` | Fallback (Linux, Windows, unbundled macOS) | No | No |
| In-app toast | `gpui-component` | Degraded mode when native delivery fails | No | No |

`NotificationService::new()` attempts `UserNotifyBackend::new()`. If that returns an error, the primary is `None` and the service logs the failure reason. `NotifyRustBackend` is always initialized as the secondary.

## Backend capabilities

```rust
pub struct NotificationCapabilities {
    pub can_request_permission: bool,
    pub can_read_permission_state: bool,
    pub can_send_immediate_native: bool,
    pub can_send_interactive: bool,
    pub requires_packaged_runtime: bool,
}
```

| Capability | UserNotify (macOS) | NotifyRust (Linux) | NotifyRust (Windows) |
|------------|-------------------|--------------------|---------------------|
| `can_request_permission` | Yes | No | No |
| `can_read_permission_state` | Yes | No | No |
| `can_send_immediate_native` | Yes | Yes | Yes |
| `can_send_interactive` | Yes | No | No |
| `requires_packaged_runtime` | Yes | No | Yes |

## Permission handling

```rust
pub enum NotificationPermissionState {
    Unknown,
    Unsupported,
    Unavailable(String),
    NotDetermined,
    Denied,
    Authorized,
}
```

| Platform | How permissions work |
|----------|---------------------|
| macOS (bundled) | Uses `UNUserNotificationCenter`. Calls `getNotificationSettingsWithCompletionHandler` to read state, `first_time_ask_for_notification_permission` to request. Requires a valid bundle identifier. |
| macOS (unbundled) | Falls back to `NotifyRustBackend`. Permission state is `Unavailable`. |
| Linux | `notify-rust` sends via libnotify/DBus. No permission model. |
| Windows | `notify-rust` sends via Windows toast XML. Permission state is `Unsupported`. |

To open the system notification settings panel on macOS:

```rust
notifications::open_system_settings(cx);
```

This opens `x-apple.systempreferences:com.apple.Notifications-Settings.extension`.

## Notification inbox

Every send attempt, permission change, and settings update is recorded in a persistent inbox backed by `NotificationInboxState` (a GPUI global).

### InboxEntry fields

```rust
pub struct NotificationInboxItem {
    pub id: NotificationId,
    pub created_at: AppTimestamp,
    pub title: String,
    pub body: String,
    pub read: bool,
    pub backend: String,
    pub delivered_natively: bool,
    pub degraded: bool,
    pub error_summary: Option<String>,
    pub kind: NotificationInboxKind,
}
```

### Inbox kinds

| Kind | Recorded when |
|------|--------------|
| `Attempt` | A notification is sent (or fails) |
| `PermissionUpdate` | Permission state changes after a request |
| `SettingsUpdate` | User toggles native notifications on or off |

### Inbox operations

| Function | Description |
|----------|-------------|
| `inbox::initialize(cx)` | Load inbox from persisted config |
| `inbox::snapshot(cx)` | Read current items |
| `inbox::record(item, cx)` | Prepend an item, cap at 200 |
| `inbox::mark_all_read(cx)` | Mark every item as read |
| `inbox::clear_all(cx)` | Remove all items |

The inbox is persisted to `target/state.json` via `app_state::update_config`. The max inbox size is 200 items (`MAX_INBOX_ITEMS`).

### Sidebar integration

`views/notifications.rs` renders the inbox page. It observes `NotificationInboxState` and displays unread count, per-item metadata (backend, timestamp, error), and "Mark all read" / "Clear all" buttons.

## Sending a notification

Build a `NotificationRequest` and call `send_from_window`:

```rust
use crate::notifications::{NotificationRequest, send_from_window};

// Simple foreground notification
let request = NotificationRequest::foreground("Title", "Body text");
send_from_window(request, window, cx);
```

### NotificationRequest builders

| Method | Use case |
|--------|----------|
| `foreground(title, body)` | Standard notification with sound |
| `action_buttons(title, body)` | Includes Open/Snooze action buttons (macOS) |
| `reply(title, body)` | Includes a text reply input (macOS) |
| `background_worthy(title, body)` | Higher importance, will not show in-app fallback |

### NotificationRequest fields

```rust
pub struct NotificationRequest {
    pub title: SharedString,
    pub body: SharedString,
    pub play_sound: bool,          // default: true
    pub thread_id: Option<String>, // groups notifications
    pub category: Option<String>,  // maps to interactive action sets
    pub prefer_native: bool,       // default: true; false skips native
    pub importance: NotificationImportance,
}
```

### Importance levels

| Level | Behavior |
|-------|----------|
| `ForegroundOnly` | If native delivery fails, an in-app toast is shown |
| `BackgroundWorthy` | No in-app fallback on native failure; marked degraded |

### Send flow

1. `send_from_window` checks `enabled_by_user` and `permission != Denied`
2. If disabled or denied, returns `UiOnly` result immediately
3. Attempts primary backend, then secondary on failure
4. Records the attempt in the inbox
5. If native delivery failed and importance is `ForegroundOnly`, pushes an in-app toast via `window.push_notification`

### Send result

```rust
pub struct NotificationSendResult {
    pub backend_used: NotificationBackendKind,
    pub degraded: bool,
    pub delivered_natively: bool,
    pub error_summary: Option<SharedString>,
    pub importance: NotificationImportance,
}
```

## Initialization

Call `notifications::initialize(cx)` during app startup. This:

1. Creates `NotificationService` (selects backends)
2. Builds a `NotificationRuntimeSnapshot`
3. Installs `NativeNotificationState` as a GPUI global
4. Registers the capability with the app's capability system
5. Starts an async permission state refresh

```rust
// In app initialization
notifications::initialize(cx);
notifications::inbox::initialize(cx);
```

### Runtime snapshot

```rust
pub fn snapshot(cx: &App) -> NotificationRuntimeSnapshot {
    // active_backend, permission, capabilities, degraded_reason, etc.
}
```

Use this to display notification status in settings UI.

## Toggling notifications

```rust
// Enable or disable native notifications at runtime
notifications::set_native_notifications_enabled(false, cx);
```

This updates the persisted config, mutates the snapshot, records a `SettingsUpdate` in the inbox, and updates the capability system.

## Adding a custom backend

1. Create a new file in `src/notifications/backend/`, for example `my_backend.rs`.

2. Implement `NotificationBackend`:

```rust
use async_trait::async_trait;
use super::NotificationBackend;
use crate::notifications::{
    NotificationBackendKind, NotificationCapabilities,
    NotificationPermissionState, NotificationRequest,
};

pub struct MyBackend;

#[async_trait]
impl NotificationBackend for MyBackend {
    fn kind(&self) -> NotificationBackendKind {
        NotificationBackendKind::UiOnly // or add a new variant
    }

    fn capabilities(&self) -> NotificationCapabilities {
        NotificationCapabilities {
            can_request_permission: false,
            can_read_permission_state: false,
            can_send_immediate_native: true,
            can_send_interactive: false,
            requires_packaged_runtime: false,
        }
    }

    async fn refresh_permission_state(&self) -> NotificationPermissionState {
        NotificationPermissionState::Unsupported
    }

    async fn request_permission(&self) -> NotificationPermissionState {
        NotificationPermissionState::Unsupported
    }

    async fn send(&self, request: &NotificationRequest) -> anyhow::Result<()> {
        // your delivery logic
        Ok(())
    }
}
```

3. Register the module in `backend/mod.rs`:

```rust
mod my_backend;
pub use my_backend::MyBackend;
```

4. Wire it into `NotificationService::new()` in `service.rs`, either replacing the primary or adding it as an intermediate fallback before `NotifyRustBackend`.
