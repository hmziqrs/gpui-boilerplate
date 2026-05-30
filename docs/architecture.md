# Architecture

GPUI Starter is a desktop application boilerplate built with the [GPUI](https://github.com/zed-industries/zed) framework from Zed, targeting macOS (with Linux support). It ships working implementations of the cross-cutting concerns every desktop app needs — persisted settings, i18n, theming, background tasks, telemetry, notifications, diagnostics, and an in-app command palette — so you can start building your own features on day one.

## Module Map

Every module lives under `src/`. Modules that depend on other modules are noted with arrows.

| Module | Description | Depends on |
|---|---|---|
| `main` | Entry point. Runs single-instance preflight, then delegates to `app::init`. | `single_instance`, `app`, `events`, `tray` |
| `app` | Top-level initialization, window creation, locale/theme helpers, action registrations. | Almost everything (the orchestrator). |
| `app_state` | Global persisted configuration (`AppConfig`) and file-backed load/save cycle. | `paths`, `errors`, `routes`, `notifications::inbox` |
| `events` | Application-wide event bus (`AppEventQueue`) with emit/drain semantics. | `ids`, `time`, `errors`, `routes` |
| `commands` | Command palette registry — defines `CommandId`, availability checks, and dispatch. | `events`, `routes`, `sidebar`, `desktop_actions`, `undo_stack`, `connectivity` |
| `root` | UI root (`AppRoot`). Owns sidebar, title bar, page views, and routing. | `events`, `routes`, `sidebar`, `views::*`, `title_bar`, `status_bar`, `tasks`, `notifications`, `connectivity`, `session` |
| `storage` | SQLite data layer via `rusqlite`. Schema migrations, health checks, maintenance. | `app_state` (for paths), `capabilities` |
| `telemetry` | Observability with pluggable sinks (disabled / local-only / remote). | `capabilities` |
| `routes` | URL-based routing (`AppRoute`) with deep-link parsing (`gpui-starter://`). | `sidebar::Page`, `errors` |
| `sidebar` | Page enum: titles, icons, ordering. The source of truth for navigation pages. | — |
| `views` | Re-exports individual page view modules (`home`, `form_page`, `settings`, etc.). | Each view depends on relevant subsystems. |
| `capabilities` | Feature-flag registry (`CapabilityRegistry`). Tracks which subsystems are supported/enabled/degraded. | — |
| `lifecycle` | Startup/shutdown stage tracking, panic hook installation. | `time` |
| `notifications` | Native + in-app notification system with inbox persistence. | `events`, `capabilities`, `app_state` |
| `tasks` | Background task registry with drain-on-shutdown. | `ids`, `time`, `capabilities` |
| `connectivity` | Network connectivity probing. | `capabilities` |
| `session` | Session tracking (start time, uptime). | `capabilities`, `time` |
| `i18n` | Internationalization via `es-fluent` and `rust_i18n`. | — |
| `logging` | Structured logging initialization and shutdown. | `paths` |
| `paths` | Platform-aware directories for config, data, logs. | — |
| `errors` | `AppError` enum and `AppErrorSeverity` classification. | — |
| `error_surface` | In-app error display with dismiss/retry actions. | `errors`, `capabilities` |
| `ids` | `EventId`, `TaskId` — unique identifiers. | — |
| `time` | `AppTimestamp` helper for RFC 3339 timestamps. | — |
| `undo_stack` | Undo/redo for reversible operations (e.g., theme switches). | `capabilities` |
| `shortcuts` | Global keyboard shortcut registration. | `app_state`, `capabilities` |
| `desktop_actions` | System integrations: clipboard, file opener, diagnostics copy. | `capabilities`, `app_state` |
| `accessibility` | Accessibility helpers. | `capabilities` |
| `secure_storage` | Keychain/credential storage abstraction. | `capabilities` |
| `first_run` | First-run experience detection. | `app_state`, `capabilities` |
| `launcher` | Command palette / search launcher overlay. | `events`, `commands`, `capabilities` |
| `menus` | Application menu bar construction. | `commands`, `app` actions |
| `app_menu` | macOS application menu setup. | `capabilities` |
| `title_bar` | Custom title bar view. | `app` actions |
| `status_bar` | Bottom status bar showing route, connectivity, session info. | `routes`, `connectivity`, `session`, `capabilities` |
| `single_instance` | Prevents multiple app instances from running simultaneously. | `ids` |
| `tray` | System tray icon (macOS only). | — |
| `config_migrations` | Migrates `AppConfig` between schema versions. | — |
| `testing` | Test-only utilities (gated behind `#[cfg(test)]`). | — |

## Initialization Order

`app::init` runs inside the GPUI `App` callback. The sequence matters because later steps depend on earlier ones.

```
1.  lifecycle::install_panic_hook()
2.  gpui_component::init(cx)                           -- must precede all component use
3.  app_state::initialize(cx)                          -- loads persisted config from disk
4.  logging::initialize(cx)                            -- sets up tracing subscriber
5.  capabilities::initialize(cx)                       -- empty registry, populated below
6.  i18n::init_i18n(...)                               -- es-fluent language selection
7.  set_locale(&persisted.locale, cx)                  -- restores saved language
8.  ThemeRegistry::watch_dir(themes/, ...)             -- hot-reloadable theme files
9.  Theme scrollbar_show restore                       -- from persisted config
10. cx.observe_global::<Theme>(...)                    -- auto-persist theme changes
11. Action handlers: SwitchTheme, SwitchThemeMode, SelectLocale
12. launcher::init(cx)
13. events::AppEventQueue installed as global
14. tasks::initialize(cx)
15. error_surface::initialize(cx)
16. undo_stack::initialize(cx)
17. shortcuts::initialize(cx)
18. connectivity::initialize(cx)
19. desktop_actions::initialize(cx)
20. accessibility::initialize(cx)
21. secure_storage::initialize(cx)
22. session::initialize(cx)
23. storage::initialize(cx)                            -- SQLite schema + health check
24. telemetry::initialize(cx)
25. notifications::inbox::initialize(cx)
26. notifications::initialize(cx)
27. Key bindings registered (cmd-k, /, cmd-q/alt-f4)
28. Action handlers: Quit, About, OpenDiagnostics, ExecuteCommand
29. Lifecycle stage set to Running
```

After `init` returns, `main` calls `create_new_window` which opens the first `AppRoot` window.

### Shutdown Order

Triggered by the `Quit` action. Runs inside a `cx.spawn` to allow async drain:

```
1. tasks::drain_with_timeout(5s)     -- wait for background tasks
2. single_instance::shutdown(cx)
3. desktop_actions::shutdown(cx)
4. shortcuts::shutdown(cx)
5. storage::shutdown(cx)
6. telemetry::shutdown(cx)           -- flush + OpenTelemetry provider shutdown
7. logging::shutdown(cx)
8. cx.quit()
```

## State Management

### AppConfig (persisted)

Defined in `app_state.rs`. Serialized as JSON to `{data_dir}/state.json` using atomic writes. Fields include:

- `theme` — active theme name
- `locale` — `"en"` or `"zh-CN"`
- `active_route` — last navigated `AppRoute`
- `sidebar_collapsed` — sidebar collapse state
- `native_notifications_enabled` — OS notification toggle
- `global_shortcut_enabled` — global hotkey toggle
- `first_run_completed` — first-run gate
- `notification_inbox` — persisted notification items
- `window_bounds` — saved window position and size
- `scrollbar_show` — scrollbar visibility preference

Every mutation goes through `app_state::update_config(cx, |config| { ... })`, which normalizes the config, writes to disk, and replaces the global. If the write fails, `last_save_error` is set but the in-memory state still updates — the app never blocks on I/O.

On load, corrupt JSON is quarantined (renamed to `.json.bad`) and defaults are used instead.

### AppEventQueue (transient)

Defined in `events.rs`. A `Global` containing `Vec<AppEvent>`. Producers call `events::emit(kind, cx)`. The root view consumes with `events::drain(cx)`, which takes all events and resets the queue. This is a manual observer pattern — setting the global triggers GPUI's `observe_global` machinery.

### Entity pattern

GPUI uses `Entity<T>` (analogous to `Rc<RefCell<T>>` with reactive notifications). Views are always `Entity<SomeView>`, created via `cx.new(|cx| SomeView::new(cx))`. When a view's internal state changes, calling `cx.notify()` schedules a re-render of just that entity.

Global state uses GPUI's `Global` trait instead of entities. The project adopts a convention:

- **Singleton state** (config, telemetry snapshot, storage snapshot, lifecycle state): `impl Global for T`, set/read via `cx.set_global(...)` / `cx.global::<T>()`.
- **Views** (pages, title bar, root): `Entity<T>` where `T: Render`, observed via `cx.observe(...)` or `cx.observe_global::<SomeGlobal>(...)`.

## View System

### How views work

Each page is a struct implementing `gpui::Render`:

```rust
pub struct HomePage { /* fields */ }

impl Render for HomePage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Return an element tree
    }
}
```

`AppRoot` owns an `Entity<T>` for every page. Pages are constructed once during `AppRoot::new` and kept alive for the app's lifetime.

### Routing

`AppRoute` (in `routes.rs`) is the routing type. It has two variants:

- `Page(Page)` — maps directly to a sidebar page.
- `SettingsNotifications` — a sub-route that still renders the Settings page but with the notifications tab active.

`AppRoot::set_route` updates `active_route`, persists it to `AppConfig`, and calls `cx.notify()` to trigger a re-render. The `render` method calls `self.active_page_view()` which matches the current page to the correct entity.

### Sidebar navigation

The sidebar iterates `Page::all()` and renders a `SidebarMenuItem` for each. Clicking an item calls `this.set_route(AppRoute::page(*page), cx)`. The active page is highlighted via `.active(active_page == *page)`.

### Deep links

Routes double as URL schemes (`gpui-starter://settings`). `AppRoute::parse_deep_link` converts a URL string into an `AppRoute`. Deep links arrive as `AppEventKind::DeepLinkReceived(link)` events, which `AppRoot` parses and navigates to.

## Data Flow

The application follows a unidirectional data flow:

```
User action / System event
        |
        v
  events::emit(AppEventKind, cx)        -- or direct action handler
        |
        v
  AppEventQueue (Global)                -- transient buffer
        |
        v
  AppRoot::observe_global::<AppEventQueue>
        |
        v
  events::drain(cx)                     -- consume all pending events
        |
        v
  Match on AppEventKind:
    Navigate(route)    --> set_route --> update_config --> cx.notify()
    DeepLinkReceived   --> parse_deep_link --> set_route --> cx.notify()
    AppError           --> error_surface::report --> cx.notify()
    BackgroundTaskChanged / DiagnosticsChanged --> cx.notify()
        |
        v
  GPUI re-renders affected Entity<...> views
```

For actions (keybindings, menu items, command palette):

```
Action dispatched
    |
    v
cx.on_action handler
    |
    v
commands::execute(CommandId, cx)       -- or direct subsystem call
    |
    v
Subsystem updates global state
    |
    v
cx.set_global(updated_snapshot)
    |
    v
observe_global callbacks fire
    |
    v
cx.notify() on affected entities
    |
    v
Re-render
```

## Adding Features

### Add a new page

1. Create `src/views/my_page.rs` with a struct implementing `Render`.
2. Add `mod my_page;` and `pub use my_page::MyPage;` to `src/views/mod.rs`.
3. Add a variant to `Page` in `src/sidebar.rs` with `title()` and `icon()` entries.
4. Add the corresponding URL mapping in `AppRoute::to_url` and `AppRoute::parse_deep_link` (in `src/routes.rs`).
5. Add the entity field to `AppRoot` in `src/root.rs`, construct it in `AppRoot::new`, and add the match arm in `active_page_view`.
6. Add a `CommandId::OpenMyPage` variant in `src/commands.rs` with registry entry and execute dispatch.

### Add a new command

1. Add a variant to `CommandId` in `src/commands.rs`.
2. Add a `command(...)` entry to the `registry()` function with title, subtitle, and icon.
3. Implement `availability()` logic for the new variant (return `enabled: true` if always available).
4. Add a match arm in `execute()` that performs the action.
5. If the command should be undoable, integrate with `undo_stack`.

### Add a new theme

1. Create a `.json` theme file in `themes/` at the project root.
2. The file is auto-detected by `ThemeRegistry::watch_dir` during init.
3. No code changes required. The theme appears in settings automatically.

### Add a new locale

1. Add translation files under the `rust_i18n` resource path (e.g., `locales/`).
2. Add the locale code constant to `app.rs` (alongside `LOCALE_EN`, `LOCALE_ZH_CN`).
3. Update `AppConfig::normalized()` to validate against the new locale.
4. Add an `es-fluent` language variant in `src/app.rs` (`Languages` enum) and corresponding FTL files.

## Key Patterns

### Entity\<Model\> + Render

All UI components are `Entity<T>` where `T: Render`. Construction uses `cx.new(|cx| T::new(cx))`. State mutations call `cx.notify()` to schedule a re-render. The framework calls `render()` on the next frame, producing an element tree via the builder API (`div().child(...).flex()`).

### cx.spawn for async work

Long-running or I/O-bound work uses `cx.spawn(async move |cx| { ... })`. The closure receives an async-capable context. Call `cx.update(|cx| { ... })` inside the future to re-enter the synchronous GPUI context and update state. The returned `Task` is detached with `.detach()` or awaited.

```rust
cx.spawn(async move |cx| {
    let result = some_async_work().await;
    cx.update(|cx| {
        // synchronous state update
        cx.notify();
    })?;
    Ok::<_, anyhow::Error>(())
}).detach();
```

### Global state via entities and Global trait

Two patterns coexist:

1. **`impl Global for T`** — for singleton snapshots (config, telemetry, connectivity, lifecycle). Set via `cx.set_global(value)`, read via `cx.global::<T>()` or `cx.try_global::<T>()`. Changes notify observers via `cx.observe_global::<T>(callback).detach()`.

2. **`Entity<T>`** — for views and models with identity. Created with `cx.new(|cx| T::new(cx))`. Observed with `cx.observe(&entity, callback).detach()`. Rendered by returning `entity.clone().into()` as `AnyView`.

### Capability tracking

Every subsystem registers its status in `CapabilityRegistry` on init. The status includes `supported`, `enabled`, `degraded`, `reason`, and `last_error`. The diagnostics page reads this registry to display system health.

### Error handling

Errors flow through two paths:

- **Structured**: `AppError` enum with `AppErrorSeverity` (Warning / Error). Emitted as `AppEventKind::AppError` and displayed via `error_surface::report`.
- **Telemetry**: `telemetry::record_error` sends errors to the active sink.

Neither path panics. The panic hook in `lifecycle` captures unexpected panics and logs them for the diagnostics page.

### Window bounds persistence

`AppRoot` registers `cx.observe_window_bounds(...)` which writes the current window position and size to `AppConfig` on every resize/move. On next launch, `create_new_window` reads `persisted_bounds` and restores the window.

### Atomic config writes

`app_state::save_config` uses `AtomicWriteFile` to prevent partial writes. If the process crashes mid-write, the previous config file remains intact.
