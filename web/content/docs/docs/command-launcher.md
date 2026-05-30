---
title: "Command launcher"
description: "The Cmd+K command palette with fuzzy search, action registry, and popup window management"
---

## Overview

The command launcher is a Spotlight-style search overlay activated with **Cmd+K** (macOS) or **Ctrl+K** (Linux/Windows). Pressing **/** also opens it. It provides filtered search across all registered commands and dispatches the selected action.

The launcher is implemented across two modules:

| Module | Responsibility |
|--------|---------------|
| `src/launcher.rs` | Popup window, search input, filtered results list, keyboard navigation |
| `src/commands.rs` | Command registry, availability checks, action execution |

## Architecture

The launcher opens as a `WindowKind::PopUp` window centered on the primary display. A `LauncherOpen` global prevents double-opening. The `Launcher` entity manages the search state and result list. When the user selects a command, the `LauncherRoot` parent handles the `LauncherEvent::Act` event, calls `commands::execute()`, and closes the popup window.

```
User presses Cmd+K
  -> ToggleSearch action
  -> open_launcher()
  -> PopUp window with LauncherRoot entity
  -> Launcher entity (search input + result list)
  -> User selects a command
  -> LauncherEvent::Act(LauncherActionKind::Execute(CommandId))
  -> commands::execute(command_id, cx)
  -> Window closes
```

## CommandId enum

Every command has a unique variant in the `CommandId` enum. This is the stable identifier used throughout the system.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandId {
    OpenHome,
    OpenForm,
    OpenSettings,
    OpenNotifications,
    OpenDiagnostics,
    OpenAbout,
    ThemeLight,
    ThemeDark,
    StartDemoTask,
    CheckConnectivity,
    CopyDiagnostics,
    OpenLogsFolder,
    OpenConfigFolder,
    Undo,
    Redo,
}
```

## CommandSpec struct

Each command is described by a `CommandSpec`, which provides the data shown in the launcher UI.

```rust
pub struct CommandSpec {
    pub id: CommandId,
    pub title: SharedString,
    pub subtitle: SharedString,
    pub icon: IconName,
}
```

| Field | Purpose |
|-------|---------|
| `id` | The `CommandId` variant used for dispatch |
| `title` | Primary label shown in the launcher |
| `subtitle` | Secondary description below the title |
| `icon` | Icon rendered next to the title |

## Command registry

The `registry()` function returns the full list of commands available in the launcher.

```rust
pub fn registry() -> Vec<CommandSpec> {
    vec![
        command(CommandId::OpenHome, "Home", "Open the Home page", IconName::Inbox),
        command(CommandId::OpenForm, "Form", "Open the Form page", IconName::File),
        command(CommandId::OpenSettings, "Settings", "Open the Settings page", IconName::Settings2),
        command(CommandId::ThemeLight, "Light Mode", "Switch to light theme", IconName::Sun),
        command(CommandId::ThemeDark, "Dark Mode", "Switch to dark theme", IconName::Moon),
        // ...
    ]
}

fn command(id: CommandId, title: &str, subtitle: &str, icon: IconName) -> CommandSpec {
    CommandSpec {
        id,
        title: title.into(),
        subtitle: subtitle.into(),
        icon,
    }
}
```

### Built-in commands

| CommandId | Title | Description |
|-----------|-------|-------------|
| `OpenHome` | Home | Navigate to the home page |
| `OpenForm` | Form | Navigate to the form page |
| `OpenSettings` | Settings | Navigate to settings |
| `OpenNotifications` | Notifications | Navigate to notifications |
| `OpenDiagnostics` | Diagnostics | Navigate to diagnostics |
| `OpenAbout` | About | Navigate to about page |
| `ThemeLight` | Light Mode | Switch to light theme |
| `ThemeDark` | Dark Mode | Switch to dark theme |
| `StartDemoTask` | Start Demo Task | Start a background task |
| `CheckConnectivity` | Check Connectivity | Run a network probe |
| `CopyDiagnostics` | Copy Diagnostics | Copy diagnostics to clipboard |
| `OpenLogsFolder` | Open Logs Folder | Open logs in file manager |
| `OpenConfigFolder` | Open Config Folder | Open config in file manager |
| `Undo` | Undo | Undo last reversible command |
| `Redo` | Redo | Redo last reversed command |

## Command availability

Some commands depend on runtime conditions. The `availability()` function checks whether a command can run and provides a reason when it cannot.

```rust
pub fn availability(id: CommandId, cx: &App) -> CommandAvailability {
    let desktop = crate::desktop_actions::snapshot(cx);
    match id {
        CommandId::CopyDiagnostics => CommandAvailability {
            enabled: desktop.clipboard_available,
            disabled_reason: (!desktop.clipboard_available)
                .then_some("Clipboard backend unavailable".into()),
        },
        CommandId::Undo => CommandAvailability {
            enabled: crate::undo_stack::can_undo(cx).is_some(),
            disabled_reason: crate::undo_stack::can_undo(cx)
                .is_none()
                .then_some("No undo available".into()),
        },
        // ... all other commands default to enabled
    }
}
```

| Condition | Commands affected |
|-----------|------------------|
| Clipboard unavailable | `CopyDiagnostics` |
| System opener unavailable | `OpenLogsFolder`, `OpenConfigFolder` |
| No undo history | `Undo` |
| No redo history | `Redo` |

## Command execution

The `execute()` function dispatches a `CommandId` to its handler.

```rust
pub fn execute(id: CommandId, cx: &mut App) {
    match id {
        CommandId::OpenHome => navigate(Page::Home, cx),
        CommandId::ThemeLight => crate::app::set_theme_mode(ThemeMode::Light, cx),
        CommandId::StartDemoTask => crate::tasks::start_demo_task(cx),
        CommandId::CopyDiagnostics => {
            if let Err(error) = crate::desktop_actions::copy_diagnostics(cx) {
                crate::error_surface::report(
                    format!("Copy diagnostics failed: {error}"),
                    crate::errors::AppErrorSeverity::Error,
                    vec![ErrorAction::Retry, ErrorAction::Dismiss],
                    cx,
                );
            }
        }
        // ...
    }
}
```

Navigation commands emit an `AppEventKind::Navigate(AppRoute)` event through the app event bus.

## Registering a new command

To add a command to the launcher:

1. Add a variant to `CommandId` in `src/commands.rs`:

```rust
pub enum CommandId {
    // ... existing variants
    MyNewCommand,
}
```

2. Add a `CommandSpec` entry to the `registry()` function:

```rust
command(CommandId::MyNewCommand, "My Command", "Does something useful", IconName::Star),
```

3. Add an availability check in `availability()` (or match it in the default enabled arm).

4. Add an execution arm in `execute()`:

```rust
CommandId::MyNewCommand => {
    // your logic here
}
```

The launcher picks up new registry entries automatically. No changes to `launcher.rs` are needed.

## Popup window configuration

The launcher opens as a `WindowKind::PopUp` window. The window bounds and appearance are set in `open_launcher()`:

```rust
let window_w = px(620.);
let window_h = px(460.);

let options = WindowOptions {
    window_bounds: Some(WindowBounds::Windowed(bounds)),
    titlebar: None,
    focus: true,
    show: true,
    kind: WindowKind::PopUp,
    is_movable: true,
    is_resizable: false,
    window_background: WindowBackgroundAppearance::Blurred,
    window_min_size: Some(gpui::Size {
        width: window_w,
        height: window_h,
    }),
    ..Default::default()
};
```

| Property | Value | Notes |
|----------|-------|-------|
| Size | 620 x 460 px | Fixed, non-resizable |
| Position | Centered on primary display, offset 12% from top | Falls back to fixed coordinates |
| Background | Blurred | Uses `WindowBackgroundAppearance::Blurred` |
| Title bar | None | Frameless popup |
| Movable | Yes | User can drag the window |

## Search and filtering

The `Launcher` entity performs case-insensitive substring matching against both `title` and `subtitle`:

```rust
fn refilter(&mut self, cx: &mut Context<Self>) {
    let q = self.input.read(cx).value().to_lowercase();
    self.filtered = if q.is_empty() {
        (0..self.items.len()).collect()
    } else {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.title.to_lowercase().contains(&q)
                    || item.subtitle.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    };
    self.selected_index = 0;
    cx.notify();
}
```

When the query is empty, all commands are shown. Results reset the selection to the first item.

## Keyboard navigation

The launcher binds three actions within its `CONTEXT` focus scope:

| Key | Action | Behavior |
|-----|--------|----------|
| Up | `SelectPrev` | Move selection up, wrapping to the last item |
| Down | `SelectNext` | Move selection down, wrapping to the first item |
| Escape | `Dismiss` | Close the launcher without executing |

Enter is handled through the `InputEvent::PressEnter` subscription on the search input, which calls `act()` to dispatch the selected command and close the window.

## Customizing launcher behavior

To change how the launcher appears or behaves, modify these areas:

**Window size and position**: Edit the `window_w`, `window_h`, and vertical offset in `open_launcher()`.

**Search behavior**: Replace the substring filter in `refilter()` with a different matching algorithm (fuzzy, prefix, weighted).

**Item rendering**: Modify the `Render` implementation for `Launcher` to change how results display (add badges, sections, keyboard hints).

**Action kinds**: Extend `LauncherActionKind` to support action types beyond `Execute`, then handle them in `LauncherRoot`'s subscription callback.

## See also

- [Getting started](/docs/getting-started/) for project structure and setup
- [Architecture](/docs/architecture/) for GPUI patterns used in the launcher
- [Themes](/docs/themes/) for theme switching commands
