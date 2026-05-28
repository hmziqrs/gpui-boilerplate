---
title: "Deep dive: the 21-theme system"
description: "How gpui-starter's theme system works, from color tokens to live hot-reloading."
date: 2025-04-28
tags: [GPUI, themes, design]
draft: false
---

gpui-starter ships with 21 built-in themes. Not stubs or placeholders — fully designed color schemes covering light, dark, and high-contrast modes. Here's how the system works.

## Theme definition

Each theme is a Rust struct that implements the `Theme` trait:

```rust
pub struct CatppuccinMocha;

impl Theme for CatppuccinMocha {
    fn name(&self) -> &str { "Catppuccin Mocha" }
    fn colors(&self) -> ThemeColors {
        ThemeColors {
            background: rgb(0x1e1e2e),
            foreground: rgb(0xcdd6f4),
            accent: rgb(0x89b4fa),
            surface: rgb(0x313244),
            border: rgb(0x45475a),
            // ... more tokens
        }
    }
}
```

Every color in the app references a semantic token — `background`, `foreground`, `accent`, `surface`, `border` — not a raw hex value. This means switching themes changes every color in the app consistently.

## The 21 themes

The built-in themes include:

- **Catppuccin**: Latte, Frappé, Macchiato, Mocha
- **Dracula**: Classic dark with purple accents
- **Nord**: Arctic blue palette
- **Gruvbox**: Dark and Material variants
- **Tokyo Night**: Storm and Night variants
- **Solarized**: Dark and Light
- **One Dark**: Pro variant
- **Rose Pine**: Dawn, Moon, and original
- **Everforest**: Dark variant
- **Kanagawa**: Wave and Dragon
- **Default**: Light and Dark

## Live hot-reloading

Themes can be switched at runtime through the command launcher (Cmd+K). The change is instant — no restart required. This works because GPUI's rendering pipeline re-queries theme colors on every frame.

```rust
fn switch_theme(cx: &mut AppContext, theme: &'static dyn Theme) {
    cx.set_global::<CurrentTheme>(CurrentTheme(theme));
    cx.refresh();
}
```

The `refresh()` call triggers a re-render of the entire view tree. Since GPUI is GPU-accelerated, this happens in a single frame — there's no visible flicker.

## Creating custom themes

To add your own theme:

1. Create a new file in `src/theme/`
2. Implement the `Theme` trait with your colors
3. Register it in the theme registry
4. Run `cargo run` and select it from the launcher

```rust
// src/theme/my_theme.rs
pub struct MyTheme;

impl Theme for MyTheme {
    fn name(&self) -> &str { "My Custom Theme" }
    fn colors(&self) -> ThemeColors {
        ThemeColors {
            background: rgb(0x0d1117),
            foreground: rgb(0xe6edf3),
            accent: rgb(0xf59e0b),
            // ...
        }
    }
}
```

## Design tokens

The theme system uses a fixed set of semantic tokens. This is intentional — it keeps themes composable and prevents the "which color variable do I use?" problem that plagues CSS-in-JS systems.

The core tokens are: `background`, `foreground`, `accent`, `surface`, `border`, `error`, `success`, `warning`, `muted`, and `text_dim`. Every UI component references these tokens, never raw colors.

See the [themes documentation](/docs/themes/) for the full token reference.
