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
- native desktop utility actions
- accessibility checklist
- local app database boundary
- undoable app commands
- telemetry boundary

Out of scope:

- production packaging
- installers
- auto-update implementation
- document/file-editor workflows
- remote push notifications

## Crate Research

Chosen crates:

- `serde`
  - Use for versioned app state, persisted inbox items, command metadata, diagnostics snapshots, and capability exports.
  - Already a standard dependency class for Rust desktop state boundaries.
- `serde_json`
  - Use for human-inspectable app config/state files.
  - Prefer this for v1 settings before adding a database.
- `thiserror`
  - Use for typed app/domain errors.
  - Prefer this over unstructured string errors for the central error surface.
- `uuid`
  - Use for notification IDs, task IDs, command IDs, event correlation IDs, and diagnostics correlation.
  - Enable generation features only where needed.
- `chrono`
  - Use for user-facing timestamps in inbox, task history, diagnostics, and logs exports.
  - Keep serialized formats explicit.
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
  - Use for the concrete connectivity probe and future HTTP-backed desktop features.
  - Keep the probe endpoint configurable and visible in Diagnostics.
- `network-interface`
  - Use for local interface diagnostics alongside the `reqwest` reachability probe.
- `accesskit`
  - Use for the accessibility implementation/audit path.
  - GPUI support should still be audited first, but AccessKit is the selected crate if direct integration is required.
- `open`
  - Use for opening URLs, log folders, config folders, and support links in the system default app.
  - Prefer this over shelling out to `open`, `xdg-open`, or `start`.
- `rfd`
  - Use for cross-platform native file/folder pickers and simple native message dialogs.
  - Keep picker demos small; the reusable part is the platform service wrapper.
- `arboard`
  - Use for clipboard text/image/file-path operations.
  - Useful for "copy diagnostics", "copy logs path", and later app-level clipboard actions.
- `notify`
  - Use for file-system watching of config/log/import folders.
  - Do not use it for static file pickers.
- `rusqlite`
  - Selected local database backend.
  - Use SQLite for app data, inbox history, sync metadata, cached records, and inspectable migrations.
- `redb`
  - Not selected for this boilerplate.
  - Keep it out unless a downstream product explicitly wants embedded key-value storage instead of SQLite.
- `undo`
  - Selected helper crate for undo/redo stack mechanics.
  - Keep app-local command types for labels, persistence, permissions, and side effects.
- `undoredo`
  - Not selected for this boilerplate.
  - Snapshot/delta undo is less explicit than command-based undo for desktop application actions.
- `opentelemetry` / `tracing-opentelemetry`
  - Selected telemetry stack.
  - Remote export remains controlled by runtime consent/settings, but the integration path is not deferred.

App-local features with no crate required:

- capabilities registry
- central error surface
- notification inbox model
- background task registry
- first-run state
- diagnostics view
- telemetry disabled/local/remote sinks
- undo command registry
- native utility service wrappers

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
- Common desktop utilities like copy diagnostics, open logs folder, and native file/folder pickers do not have a shared service boundary.
- There is no local database boundary for apps that outgrow JSON state.
- There is no undo/redo command model for user-facing state changes.
- There is no telemetry boundary that makes analytics/diagnostics export explicitly opt-in.

## Dependency-Ordered Roadmap

This is the order to implement the plan in practice. It is stricter than the feature list because later features should not invent their own state, errors, logging, or capability reporting.

### Layer 0: Shared Primitives

Implement first:

1. `src/ids.rs`
2. `src/time.rs`
3. `src/errors.rs`
4. `src/paths.rs`
5. common serialization helpers

Crates:

- `serde`
- `serde_json`
- `thiserror`
- `uuid`
- `chrono`
- `directories`

Reason:

- Routes, events, notifications, tasks, diagnostics, and persistence all need stable IDs, timestamps, typed errors, and OS-correct paths.

### Layer 1: Routing, Events, And State

Implement next:

1. Phase 1: app routes and deep links
2. Phase 2: central app event bus
3. Phase 3: persistent app state in user config directory
4. Phase 10: first-run/setup state

Reason:

- Everything user-visible should move through typed routes/events and survive relaunch before platform features are layered on.

### Layer 2: Observability And Support Surface

Implement before advanced features:

1. Phase 11: file logging
2. Phase 12: runtime capabilities registry
3. Phase 8: central error surface
4. Phase 9: diagnostics view
5. Phase 18: native desktop utility actions

Reason:

- Deep links, notifications, shortcuts, secure storage, and IPC are platform-sensitive. They need logs, capability status, diagnostics, and common utility actions before debugging becomes expensive.

### Layer 3: External Entry Points

Implement after observability:

1. Phase 4: single-instance and deep-link forwarding
2. Phase 13: global shortcut settings

Reason:

- These features affect process lifetime and global OS state. They should be capability-gated and diagnosable before they are enabled broadly.

### Layer 4: User-Facing App Shell

Implement after routes/events/state exist:

1. Phase 5: internal notification inbox
2. Phase 6: background task manager
3. Phase 7: status bar/activity area
4. Phase 20: undoable app commands

Reason:

- These features are core desktop UX, but they depend on the event bus, persistence, errors, and diagnostics.

### Layer 5: Platform And Product Boundaries

Implement once the app shell is stable:

1. Phase 14: connectivity state
2. Phase 15: secure storage boundary
3. Phase 16: account/session placeholder
4. Phase 17: accessibility checklist
5. Phase 19: local app database boundary
6. Phase 21: telemetry boundary

Reason:

- These features are important and the crate choices are now fixed; they come later only because they depend on the shell and observability layers.

## Concrete Decisions

- Deep-link parsing: use `url`.
- App paths: use `directories`.
- Single instance: use `single-instance` for the process guard and `interprocess` for forwarding payloads.
- Persistent logs: use `tracing-appender`.
- Secure storage: use `keyring`.
- Global shortcuts: continue with `global-hotkey`.
- Connectivity: use `reqwest` for reachability checks and `network-interface` for local interface diagnostics.
- Accessibility: use `accesskit` as the selected accessibility integration crate after the GPUI audit identifies the exact bridge point.
- Desktop actions: use `open`, `rfd`, `arboard`, and `notify`.
- Local database: use `rusqlite`; do not use `redb` in this boilerplate.
- Undo/redo: use `undo`; do not use `undoredo` in this boilerplate.
- Telemetry: use `opentelemetry` and `tracing-opentelemetry`; remote export is runtime-consent controlled, not feature-deferred.
- Serialization/errors/IDs/time: use `serde`, `serde_json`, `thiserror`, `uuid`, and `chrono`.

## Detailed Implementation Phases

### Phase 0: Shared Primitives

Purpose:

- Create the common types every later feature should reuse instead of inventing local versions.

Files:

- `Cargo.toml`
- `src/ids.rs`
- `src/time.rs`
- `src/errors.rs`
- `src/paths.rs`

Recommended dependencies:

- `serde`
- `serde_json`
- `thiserror`
- `uuid`
- `chrono`
- `directories`

Work:

- Add typed IDs for notifications, tasks, commands, and events.
- Add timestamp helpers for persisted state and UI display.
- Add typed app error categories.
- Add app path helpers for config, data, cache, logs, and runtime IPC.
- Add serialization helpers for versioned state structs.

Acceptance criteria:

- Later modules do not use raw `String` IDs where typed IDs exist.
- App paths never point at `target/` except test/dev fixtures.
- Common errors can be logged, displayed, and serialized for diagnostics.

### Phase 1: App Routes And Deep Links

Purpose:

- Make external inputs and internal navigation use the same route model.

Files:

- `src/routes.rs`
- `src/sidebar.rs`
- `src/root.rs`
- `src/launcher.rs`
- `src/app.rs`

Recommended dependency:

- `url`

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

Recommended dependencies:

- `uuid`
- `chrono`
- `serde`

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
- `serde`
- `serde_json`

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

- `single-instance` for the process guard.
- `interprocess` for second-instance payload forwarding.

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

Recommended dependencies:

- `serde`
- `uuid`
- `chrono`
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

Recommended dependencies:

- `uuid`
- `chrono`
- `serde`

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

Recommended dependency:

- `thiserror`

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

Recommended dependencies:

- `arboard`
- `open`

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

Selected dependencies:

- `reqwest`
- `network-interface`

Implementation:

- Define the state model for `Unknown`, `Online`, `Offline`, and `CaptiveOrFiltered`.
- Add a `reqwest` reachability probe with a configurable endpoint.
- Add `network-interface` diagnostics for local interface names, addresses, and link-adjacent state where available.
- Wire the state to status bar and diagnostics.

Policy:

- The default probe endpoint must be explicit in Diagnostics.
- Probe failures should degrade connectivity state, not create noisy user errors.
- Do not send app/user identity with connectivity probes.

Acceptance criteria:

- Connectivity state is represented centrally.
- Status bar and diagnostics can show it.
- A manual "Check connectivity now" action runs the `reqwest` probe.
- Local interface diagnostics are visible.

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
- Implement a `keyring` backend.
- Add an unavailable backend only for platforms where `keyring` cannot initialize at runtime.
- Report capability status in Diagnostics.

Policy:

- Never store secrets in JSON app state.
- Never log secret values.
- Secret keys should be namespaced by app id and environment.

Acceptance criteria:

- Secure storage availability is visible.
- Basic set/get/delete tests cover the mock backend and a manually run native backend check.
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

Selected dependency:

- `accesskit`

Work:

- Document keyboard navigation expectations.
- Audit GPUI's current accessibility bridge points.
- Add AccessKit integration where GPUI does not expose enough accessibility metadata.
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

### Phase 18: Native Desktop Utility Actions

Purpose:

- Provide cross-platform wrappers for desktop actions that almost every app eventually needs.

Files:

- `Cargo.toml`
- `src/desktop_actions.rs`
- `src/views/diagnostics.rs`
- `src/views/settings.rs`
- `src/capabilities.rs`

Recommended dependencies:

- `open`
- `arboard`
- `rfd`
- `notify`

Work:

- Add service functions:
  - `open_url`
  - `open_path`
  - `open_logs_folder`
  - `copy_text`
  - `copy_diagnostics`
  - `pick_file`
  - `pick_folder`
  - `save_file`
  - `watch_path`
  - `unwatch_path`
- Report per-action availability in capabilities/diagnostics.
- Route failures into the central error surface.
- Keep dialogs behind user actions, never startup.

Policy:

- Do not make arbitrary file/folder demos the feature; make reusable desktop service wrappers the feature.
- File picker results should be treated as user-granted paths, not permanent broad file-system access.
- Clipboard actions should never copy secrets unless the user explicitly requested it.

Acceptance criteria:

- Diagnostics can copy text through the shared clipboard service.
- Logs/config folders can be opened through the shared opener service.
- File/folder picker wrappers exist behind a small manual test surface.
- Config/log/import path watchers can be registered and stopped cleanly.
- Unsupported platform behavior is visible through capabilities.

### Phase 19: Local App Database Boundary

Purpose:

- Give future app features a durable data layer without forcing every boilerplate app to use a database immediately.

Files:

- `Cargo.toml`
- `src/storage.rs`
- `src/app_state.rs`
- `src/views/diagnostics.rs`

Selected dependency:

- `rusqlite`

Implementation:

- Add a SQLite database under the app data directory.
- Keep simple user preferences in JSON config.
- Store queryable app records in SQLite:
  - notification inbox history
  - background task history
  - diagnostics snapshots
  - future sync/cache records

Work:

- Define app-local storage traits for:
  - schema/version reporting
  - migrations
  - health check
  - compact/vacuum or maintenance
- Keep app preferences in config JSON unless there is a clear query/migration need.
- Use `rusqlite`.
- Store database under the app data directory.
- Include migration version in diagnostics.
- Add a small migration runner with explicit schema versions.
- Add a health check query.

Acceptance criteria:

- Diagnostics shows the SQLite database path and active schema version.
- App startup initializes or migrates the database.
- Corrupted or incompatible local data fails visibly through the central error surface.

### Phase 20: Undoable App Commands

Purpose:

- Give desktop-style user actions a reusable undo/redo model.

Files:

- `src/commands.rs`
- `src/events.rs`
- `src/launcher.rs`
- `src/status_bar.rs`
- `src/views/settings.rs`

Selected dependency:

- `undo`

Implementation:

- Use `undo` for undo/redo stack mechanics.
- Keep app-local command structs for action labels, side effects, persistence, and permissions.

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

### Phase 21: Telemetry Boundary

Purpose:

- Make product analytics and remote diagnostics an explicit opt-in layer instead of accidental logging creep.

Files:

- `src/telemetry.rs`
- `src/app.rs`
- `src/app_state.rs`
- `src/views/settings.rs`
- `src/views/diagnostics.rs`

Selected dependencies:

- `opentelemetry`
- `tracing-opentelemetry`

Implementation:

- Implement telemetry sinks:
  - disabled sink
  - local diagnostics sink
  - OpenTelemetry tracing sink
- Keep local tracing/file logs separate from telemetry export.
- Compile the telemetry stack into the boilerplate.
- Remote export is controlled by runtime consent and settings.

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

- Default remote export must be disabled.
- Never send notification contents, secrets, file paths, or raw deep-link payloads without product-specific review.
- Keep correlation IDs allowed, but avoid stable user identity until an app explicitly opts in.

Acceptance criteria:

- Boilerplate builds with telemetry crates present.
- Settings and Diagnostics clearly show telemetry state.
- Enabling remote export requires a deliberate runtime setting change.

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

1. shared primitives
2. `AppRoute`
3. app event bus
4. persistent app state in user config dir
5. route persistence
6. first-run state
7. command palette updated to emit routes

Reason:

- Every later feature needs stable IDs, typed errors, app paths, serialization, routes, events, and persisted state.

## Recommended Second Implementation Batch

Implement:

1. file logging
2. capabilities registry
3. central error surface
4. diagnostics page
5. native desktop utility actions

Reason:

- Platform-heavy work should not start until logs, capability status, diagnostics, and common desktop actions are available.

## Recommended Third Implementation Batch

Implement:

1. single-instance lock
2. second-instance IPC forwarding
3. focused-window restore on forwarded links
4. global shortcut settings

Reason:

- External entry points affect process lifetime and OS-global state, so they come after observability.

## Recommended Fourth Implementation Batch

Implement:

1. notification inbox
2. bell popover/page
3. unread state
4. notification persistence
5. background task manager
6. status bar/activity area
7. undoable app command model

Reason:

- These are the core desktop app-shell features and should all share routes, events, state, errors, diagnostics, and IDs.

## Recommended Fifth Implementation Batch

Implement:

1. connectivity state model
2. secure storage boundary
3. account/session placeholder
4. accessibility checklist

Reason:

- These are cross-cutting product/platform boundaries that should be wired after the shell is stable.

## Recommended Sixth Implementation Batch

Implement:

1. local app database boundary
2. telemetry boundary

Reason:

- These are heavier foundations, but the crate choices are settled: SQLite via `rusqlite` and telemetry via `opentelemetry` / `tracing-opentelemetry`.

## Original Feature Groups

These are retained only as a checklist. The dependency-ordered batches above should drive implementation.

Group 1:

1. routes
2. event bus
3. persistent app state
4. first-run state

Group 2:

1. logs
2. capabilities
3. errors
4. diagnostics
5. desktop utility actions

Group 3:

1. single instance
2. deep-link forwarding
3. global shortcuts

Group 4:

1. notification inbox
2. background tasks
3. status/activity area
4. undo/redo

Group 5:

1. connectivity
2. secure storage
3. account/session
4. accessibility
5. local database
6. telemetry

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
- Desktop utility actions:
  - copy diagnostics uses the shared clipboard wrapper
  - open logs/config folder uses the shared opener wrapper
  - picker cancellation is handled without error noise
  - file watchers can be registered and stopped cleanly
- Connectivity:
  - manual reachability probe updates state
  - local interface diagnostics render
  - probe endpoint is visible in Diagnostics
- Secure storage:
  - unavailable backend is handled without panic
  - no secret values appear in logs or app state
- Accessibility:
  - keyboard path exists for new controls
  - AccessKit integration status is visible in Diagnostics
- Local database:
  - SQLite database initializes at startup
  - schema/version diagnostics are visible
  - migration failure routes to the central error surface
- Undo/redo:
  - undoable commands expose labels
  - irreversible operations are not added to the undo stack
  - rejected commands surface structured errors
- Telemetry:
  - crates are compiled in
  - remote export is disabled by default
  - runtime consent controls exporter activation
  - diagnostics distinguish local logs from telemetry export

## Final Product Decisions

- Diagnostics location: real sidebar page, also linked from Settings.
- Notification inbox: bell popover for quick access plus a full Notifications page.
- Single-instance IPC: `single-instance` for lock, `interprocess` for payload forwarding.
- URL scheme: keep `gpui-starter://` until the boilerplate is renamed.
- First-run: enabled by default.
- File logging: enabled in dev and release, with user-visible log path and retention policy.
- Secure storage: compiled by default with `keyring`.
- Connectivity: real manual `reqwest` probe plus `network-interface` diagnostics.
- Accessibility: use GPUI metadata where available and `accesskit` where the bridge is missing.
- File/folder pickers: exposed as shared desktop actions with a small Settings/Diagnostics manual test surface.
- Clipboard: support text first for Diagnostics and paths where the platform backend supports it.
- Local database: SQLite via `rusqlite`.
- Undo/redo: general app command system using `undo`.
- Telemetry: compile `opentelemetry` / `tracing-opentelemetry`; remote export disabled until user/runtime consent.

## References

- `serde`: <https://docs.rs/serde/latest/serde/>
- `serde_json`: <https://docs.rs/serde_json/latest/serde_json/>
- `thiserror`: <https://docs.rs/thiserror/latest/thiserror/>
- `uuid`: <https://docs.rs/uuid/latest/uuid/>
- `chrono`: <https://docs.rs/chrono/latest/chrono/>
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
- `open`: <https://docs.rs/open/latest/open/>
- `rfd`: <https://docs.rs/rfd/latest/rfd/>
- `arboard`: <https://docs.rs/arboard/latest/arboard/>
- `notify`: <https://docs.rs/notify/latest/notify/>
- `rusqlite`: <https://docs.rs/rusqlite/latest/>
- `redb`: <https://docs.rs/redb/latest/redb/>
- `undo`: <https://docs.rs/undo/latest/undo/>
- `undoredo`: <https://docs.rs/undoredo/latest/undoredo/>
- `opentelemetry`: <https://docs.rs/opentelemetry/latest/opentelemetry/>
- `tracing-opentelemetry`: <https://docs.rs/tracing-opentelemetry/latest/tracing_opentelemetry/>
