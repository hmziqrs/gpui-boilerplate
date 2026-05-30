---
question: "How does the macOS tray icon work?"
description: "gpui-starter includes a system tray icon with a quick-access menu for showing/hiding the app."
category: "Features"
order: 10
---

gpui-starter includes a **macOS system tray** icon with a context menu. When the app is running, an icon appears in the menu bar.

## Tray menu options

- Show: bring the app window to front
- Hide: minimize to tray
- Quit: exit the app

## How it works

The tray icon is registered during app initialization using GPUI's platform APIs. The icon and menu are defined in Rust: no separate native code or plugins needed.

```rust
fn setup_tray(cx: &mut AppContext) {
    cx.set_tray_menu(TrayMenu::new()
        .action("Show", show_window)
        .separator()
        .action("Quit", quit_app)
    );
}
```

The tray icon is optional: you can remove it by removing the `setup_tray` call in `app.rs`.
