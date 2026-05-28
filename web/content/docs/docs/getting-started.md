---
title: Getting Started
description: Install and set up gpui-starter
---

## Prerequisites

- **Rust** 1.85 or later ([install via rustup](https://rustup.rs))
- **macOS** (native), **Linux** (X11 + Wayland), or **Windows**
- On Linux: X11 and Wayland development headers

## Quick start

```bash
git clone https://github.com/hmziqrs/gpui-boilerplate.git
cd gpui-boilerplate
cargo run
```

The app window opens with four pages: **Home**, **Form**, **Settings**, and **About**.

## Project structure

```
gpui-app/
├── Cargo.toml          # Dependencies and build config
├── build.rs            # es-fluent asset tracking
├── i18n.toml           # rust-i18n configuration
├── src/
│   ├── main.rs         # Entry point
│   ├── app.rs          # App init, actions, window creation, theme persistence
│   ├── root.rs         # AppRoot layout: title bar + sidebar + content
│   ├── sidebar.rs      # Page enum with titles and icons
│   ├── title_bar.rs    # Custom title bar with menus
│   ├── launcher.rs     # Cmd+K command palette
│   ├── menus.rs        # Native menu bar
│   ├── tray.rs         # macOS system tray (macOS only)
│   ├── i18n.rs         # es-fluent initialization and helpers
│   └── views/
│       ├── home.rs      # Welcome page
│       ├── form_page.rs # Registration form with validation
│       ├── settings.rs  # Dark mode, language, notifications
│       └── about.rs     # About page
├── themes/             # 21 JSON theme files (hot-reloadable)
└── i18n/
    ├── en/             # English translations
    └── zh-CN/          # Simplified Chinese translations
```

## What you get

| Feature | Details |
|---------|---------|
| Multi-page app | Resizable sidebar with 4 pages |
| 21 themes | Catppuccin, Dracula, Tokyo Night, and more |
| i18n | English + Simplified Chinese via es-fluent |
| Form validation | gpui-form + koruma with fluent error messages |
| Command launcher | Cmd+K spotlight search |
| macOS tray | Menu bar icon with global hotkey |
| Theme persistence | Survives restarts via `target/state.json` |

## Next steps

- [Theme system](/docs/themes/) — customize or add themes
- [Internationalization](/docs/i18n/) — add new languages
- [Forms](/docs/forms/) — build validated forms
- [Architecture](/docs/architecture/) — understand GPUI patterns
