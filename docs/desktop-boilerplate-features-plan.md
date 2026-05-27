# Desktop Boilerplate Features Plan

## Goal

Add broadly useful desktop-app infrastructure to this GPUI boilerplate without turning it into a product-specific app.

This plan focuses on reusable app behavior:

- deep links
- single-instance forwarding
- persistent app state
- internal notification inbox
- background task/activity model
- status/activity surface
- diagnostics
- first-run state

Out of scope:

- production packaging
- installers
- auto-update implementation
- document/file-editor workflows
- arbitrary file/folder picker demos unless a feature needs them
- remote push notifications

## Current Architecture Audit

### Existing strengths

- `src/sidebar.rs` already defines the core page enum.
- `src/root.rs` already owns active page state and listens for `PendingNavigation`.
- `src/launcher.rs` already acts like a command palette and can dispatch page navigation.
- `src/app.rs` already centralizes startup, global state, key bindings, and app actions.
- `src/tray.rs` already proves platform event polling and global hotkey integration.
- `src/notifications` already provides native local notification backend state and fallback behavior.
- Settings already exposes backend/status controls.

### Gaps

- Navigation is page-only and not yet URL/deep-link aware.
- `PendingNavigation` is too narrow for future app events.
- There is no single-instance guard or IPC forwarding.
- Settings persistence uses `target/state.json`, which is a dev artifact path.
- Window size, active route, sidebar state, notification preferences, and first-run state are not persisted.
- The bell button has no notification inbox behind it.
- Background work has no shared model for progress, cancellation, and errors.
- There is no status bar or activity area for ambient app state.
- Diagnostics are spread across logs and Settings text, not exposed as a coherent view.

## Implementation Order

### Phase 1: App Routes And Deep Links

Purpose:

- Make external inputs and internal navigation use the same route model.

Files:

- `src/routes.rs`
- `src/sidebar.rs`
- `src/root.rs`
- `src/launcher.rs`
- `src/app.rs`

Work:

- Add `AppRoute`.
- Add route parsing from strings and custom URLs.
- Convert `PendingNavigation` from `Option<Page>` to `Option<AppRoute>`.
- Add route helpers:
  - `AppRoute::page(Page)`
  - `AppRoute::parse_deep_link(&str)`
  - `AppRoute::to_url()`
- Support URLs such as:
  - `gpui-starter://home`
  - `gpui-starter://form`
  - `gpui-starter://settings`
  - `gpui-starter://about`
  - `gpui-starter://settings/notifications`
- Route unknown or invalid links to a logged error and a visible app error.

Acceptance criteria:

- Launcher navigation uses `AppRoute`.
- Sidebar clicks use `AppRoute`.
- Root stores and renders the active route.
- Invalid deep links do not panic.

### Phase 2: Central App Event Bus

Purpose:

- Avoid adding more one-off globals like `PendingNavigation` for every feature.

Files:

- `src/events.rs`
- `src/app.rs`
- `src/root.rs`
- `src/launcher.rs`
- `src/notifications/service.rs`

Work:

- Add an app-local event queue/global.
- Define typed events:
  - `Navigate(AppRoute)`
  - `DeepLinkReceived(String)`
  - `NotificationRecorded(NotificationInboxItem)`
  - `BackgroundTaskChanged(TaskId)`
  - `AppError(AppError)`
  - `DiagnosticsChanged`
- Add helper functions:
  - `events::emit(event, cx)`
  - `events::drain(cx)`
- Root observes the event queue and applies events relevant to UI state.

Acceptance criteria:

- Navigation can be emitted without directly mutating root internals.
- Notification service can record inbox events without owning UI.
- Error and diagnostics events have a single path.

### Phase 3: Persistent App State In User Config Directory

Purpose:

- Move app state out of `target/` and make boilerplate behavior survive relaunch.

Files:

- `Cargo.toml`
- `src/app_state.rs`
- `src/app.rs`
- `src/root.rs`
- `src/views/settings.rs`

Dependencies:

- `directories`

State to persist:

- active route
- theme name/mode
- locale
- scrollbar preference
- sidebar collapsed state
- window size and position
- native notifications enabled
- first-run completed

Policy:

- Use a versioned state struct.
- Read on startup.
- Write on meaningful changes.
- Log parse failures and fall back to defaults.
- Never store secrets.

Acceptance criteria:

- State path lives under the OS user config/data directory.
- Existing theme persistence no longer writes to `target/state.json`.
- Active page and sidebar collapsed state restore after relaunch.
- Settings controls update persisted state.

### Phase 4: Single-Instance And Deep-Link Forwarding

Purpose:

- Prevent duplicate app instances and make external opens route into the running app.

Files:

- `Cargo.toml`
- `src/single_instance.rs`
- `src/app.rs`
- `src/routes.rs`
- `src/events.rs`

Recommended dependency:

- `single-instance` or an app-local local-socket implementation.

Work:

- On startup, acquire an app lock.
- If lock succeeds, start a local IPC listener.
- If lock fails, forward argv/deep-link payload to the running instance and exit.
- Running instance receives payload and emits `DeepLinkReceived`.
- Existing window is focused/raised when a forwarded route arrives.

Important:

- This should be capability-gated per platform if a crate has uneven support.
- It must not break tests or development runs.

Acceptance criteria:

- Second app launch does not create a second tray/hotkey process.
- Forwarded `gpui-starter://settings` opens Settings in the existing app.
- Invalid forwarded payload is logged and ignored safely.

### Phase 5: Internal Notification Inbox

Purpose:

- Native local notifications are transient. The app needs persistent in-app notification history.

Files:

- `src/notifications/inbox.rs`
- `src/notifications/service.rs`
- `src/title_bar.rs`
- `src/views/notifications.rs`
- `src/views/mod.rs`
- `src/sidebar.rs`
- `src/root.rs`

Work:

- Add `NotificationInboxItem`.
- Store:
  - id
  - title
  - body
  - timestamp
  - read/unread state
  - source backend
  - importance
  - optional action metadata
- Record every meaningful app notification before native delivery.
- Bell button opens the inbox instead of showing a throwaway toast.
- Add mark-read, clear-all, and item click behavior.
- Persist recent inbox items in app state with a reasonable cap.

Acceptance criteria:

- Bell shows unread count or state.
- Native notification sends are visible in the inbox.
- Failed/degraded notification attempts are visible in the inbox.
- Inbox survives relaunch.

### Phase 6: Background Task Manager

Purpose:

- Provide a standard model for long-running work.

Files:

- `src/tasks.rs`
- `src/events.rs`
- `src/root.rs`
- `src/views/settings.rs` or a new demo page

Core model:

- `TaskId`
- `TaskStatus`
  - queued
  - running
  - succeeded
  - failed
  - cancelled
- `TaskProgress`
  - indeterminate
  - percent
  - steps
- `BackgroundTask`
  - id
  - label
  - status
  - progress
  - started_at
  - finished_at
  - error

Work:

- Add task registry global.
- Add helpers:
  - `tasks::start`
  - `tasks::update_progress`
  - `tasks::succeed`
  - `tasks::fail`
  - `tasks::cancel`
- Add one demo async task in Settings or launcher.

Acceptance criteria:

- A demo background task can be started.
- Progress is observable by UI.
- Failure is visible through the error surface and diagnostics.

### Phase 7: Status Bar / Activity Area

Purpose:

- Give ambient app state a stable place to live.

Files:

- `src/status_bar.rs`
- `src/root.rs`
- `src/tasks.rs`
- `src/notifications/service.rs`

Show:

- current route
- active background task count
- notification backend/degraded state
- offline/error state placeholder
- last app error indicator

Acceptance criteria:

- Status bar updates when tasks start/finish.
- Notification degraded state is visible without opening Settings.
- It does not become a noisy second Settings page.

### Phase 8: Central Error Surface

Purpose:

- Replace random one-off toasts with structured recoverable errors.

Files:

- `src/errors.rs`
- `src/events.rs`
- `src/root.rs`
- `src/views/settings.rs`

Work:

- Add `AppError`.
- Add severity:
  - info
  - warning
  - error
- Add optional actions:
  - retry
  - open settings
  - copy details
  - dismiss
- Route notification/backend/task failures into this system.

Acceptance criteria:

- Errors can be shown, dismissed, and inspected.
- Errors can still use GPUI toasts for foreground hints, but the source of truth is structured.

### Phase 9: Diagnostics View

Purpose:

- Make the boilerplate debuggable by users and developers.

Files:

- `src/views/diagnostics.rs`
- `src/views/mod.rs`
- `src/sidebar.rs`
- `src/root.rs`
- `src/app.rs`

Show:

- app version
- build profile
- OS/platform
- app config path
- app data path
- active route
- notification backend
- notification permission state
- single-instance status
- deep-link scheme status
- background task count
- last errors

Actions:

- copy diagnostics
- open logs folder once file logging exists
- refresh diagnostics

Acceptance criteria:

- Diagnostics page can be reached from Settings and command palette.
- It provides enough detail to debug local notification/deep-link issues.

### Phase 10: First-Run / Setup State

Purpose:

- Give boilerplate apps a clean place for first-launch behavior.

Files:

- `src/first_run.rs`
- `src/app_state.rs`
- `src/root.rs`
- `src/views/settings.rs`

Work:

- Detect first launch.
- Show a small first-run panel or route.
- Let users choose:
  - theme
  - locale
  - native notifications enabled
- Mark first-run complete.

Acceptance criteria:

- First-run state is persisted.
- It does not show repeatedly.
- It can be reset from diagnostics/settings in development.

## Cross-Cutting Design Rules

- Prefer typed app routes over raw strings after parsing.
- Prefer typed events over direct cross-module mutation.
- Keep GPUI toasts as foreground hints, not state storage.
- Persist only user/application state, not debug-only transient values.
- Make platform-dependent features capability-driven.
- Every background operation should produce trace logs and structured status.
- Every feature should expose enough diagnostics to debug itself.

## Recommended First Implementation Batch

Implement these together:

1. `AppRoute`
2. app event bus
3. persistent app state in user config dir
4. route persistence
5. command palette updated to emit routes

Reason:

- Deep links, single-instance forwarding, notification inbox, diagnostics, and first-run all need a real route/event/state foundation.

## Recommended Second Implementation Batch

Implement:

1. single-instance lock
2. deep-link forwarding
3. focused-window restore on forwarded links

Reason:

- This is the natural follow-up after routes exist.

## Recommended Third Implementation Batch

Implement:

1. notification inbox
2. bell popover/page
3. unread state
4. notification persistence

Reason:

- Native local notifications already exist, but the app still needs internal notification history.

## Verification Strategy

For every batch:

- `cargo fmt`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- manual launch from raw binary
- manual launch from bundled macOS helper

Feature-specific checks:

- Deep links:
  - parse valid routes
  - reject invalid routes
  - route into existing UI
- Single instance:
  - second launch exits after forwarding
  - first instance focuses
  - tray/hotkey is not duplicated
- Persistent state:
  - config file created in user config dir
  - corrupted config falls back safely
  - active route restores
- Inbox:
  - records native notification attempts
  - unread count updates
  - clear/read state persists
- Background tasks:
  - progress updates render
  - cancellation and failure states render
  - errors route to central error surface

## Open Questions

- Should Diagnostics be a real sidebar page or a Settings subpage?
- Should notification inbox be a sidebar page, a bell popover, or both?
- Which crate should we use for single-instance IPC?
- Do we want the custom URL scheme to be `gpui-starter://` or a product-neutral placeholder?
- Should first-run be enabled in the boilerplate by default, or only as an optional example?
