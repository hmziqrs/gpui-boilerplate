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
- file logging
- runtime capabilities registry
- global shortcut settings
- connectivity state
- secure storage boundary
- accessibility checklist
- local app database boundary
- undoable app commands
- telemetry boundary

Out of scope:

- production packaging
- installers
- auto-update implementation
- document/file-editor workflows
- arbitrary file/folder picker demos unless a feature needs them
- remote push notifications

## Crate Research

Recommended candidates:

- `url`
  - Use for parsing custom deep-link URLs into structured data.
  - Prefer this over ad hoc string splitting once routes have query parameters.
- `directories`
  - Use `ProjectDirs` for config, data, cache, and log paths.
  - Prefer this over `dirs` for app-specific paths.
- `single-instance`
  - Simple process-level single-instance guard.
  - Good for lock detection, but it does not solve payload forwarding by itself.
- `interprocess`
  - Use local sockets for second-instance-to-first-instance forwarding.
  - Better fit for deep-link forwarding than lock-only crates.
- `tracing-appender`
  - Use rolling file appenders and non-blocking writers for persistent logs.
  - Keep the `WorkerGuard` alive for the whole app lifetime.
- `keyring`
  - Use as the secure storage boundary for tokens/API keys later.
  - Requires platform feature choices, especially for Linux secret service behavior.
- `global-hotkey`
  - Already in use for macOS global hotkey support.
  - Extend existing usage rather than replacing it.
- `reqwest`
  - Use only if we add a real network client or connectivity probe.
  - Do not add it only to ping a URL unless network features are coming.
- `network-interface`
  - Useful for diagnostics about local interfaces, not enough by itself for internet reachability.
- `accesskit`
  - Useful if GPUI does not already expose the accessibility tree we need.
  - First step should be auditing GPUI accessibility support before adding it directly.
- `rusqlite`
  - Use if we want a conventional local SQLite database for app data, inbox history, sync metadata, or cached records.
  - Best fit when data is relational, inspectable, and likely to need migrations.
- `redb`
  - Use if we want a pure-Rust embedded key-value store.
  - Better fit for simple local records and indexes when SQL is unnecessary.
- `undo` / `undoredo`
  - Optional helpers for undo/redo stacks.
  - Prefer app-local command types first so UI labels, persistence, permissions, and side effects stay explicit.
- `opentelemetry` / `tracing-opentelemetry`
  - Use only behind an explicit telemetry feature flag.
  - The boilerplate should define a telemetry boundary, not send events by default.

App-local features with no crate required:

- capabilities registry
- central error surface
- notification inbox model
- background task registry
- first-run state
- diagnostics view
- telemetry no-op sink
- undo command registry

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
- There is no local database boundary for apps that outgrow JSON state.
- There is no undo/redo command model for user-facing state changes.
- There is no telemetry boundary that makes analytics/diagnostics export explicitly opt-in.

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

### Phase 11: File Logging

Purpose:

- Make logs available after the app exits and expose them through Diagnostics.

Files:

- `Cargo.toml`
- `src/logging.rs`
- `src/app.rs`
- `src/views/diagnostics.rs`

Recommended dependencies:

- `tracing-appender`
- `directories`

Work:

- Create app log directory under the user data directory.
- Add rolling file appender.
- Keep terminal logging in development.
- Keep the `tracing_appender::non_blocking::WorkerGuard` alive in app-global state.
- Add diagnostics fields:
  - log directory
  - current log file prefix
  - logging enabled
- Add a future "Open logs folder" action.

Policy:

- Logs must not contain secrets.
- Use structured fields for subsystem/backend/error details.
- Keep file logging enabled by default in dev and configurable in release.

Acceptance criteria:

- App writes logs to a user data directory.
- Logs include notification/deep-link/single-instance events.
- Logs flush on normal app quit.
- Diagnostics can show the log path.

### Phase 12: Runtime Capabilities Registry

Purpose:

- Centralize platform/runtime capability reporting.

Files:

- `src/capabilities.rs`
- `src/app.rs`
- `src/views/settings.rs`
- `src/views/diagnostics.rs`
- `src/notifications/service.rs`
- `src/tray.rs`

Work:

- Add `CapabilityRegistry` global.
- Track capabilities:
  - native local notifications
  - notification permission management
  - notification actions/reply
  - tray icon
  - global hotkey
  - deep links
  - single-instance lock
  - second-instance forwarding
  - file logging
  - secure storage
  - accessibility status
- Each capability should have:
  - `supported`
  - `enabled`
  - `degraded`
  - `reason`
  - `last_error`

Acceptance criteria:

- Settings and Diagnostics read from one registry instead of duplicating backend state.
- Notification, tray, and global-hotkey initialization report capability status.
- Unsupported features are explicit, not inferred from missing UI.

### Phase 13: Global Shortcut Settings

Purpose:

- Make the existing global hotkey observable and configurable.

Files:

- `src/shortcuts.rs`
- `src/tray.rs`
- `src/views/settings.rs`
- `src/app_state.rs`
- `src/capabilities.rs`

Current dependency:

- `global-hotkey`

Work:

- Move hotkey registration out of `tray.rs` into a shortcut service.
- Persist enabled/disabled state.
- Show registration result in Settings and Diagnostics.
- Add conflict/error reporting.
- Keep `Alt+Space` as the default launcher shortcut.

Future work:

- User-configurable shortcut capture UI.
- Per-command shortcut registry.

Acceptance criteria:

- User can disable global shortcut.
- Failed registration is visible in Settings/Diagnostics.
- Tray setup no longer owns shortcut state.

### Phase 14: Connectivity State

Purpose:

- Provide a reusable state model for network-aware desktop apps.

Files:

- `src/connectivity.rs`
- `src/tasks.rs`
- `src/status_bar.rs`
- `src/views/diagnostics.rs`

Candidate dependencies:

- `reqwest` if the app adds real HTTP/network features.
- `network-interface` for diagnostics about local interfaces.

Recommended v1:

- Define the state model without adding a network crate yet.
- Add manual/demo transitions for `Unknown`, `Online`, and `Offline`.
- Wire the state to status bar and diagnostics.

Reason:

- A boilerplate should not make external network calls by default unless the app has a real network feature.

Acceptance criteria:

- Connectivity state is represented centrally.
- Status bar and diagnostics can show it.
- Future HTTP clients/background sync can update it.

### Phase 15: Secure Storage Boundary

Purpose:

- Give future auth/account features a safe place to store secrets without designing auth now.

Files:

- `Cargo.toml`
- `src/secure_storage.rs`
- `src/capabilities.rs`
- `src/views/diagnostics.rs`

Recommended dependency:

- `keyring`

Work:

- Define app-local trait:
  - `set_secret`
  - `get_secret`
  - `delete_secret`
  - `is_available`
- Implement a `keyring` backend behind feature/platform guards.
- Add a no-op unavailable backend for unsupported/dev environments.
- Report capability status in Diagnostics.

Policy:

- Never store secrets in JSON app state.
- Never log secret values.
- Secret keys should be namespaced by app id and environment.

Acceptance criteria:

- Secure storage availability is visible.
- Basic set/get/delete integration test can run behind an opt-in flag or mock backend.
- App state and logs do not contain secret material.

### Phase 16: Account / Session Placeholder

Purpose:

- Prepare for apps that need auth without implementing auth in the boilerplate.

Files:

- `src/session.rs`
- `src/events.rs`
- `src/status_bar.rs`
- `src/views/settings.rs`

Work:

- Add `SessionState`:
  - `SignedOut`
  - `SigningIn`
  - `SignedIn`
  - `Error`
- Add event hooks for deep-link auth callback later.
- Store non-secret account metadata only.

Acceptance criteria:

- Session state exists and can be displayed.
- No credentials are persisted outside secure storage.
- Deep-link plan has a place to route future auth callbacks.

### Phase 17: Accessibility Checklist

Purpose:

- Make accessibility a required review surface for every UI feature.

Files:

- `docs/accessibility-checklist.md`
- `src/views/diagnostics.rs`

Candidate dependency:

- `accesskit`, only after auditing GPUI's current accessibility support.

Work:

- Document keyboard navigation expectations.
- Audit icon-only buttons for labels/tooltips.
- Check focus order for:
  - sidebar
  - launcher
  - settings
  - notification inbox
  - diagnostics
- Add reduced-motion setting only if motion is introduced.
- Add diagnostics entry for accessibility support status.

Acceptance criteria:

- Every new interactive control has a keyboard path.
- Icon-only controls have accessible naming strategy or tooltip at minimum.
- Accessibility limitations are documented rather than hidden.

### Phase 18: Local App Database Boundary

Purpose:

- Give future app features a durable data layer without forcing every boilerplate app to use a database immediately.

Files:

- `Cargo.toml`
- `src/storage.rs`
- `src/app_state.rs`
- `src/views/diagnostics.rs`

Candidate dependencies:

- `rusqlite` for SQLite-backed relational local data.
- `redb` for pure-Rust embedded key-value data.

Recommended v1:

- Do not migrate current small app settings from JSON yet.
- Define a storage boundary and diagnostics first.
- Add a concrete database backend only when notification inbox history, task history, sync cache, or app records need it.

Work:

- Define app-local storage traits for:
  - schema/version reporting
  - migrations
  - health check
  - compact/vacuum or maintenance
- Keep app preferences in config JSON unless there is a clear query/migration need.
- If choosing SQLite:
  - use `rusqlite`
  - store database under the app data directory
  - include migration version in diagnostics
- If choosing embedded KV:
  - use `redb`
  - define typed table/key namespaces
  - avoid ad hoc serialized blobs without versioning

Acceptance criteria:

- Diagnostics can show whether a database backend exists, where it lives, and which schema version is active.
- App startup does not require a database unless a feature actually uses it.
- Corrupted or incompatible local data fails visibly through the central error surface.

### Phase 19: Undoable App Commands

Purpose:

- Give desktop-style user actions a reusable undo/redo model.

Files:

- `src/commands.rs`
- `src/events.rs`
- `src/launcher.rs`
- `src/status_bar.rs`
- `src/views/settings.rs`

Candidate dependencies:

- `undo`
- `undoredo`

Recommended v1:

- Start with app-local command structs and an in-memory undo stack.
- Add a crate only if the command stack becomes repetitive or needs snapshot/delta helpers.

Work:

- Define `AppCommand` metadata:
  - id
  - label
  - undo label
  - redo label
  - source route
  - timestamp
- Define command execution result:
  - applied
  - rejected with reason
  - requires confirmation
- Add command stack operations:
  - execute
  - undo
  - redo
  - clear scope
- Wire launcher actions and selected settings changes through the command system where it makes sense.

Policy:

- Do not put irreversible side effects into undo unless the inverse operation is reliable.
- File/network/notification side effects should create visible history, not fake undo.
- Command labels must be user-facing because menu items and launcher entries will reuse them.

Acceptance criteria:

- At least one low-risk setting change can be undone/redone.
- Undo/redo availability is visible in launcher or status area.
- Rejected commands route to the central error surface.

### Phase 20: Telemetry Boundary

Purpose:

- Make product analytics and remote diagnostics an explicit opt-in layer instead of accidental logging creep.

Files:

- `src/telemetry.rs`
- `src/app.rs`
- `src/app_state.rs`
- `src/views/settings.rs`
- `src/views/diagnostics.rs`

Candidate dependencies:

- `opentelemetry`
- `tracing-opentelemetry`

Recommended v1:

- Implement a no-op telemetry sink.
- Keep local tracing/file logs separate from telemetry export.
- Add remote telemetry crates only behind a feature flag and user-visible setting.

Work:

- Define `TelemetrySink`:
  - `record_event`
  - `record_error`
  - `set_user_properties`
  - `flush`
- Add telemetry consent state:
  - disabled
  - local-only diagnostics
  - remote opt-in
- Add diagnostics fields:
  - telemetry compiled
  - telemetry enabled
  - telemetry endpoint redacted
  - last export error

Policy:

- Default must be disabled.
- Never send notification contents, secrets, file paths, or raw deep-link payloads without product-specific review.
- Keep correlation IDs allowed, but avoid stable user identity until an app explicitly opts in.

Acceptance criteria:

- Boilerplate builds with telemetry disabled.
- Settings and Diagnostics clearly show telemetry state.
- Enabling any remote exporter requires a deliberate feature/config change.

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

## Recommended Fourth Implementation Batch

Implement:

1. file logging
2. capabilities registry
3. diagnostics page

Reason:

- These make every platform feature easier to debug and give the boilerplate a professional support surface.

## Recommended Fifth Implementation Batch

Implement:

1. global shortcut settings
2. connectivity state model
3. secure storage boundary
4. accessibility checklist

Reason:

- These are valuable desktop foundations, but they should build on the route/event/state/capability layers.

## Recommended Sixth Implementation Batch

Implement:

1. local app database boundary
2. undoable app command model
3. telemetry no-op boundary

Reason:

- These are important production-grade foundations, but they should not block deep links, diagnostics, notifications, or state persistence. They also need clear product decisions before concrete backends are enabled.

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
- File logging:
  - log directory is created
  - logs include startup and backend diagnostics
  - log guard flushes on exit
- Capabilities:
  - capabilities page/status matches actual runtime
  - degraded features include reasons
- Secure storage:
  - unavailable backend is handled without panic
  - no secret values appear in logs or app state
- Local database:
  - database backend is optional at startup
  - schema/version diagnostics are visible when enabled
  - migration failure routes to the central error surface
- Undo/redo:
  - undoable commands expose labels
  - irreversible operations are not added to the undo stack
  - rejected commands surface structured errors
- Telemetry:
  - disabled by default
  - remote exporter code is feature-gated
  - diagnostics distinguish local logs from telemetry export

## Open Questions

- Should Diagnostics be a real sidebar page or a Settings subpage?
- Should notification inbox be a sidebar page, a bell popover, or both?
- Which crate should we use for single-instance IPC?
- Do we want the custom URL scheme to be `gpui-starter://` or a product-neutral placeholder?
- Should first-run be enabled in the boilerplate by default, or only as an optional example?
- Should file logging be enabled in release by default?
- Should secure storage be compiled by default or behind a feature flag?
- Should connectivity ever make a real network request in boilerplate mode?
- Does GPUI already provide enough accessibility support, or do we need direct AccessKit integration?
- Should the first database backend be SQLite via `rusqlite`, embedded KV via `redb`, or no backend until a feature requires it?
- Should undo/redo be limited to settings/demo state, or should it become a general app command system immediately?
- Should telemetry remain a no-op boundary forever in the boilerplate, with real exporters only in downstream apps?

## References

- `url`: <https://docs.rs/url/latest/>
- `directories` / `dirs` family: <https://docs.rs/crate/dirs/latest/source/README.md>
- `single-instance`: <https://docs.rs/single-instance/latest/single_instance/struct.SingleInstance.html>
- `interprocess`: <https://docs.rs/interprocess/latest/interprocess/>
- `tracing-appender`: <https://docs.rs/tracing-appender/latest/tracing_appender/>
- `keyring`: <https://docs.rs/keyring/latest/keyring/>
- `global-hotkey`: <https://docs.rs/global-hotkey/latest/global_hotkey/>
- `reqwest`: <https://docs.rs/reqwest/latest/reqwest/>
- `network-interface`: <https://docs.rs/network-interface>
- `accesskit`: <https://docs.rs/accesskit/latest/accesskit/>
- `rusqlite`: <https://docs.rs/rusqlite/latest/>
- `redb`: <https://docs.rs/redb/latest/redb/>
- `undo`: <https://docs.rs/undo/latest/undo/>
- `undoredo`: <https://docs.rs/undoredo/latest/undoredo/>
- `opentelemetry`: <https://docs.rs/opentelemetry/latest/opentelemetry/>
- `tracing-opentelemetry`: <https://docs.rs/tracing-opentelemetry/latest/tracing_opentelemetry/>
