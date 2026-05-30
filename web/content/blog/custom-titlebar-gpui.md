---
title: "Building a custom title bar in GPUI"
description: "How to replace the native window chrome with a custom title bar that matches your app design, including drag regions and traffic lights."
date: 2025-05-15
tags: [GPUI, Rust, design]
draft: false
---

The native title bar on macOS, Windows, and Linux works fine for most apps. But if you're building something with its own visual identity, that strip of system chrome at the top of the window sticks out. A custom title bar gives you control over background color, spacing, and what lives in that space: navigation breadcrumbs, search, actions, whatever your app needs.

gpui-starter ships a fully working custom title bar. Here's how it works.

## Why bother

Visual consistency is the obvious motivation. Your app has a theme system with carefully chosen colors and borders. The native title bar ignores all of that. It draws its own background, its own separator line, and its own text style. On a dark-themed app, the native title bar sits there looking like it belongs to a different program.

The title bar is also prime real estate. In a code editor, it holds the file path and git branch. In a design tool, it holds zoom controls. In gpui-starter, it holds the app menu, a settings dropdown, a search button, and a notifications button. That is five things that would otherwise need their own toolbar row.

And for apps that people use for hours, the chrome around the content matters. The title bar is the first thing users see.

## Setting up the transparent title bar

The first step is telling GPUI to make the native title bar invisible. You do this in your window options when opening the window.

```rust
let options = WindowOptions {
    window_bounds: Some(WindowBounds::Windowed(window_bounds)),
    titlebar: Some(TitleBar::title_bar_options()),
    window_min_size: Some(gpui::Size {
        width: px(480.),
        height: px(320.),
    }),
    kind: WindowKind::Normal,
    ..Default::default()
};
```

The `title_bar_options()` function returns configuration that tells GPUI to render the title bar as transparent and position the macOS traffic light buttons at a specific offset:

```rust
pub fn title_bar_options() -> TitlebarOptions {
    TitlebarOptions {
        title: None,
        appears_transparent: true,
        traffic_light_position: Some(gpui::point(px(9.0), px(9.0))),
    }
}
```

On macOS, `appears_transparent: true` hides the native title text and background while keeping the close/minimize/zoom buttons (the traffic lights) visible. On Linux, you also need to set `window_decorations: Some(gpui::WindowDecorations::Client)` to switch to client-side decorations. The title bar component handles the difference internally.

## The drag region problem

When you remove the native title bar, you lose the thing users drag to move the window around. You need to recreate that behavior yourself.

The GPUI title bar component solves this with mouse event tracking and `window.start_window_move()`:

```rust
div()
    .id("title-bar")
    .h(TITLE_BAR_HEIGHT)
    .on_mouse_down(MouseButton::Left, window.listener_for(&state, |state, _, _, _| {
        state.should_move = true;
    }))
    .on_mouse_up(MouseButton::Left, window.listener_for(&state, |state, _, _, _| {
        state.should_move = false;
    }))
    .on_mouse_move(window.listener_for(&state, |state, _, window, _| {
        if state.should_move {
            state.should_move = false;
            window.start_window_move();
        }
    }))
```

This tracks whether the user is holding the mouse button down. On the first mouse move event while the button is held, it calls `start_window_move()`, which hands control back to the operating system's window move behavior. The `should_move` flag is then set to false immediately so the move only starts once per drag.

Interactive elements inside the title bar (buttons, menus) need to call `cx.stop_propagation()` on their mouse down events. This prevents the drag handler from firing when the user is trying to click a button. Without this, clicking your search button would also start a window drag.

## Platform differences

The title bar component is 34 pixels tall on all platforms. The left padding is different: 80 pixels on macOS to leave room for the traffic lights, 12 pixels on Linux and Windows.

On macOS, the traffic light buttons are rendered by the system. GPUI positions them with `traffic_light_position` and they just work. Double-clicking the title bar calls `window.titlebar_double_click()`, which triggers the macOS system behavior (either minimize or zoom, depending on System Settings).

On Windows, the component renders its own control buttons (minimize, maximize, close) using the `WindowControlArea` API. Each button is mapped to the correct window action through GPUI.

On Linux, the component renders control buttons manually and handles click events directly:

```rust
.when(is_linux, |this| {
    this.on_mouse_down(MouseButton::Left, move |_, window, cx| {
        window.prevent_default();
        cx.stop_propagation();
    })
    .on_click(move |_, window, cx| {
        cx.stop_propagation();
        match icon {
            Self::Minimize => window.minimize_window(),
            Self::Restore | Self::Maximize => window.zoom_window(),
            Self::Close { .. } => window.remove_window(),
        }
    })
})
```

Right-clicking the title bar on Linux also shows the window menu via `window.show_window_menu()`.

## The AppTitleBar in gpui-starter

The `AppTitleBar` struct wraps the generic `TitleBar` component and adds app-specific content: the menu bar on the left, settings and action buttons on the right.

```rust
pub struct AppTitleBar {
    app_menu_bar: Entity<AppMenuBar>,
    settings: Entity<SettingsDropdown>,
    child: TitleBarChild,
}
```

The render method puts the menu bar in one flex child and the right-side controls in another. The right section has a fixed `on_mouse_down` handler that stops propagation, so clicking anywhere in that area doesn't trigger a window drag. This is the pattern you want: partition the title bar into a drag zone and an interactive zone.

The `child` field is a closure that returns an `AnyElement`. This lets different pages inject custom content into the title bar without the title bar knowing about them. It's a simple form of dependency injection that keeps the component flexible.

The `SettingsDropdown` is a focus-tracked div that uses the dropdown menu system to let users change font size and border radius at runtime. Changes go through `Theme::global_mut(cx)` and trigger a window refresh.

## How the pieces connect

In `root.rs`, the `AppRoot` creates the title bar during initialization:

```rust
let title_bar = cx.new(|cx| AppTitleBar::new(title, window, cx));
```

The render method places it at the top of the vertical flex layout, above the sidebar and content area:

```rust
v_flex()
    .size_full()
    .child(self.title_bar.clone())
    .child(/* sidebar + content */)
    .child(/* status bar */)
```

The title bar is an entity, not a function call that returns an element. This means it manages its own state and re-renders independently when its focus handle or dropdown state changes. The rest of the layout doesn't need to care about title bar updates.

## What to watch for

The biggest gotcha is the `stop_propagation()` calls. If you add a new interactive element to the title bar and forget to stop propagation on its mouse down, clicking it will start a window drag instead. Test every button, every dropdown, every clickable thing in the title bar.

Another thing: the title bar height is a constant at 34 pixels. If you change it, you also need to update the `traffic_light_position` on macOS so the buttons stay vertically centered.

The [architecture docs](/docs/architecture/) cover how the title bar fits into the overall view hierarchy. For the theme tokens that control title bar colors (`title_bar`, `title_bar_border`), see the [themes guide](/docs/themes/).

If you're starting a new GPUI project, [gpui-starter](/docs/getting-started/) has all of this wired up and ready to go.
