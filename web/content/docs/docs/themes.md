---
title: Themes
description: Theme system with 21 built-in themes, hot-reload, and runtime switching
---

## Overview

gpui-starter uses `gpui-component`'s `ThemeRegistry` to manage themes. Themes are JSON files in the `themes/` directory that are loaded at startup and hot-reloaded on file change.

## Built-in themes

| Theme | Style |
|-------|-------|
| Adventure | Dark, warm tones |
| Alduin | Dark, reddish |
| Asciinema | Dark, terminal-like |
| Ayu | Dark and light variants |
| Catppuccin | Pastel dark (Mocha, Macchiato, Frappé, Latte) |
| Everforest | Dark, nature-inspired |
| Fahrenheit | Dark, warm |
| Flexoki | Ink-on-paper inspired |
| Gruvbox | Dark, retro warmth |
| Harper | Light, elegant |
| Hybrid | Dark, balanced |
| Jellybeans | Dark, colorful |
| Kibble | Dark, muted |
| macOS Classic | Light, system-native |
| Matrix | Dark, green on black |
| Mellifluous | Dark, purple tones |
| Molokai | Dark, vibrant |
| Solarized | Dark and light, scientific |
| Spaceduck | Dark, purple |
| Tokyo Night | Dark, blue-purple |
| Twilight | Dark, soft |

## Theme JSON structure

Each theme is a JSON file with color definitions for the GPUI component system. Example structure:

```json
{
  "name": "My Theme",
  "mode": "dark",
  "appearance": "dark",
  "colors": {
    "background": "#1a1b26",
    "foreground": "#c0caf5",
    "primary": "#f59e0b",
    "border": "#292e42"
  }
}
```

Look at `themes/tokyonight.json` for a full example with all available tokens.

## Adding custom themes

1. Create a new `.json` file in the `themes/` directory
2. Follow the JSON structure from an existing theme
3. The theme is automatically picked up by hot-reload — no restart needed

```bash
# Create a new theme
cp themes/tokyonight.json themes/my-theme.json
# Edit the colors, name, and mode
# The app reloads the theme automatically
```

## Hot-reload

Theme files are watched via `ThemeRegistry::watch_dir()` in `src/app.rs`. When a file changes:

1. The registry reloads the JSON
2. If a persisted theme matches, it's re-applied
3. All windows refresh with the new colors

## Runtime theme switching

Switch themes programmatically using the `SwitchTheme` action:

```rust
cx.on_action(|switch: &SwitchTheme, cx| {
    if let Some(config) = ThemeRegistry::global(cx)
        .themes()
        .get(&switch.0)
        .cloned()
    {
        Theme::global_mut(cx).apply_config(&config);
    }
    cx.refresh_windows();
});
```

Or toggle light/dark mode with `SwitchThemeMode`:

```rust
cx.on_action(|switch: &SwitchThemeMode, cx| {
    Theme::change(switch.0, None, cx);
    cx.refresh_windows();
});
```

## Theme persistence

The current theme is saved to `target/state.json` on every change:

```json
{
  "theme": "Tokyo Night",
  "scrollbar_show": "scrolloff"
}
```

This state is restored on the next app launch.
