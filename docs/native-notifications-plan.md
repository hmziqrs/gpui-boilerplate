# Native Local Notifications Plan

## Goal

Add native OS-level local notifications to this GPUI app with:

- a proper macOS permission flow,
- a cross-platform backend shape for macOS, Linux, and Windows,
- and explicit backend fallback behavior when the preferred native path is unavailable.

## Current State

- The app only shows in-window GPUI toasts through `window.push_notification(...)`.
- Current call sites:
  - `src/views/settings.rs`
  - `src/views/form_page.rs`
  - `src/title_bar.rs`
- Good integration seams already exist:
  - `src/app.rs` for global state and startup wiring
  - `src/main.rs` for module registration
  - `src/tray.rs` for macOS-specific runtime behavior
- The current local build output is a raw binary at `target/debug/gpui-starter`.
- There is no checked-in `.app` bundle path, `Info.plist`, or signing workflow yet.

## Technical Choice

### Primary backend

Use `user-notify` as the primary notification abstraction.

Why:

- It is explicitly cross-platform across `macOS`, `Linux`, and `Windows`.
- It exposes higher-level notification management instead of just "fire and forget".
- On macOS it uses the modern `UNUserNotificationCenter` stack via `objc2-user-notifications`.
- It exposes macOS permission APIs we need:
  - permission state lookup
  - first-time permission request
- It supports richer features than the simpler crates:
  - categories
  - actions
  - notification metadata
  - active notification queries

Important caveat:

- `user-notify` is `LGPL-3.0-or-later`.
- We should proceed only if that license is acceptable for this app's distribution model.

### Secondary backend

Use `notify-rust` as the explicit fallback backend.

Why:

- It is mature and established.
- It has working Linux, macOS, and Windows delivery paths.
- It is appropriate for best-effort OS notification delivery when the richer `user-notify` path cannot be initialized or when only simple summary/body delivery is needed.

Limitations:

- It is not the right place to drive macOS permission UX.
- Its macOS path uses `mac-notification-sys`, which is less capable than the `UNUserNotificationCenter` path.
- It should be treated as a degraded delivery backend, not the source of truth for permission state.

### Final fallback

Keep GPUI in-window toasts as the last-resort fallback.

That gives us a three-layer delivery model:

1. `user-notify`
2. `notify-rust`
3. `window.push_notification(...)`

## Platform Scope

The architecture should be cross-platform from the start, but implementation can be staged.

Recommended rollout:

1. Build the full backend abstraction now.
2. Validate macOS first, because it has the hardest runtime constraints:
   - bundle ID
   - signing
   - permission prompt
3. Add Linux and Windows verification next using the same abstraction.

This avoids rewriting the service later while still prioritizing the platform with the most failure modes.

## Permission Model

### macOS

macOS is the only platform in this plan where we need an app-managed permission flow.

Behavior:

- Read permission state through `user-notify`.
- Request permission only through `user-notify`.
- If permission is denied, do not re-prompt. Show UI guidance and an "Open Settings" recovery path.
- If the app is not bundled/signed correctly, treat the primary backend as unavailable and surface that clearly in Settings.

### Linux

No app-level notification permission prompt is expected in the same sense.

Behavior:

- Attempt native delivery.
- If the desktop notification daemon is unavailable, fall back to GPUI toast.

### Windows

No app-driven permission flow is planned in the same sense as macOS.

Behavior:

- Attempt native delivery.
- If the platform cannot show toasts because app identity/runtime requirements are not met, fall back to GPUI toast.

## Backend Strategy

Add an app-local notification service that owns backend selection and fallback rules.

### Backend order

At startup:

1. Try to initialize `user-notify`.
2. If that fails, initialize `notify-rust`.
3. If that also fails or is unavailable, mark the service as `UiToastOnly`.

At send time:

1. Try the selected backend.
2. If send fails, try the next backend.
3. If all native paths fail, show a GPUI toast and log the backend errors.

### Fallback rules

#### `user-notify` -> `notify-rust`

Use this fallback when:

- `user-notify` manager initialization fails.
- macOS runtime constraints are missing:
  - no bundle ID
  - no valid app package
  - signing/runtime requirements not satisfied
- a requested feature is unsupported by the current `user-notify` platform backend and a simple notification is still acceptable.

Do not use this fallback for:

- macOS permission lookup
- macOS permission requests

Those must stay primary-backend-only. If the primary backend is unavailable, the Settings UI should say that permission-aware native notifications are unavailable in the current build/runtime.

#### `notify-rust` -> GPUI toast

Use this fallback when:

- the OS notification daemon or API path is unavailable,
- the send call errors,
- or the user has disabled native notifications in app settings.

## Proposed Architecture

### Module layout

- `src/notifications/mod.rs`
  - public API
  - shared types
- `src/notifications/service.rs`
  - backend selection
  - send-time fallback logic
- `src/notifications/backend/user_notify.rs`
  - `user-notify` adapter
- `src/notifications/backend/notify_rust.rs`
  - `notify-rust` adapter
- `src/notifications/backend/mod.rs`
  - backend trait + wiring

Optional:

- `src/notifications/backend/ui_only.rs`
  - explicit no-native backend used when we only want GPUI fallback

### Core types

Suggested types:

- `NotificationPermissionState`
  - `Unknown`
  - `NotDetermined`
  - `Denied`
  - `Authorized`
  - `Unsupported`
  - `Unavailable`
- `NotificationBackendKind`
  - `UserNotify`
  - `NotifyRust`
  - `UiToastOnly`
- `NotificationRequest`
  - `title`
  - `body`
  - `deliver_after`
  - `play_sound`
  - `thread_id`
  - `category`
  - `prefer_native`
- `NotificationSendResult`
  - `backend_used`
  - `degraded`
  - `error_summary`

### Backend trait

Define a trait around the features we actually need:

- `backend_kind()`
- `capabilities()`
- `refresh_permission_state()`
- `request_permission()`
- `send()`
- `open_system_settings()`

Capabilities should explicitly model whether the backend can:

- request permission,
- read permission state,
- send delayed notifications,
- send interactive notifications.

This prevents the UI from assuming every backend can do everything.

### Global state

In `src/app.rs`, register global notification state such as:

- `NativeNotificationState`
  - `enabled_by_user: bool`
  - `permission: NotificationPermissionState`
  - `active_backend: NotificationBackendKind`
  - `last_backend_error: Option<SharedString>`

This lets the Settings page explain both the OS permission state and the runtime-selected backend.

## Settings UX

Replace the current demo "Notify" row with a real notification settings section.

### Required UI

- Native notifications enabled switch
- Active backend label
- Permission state label
- "Request permission" button when supported and `NotDetermined`
- "Open Settings" button when permission is denied on macOS
- "Test native notification" button
- Small explanatory text when the app is running in a degraded backend mode

### Suggested messaging

Examples:

- `Native backend: user-notify`
- `Native backend: notify-rust (fallback)`
- `Native backend unavailable in this build; using in-app toasts`

This matters because otherwise debugging dev-build failures on macOS will be opaque.

## File-by-File Implementation Plan

### 1. Add dependencies

Files:

- `Cargo.toml`

Work:

- Add `user-notify`.
- Add direct `notify-rust`.

Use both as direct dependencies even though `user-notify` already depends on `notify-rust` internally on some paths. We want our own explicit fallback backend instead of relying on transitive internals.

### 2. Add the notification service and backend adapters

Files:

- `src/main.rs`
- `src/notifications/mod.rs`
- `src/notifications/service.rs`
- `src/notifications/backend/mod.rs`
- `src/notifications/backend/user_notify.rs`
- `src/notifications/backend/notify_rust.rs`

Work:

- Add the `notifications` module.
- Define the backend trait and shared request/result types.
- Implement a `user-notify` adapter.
- Implement a `notify-rust` adapter.
- Implement startup backend selection.
- Implement send-time fallback chaining.

### 3. Initialize state at startup

Files:

- `src/app.rs`

Work:

- Create the notification service during `app::init`.
- Store active backend and permission state in app-global state.
- On startup:
  - try `user-notify`
  - otherwise fall back to `notify-rust`
  - otherwise mark `UiToastOnly`
- Refresh permission state asynchronously after startup when the selected backend supports it.

### 4. Build the Settings permission and fallback UI

Files:

- `src/views/settings.rs`
- `i18n/en/gpui-starter.ftl`
- `i18n/zh-CN/gpui-starter.ftl`

Work:

- Replace the demo notification button with a proper settings section.
- Show backend and permission status.
- Gate permission actions by backend capability.
- Add a "Test native notification" action that always goes through the notification service, not directly to GPUI toasts.

### 5. Route current call sites through the service

Files:

- `src/views/settings.rs`
- `src/views/form_page.rs`
- `src/title_bar.rs`

Work:

- Replace direct `window.push_notification(...)` calls where native delivery is intended.
- Preserve GPUI toasts for foreground-only feedback where native OS delivery would be noisy.

Suggested first pass:

- Settings "Test notification": always service-based.
- Form submit success: service-based only when native notifications are enabled and the app decides the event is background-worthy.
- Title-bar bell: keep as GPUI toast until the app has a real notification feed.

### 6. Add macOS bundle/signing support for the primary backend

Files:

- `build.rs` or a new script such as `scripts/macos-dev-app.sh`
- optional bundled metadata file

Work:

- Build a minimal `.app` bundle for local testing.
- Set a stable bundle identifier.
- Add ad-hoc signing or a documented signing path for local development.

This is required because `user-notify`'s macOS path expects a real app package and bundle ID, and its docs explicitly call out signing/runtime constraints.

### 7. Implement a capability-aware fallback policy

Files:

- `src/notifications/service.rs`
- `src/notifications/backend/user_notify.rs`
- `src/notifications/backend/notify_rust.rs`

Work:

- If permission state APIs are unavailable, report `Unsupported` or `Unavailable` instead of guessing.
- If a request includes unsupported rich features on the active backend, degrade it to a simple summary/body notification before failing over.
- Persist the last backend failure so the Settings page can explain why the fallback was selected.

## Fallback Behavior Spec

### Startup fallback

Expected order:

1. `user-notify`
2. `notify-rust`
3. `UiToastOnly`

### Send fallback

Expected order:

1. Primary backend send attempt
2. Secondary backend send attempt
3. GPUI toast fallback

### Permission fallback

Expected order:

1. Use `user-notify` if it is active and supports permission APIs
2. Otherwise report permission management as unavailable in the current runtime
3. Do not fake a permission result from `notify-rust`

### Example degraded scenarios

- Raw `cargo run` on macOS without bundle/signing:
  - likely no working `user-notify` macOS path
  - `notify-rust` may or may not deliver
  - Settings should show degraded backend state
  - GPUI toast must still work

- Linux desktop with no notification daemon:
  - native backends fail at send time
  - GPUI toast becomes final fallback

- Windows environment without toast identity/runtime support:
  - native send fails
  - GPUI toast becomes final fallback

## Verification Plan

### Build verification

- `cargo check`
- `cargo build`

Confirm:

- both crates compile in this workspace,
- backend selection compiles on macOS,
- Linux/Windows paths remain guarded appropriately if cross-compiling is deferred.

### Manual verification on macOS

1. Launch from the bundled `.app`.
2. Confirm active backend is `user-notify`.
3. Confirm permission state is readable.
4. Request permission.
5. Accept permission and trigger a test notification.
6. Deny permission and relaunch.
7. Confirm the app:
   - reads `Denied`,
   - does not re-prompt,
   - offers recovery UI,
   - can still fall back to GPUI toast.

### Manual fallback verification on macOS

1. Launch a dev build that lacks the proper runtime shape.
2. Confirm backend degrades away from `user-notify`.
3. Trigger a test notification.
4. Confirm:
   - fallback backend selection is visible,
   - send either works through `notify-rust` or degrades to GPUI toast,
   - the app never lies about permission state.

### Manual verification on Linux

1. Trigger a simple native notification.
2. Confirm it uses the native backend when a notification daemon exists.
3. Kill or disable the notification daemon if practical.
4. Confirm send failure degrades to GPUI toast.

### Manual verification on Windows

1. Trigger a simple native notification.
2. Confirm it uses the native backend when toast delivery is available.
3. Confirm GPUI fallback behavior if toast delivery is unavailable.

## Acceptance Criteria

- The app has a single notification service API that views call into.
- `user-notify` is the preferred backend when available.
- `notify-rust` is used as an explicit degraded fallback backend.
- GPUI toast remains the final fallback.
- macOS permission state and permission requests are handled only through the primary backend.
- The Settings page shows:
  - current backend
  - permission state
  - degraded/fallback status
- The app never silently drops notification requests without either native delivery or in-app feedback.

## Risks and Open Questions

### Risk: LGPL from `user-notify`

This is the biggest product/legal caveat in the new plan.

If LGPL is unacceptable, we should stop before implementation and switch to a fully app-local multi-backend facade based on lower-level crates.

### Risk: macOS fallback may still be weak in dev mode

`notify-rust` on macOS is a degraded path, not a guaranteed escape hatch. It may not rescue every unbundled local dev run.

That is why GPUI toast remains the final fallback, and why the Settings page must expose the selected backend clearly.

### Risk: duplicate delivery

If we send both a native banner and a GPUI toast for the same foreground event, the UX will feel noisy.

Default policy should be:

- background-worthy events -> native
- immediate foreground feedback -> GPUI toast

## Suggested Implementation Order

1. Add both dependencies and the service module.
2. Implement `user-notify` adapter.
3. Implement `notify-rust` adapter.
4. Add backend selection and send fallback logic.
5. Add app-global state for backend + permission visibility.
6. Replace the Settings demo row with real status and controls.
7. Add the macOS `.app` bundle/signing dev path.
8. Route the first real notification event through the service.
9. Expand usage to the remaining call sites after UX policy is validated.

## References

- Apple User Notifications overview: https://developer.apple.com/documentation/UserNotifications
- Apple notification permission guidance: https://developer.apple.com/documentation/UserNotifications/asking-permission-to-use-notifications
- Apple `requestAuthorization(options:completionHandler:)`: https://developer.apple.com/documentation/UserNotifications/UNUserNotificationCenter/requestAuthorization%28options%3AcompletionHandler%3A%29
- Apple `authorizationStatus`: https://developer.apple.com/documentation/usernotifications/unnotificationsettings/authorizationstatus
- `user-notify` docs: https://docs.rs/user-notify/latest/user_notify/
- `user-notify` README/source: https://docs.rs/crate/user-notify/latest/source/Readme.md
- `notify-rust` docs: https://docs.rs/crate/notify-rust/latest
- `notify-rust` repository: https://github.com/hoodie/notify-rust
