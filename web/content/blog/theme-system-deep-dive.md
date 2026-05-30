---
title: "Deep dive: the 21-theme system"
description: "How gpui-starter's theme system works, from color tokens to live hot-reloading."
date: 2026-05-03
tags: [GPUI, themes, design]
draft: false
---

gpui-starter ships with 21 built-in themes. Not stubs or placeholders: fully designed color schemes covering light, dark, and high-contrast modes. Here's how the system works.

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

Every color in the app references a semantic token rather than a raw hex value. This means switching themes changes every color in the app consistently, with no orphaned values left behind.

## How ThemeColors works

The `ThemeColors` struct is the backbone of the entire system. It holds exactly ten semantic color fields, and every component in the framework reads from one of them. The struct is deliberately small. Having fewer tokens means less ambiguity when you sit down to design a new theme, and it means every token gets real use across the UI.

Here is what each field controls.

`background` is the base layer. It fills the window behind all other content. Pick something your eyes can tolerate for hours.

`foreground` is the default text color. It sits on top of `background`, so contrast ratio matters here more than anywhere else. If you get this pair wrong, the rest of the theme will not save you.

`accent` is the action color. Buttons, active tabs, selected items, links, and focus rings all draw from this token. A good accent color is saturated enough to stand out against both `background` and `surface` but not so loud that it fights for attention with text.

`surface` is the elevated-layer color. Sidebars, cards, panels, and popovers sit on `surface` instead of `background` to create visual depth. In most dark themes it is slightly lighter than `background`. In light themes it is usually white or near-white.

`border` draws the lines between sections, around inputs, and under headers. It should be visible enough to define structure but muted enough not to look like a wireframe. Many themes set this to a low-opacity version of `foreground`.

`error`, `warning`, and `success` are status colors. Red, yellow, and green are the conventional choices, but the exact shades should harmonize with the rest of the palette. A bright cherry red that works on a neutral dark background might look garish on a warm-toned theme like Gruvbox.

`muted` is for secondary or inactive text. Think placeholder text in inputs, timestamps, and disabled labels. It is usually a desaturated midpoint between `background` and `foreground`.

`text_dim` is similar to `muted` but reserved for text that needs to be readable but clearly secondary. Tooltips and metadata labels are common uses.

This fixed vocabulary is intentional. It avoids the problem that plagues CSS-in-JS systems where you have thirty shades of gray and no guidance on which one to use.

## The 21 themes

The built-in themes include:

Catppuccin in Latte, Frappe, Macchiato, and Mocha variants. Dracula with its signature purple accents on a dark base. Nord with an arctic blue palette that manages to feel calm without being boring. Gruvbox in Dark and Material variants, both built around warm earth tones. Tokyo Night in Storm and Night variants, which favor deep blues and purples. Solarized in both Dark and Light, using its distinctive amber-and-blue complement scheme. One Dark Pro. Rose Pine in Dawn, Moon, and the original. Everforest Dark. Kanagawa in Wave and Dragon variants. And a Default Light plus Default Dark.

That covers most of the popular editor palettes. If your favorite is missing, adding it is straightforward.

## Live hot-reloading

Themes can be switched at runtime through the command launcher (Cmd+K). The change is instant. No restart required. This works because GPUI's rendering pipeline re-queries theme colors on every frame rather than caching them at startup.

```rust
fn switch_theme(cx: &mut AppContext, theme: &'static dyn Theme) {
    cx.set_global::<CurrentTheme>(CurrentTheme(theme));
    cx.refresh();
}
```

The `refresh()` call triggers a re-render of the entire view tree. Since GPUI is GPU-accelerated, this happens in a single frame with no visible flicker. You can cycle through all 21 themes in a few seconds and see each one render cleanly.

The mechanism is worth understanding if you plan to build your own theme-aware components. Because `ThemeColors` is stored as a GPUI global, any component can read the current theme at render time by calling `cx.theme()`. There is no subscription system, no event bus, no reactive primitive to wire up. You read the global, you use the colors, and GPUI handles the rest. This is one of the advantages of a retained-mode GPU UI framework: state changes propagate naturally through the render loop without requiring the developer to manage invalidation.

## Creating a custom theme from scratch

Let's walk through building a complete theme. Suppose you want a theme inspired by GitHub's dark mode.

First, create a new file at `src/theme/github_dark.rs`:

```rust
use crate::theme::{Theme, ThemeColors};
use gpui::rgb;

pub struct GitHubDark;

impl Theme for GitHubDark {
    fn name(&self) -> &str { "GitHub Dark" }

    fn colors(&self) -> ThemeColors {
        ThemeColors {
            background: rgb(0x0d1117),  // deep navy-black
            foreground: rgb(0xe6edf3),  // off-white
            accent: rgb(0x58a6ff),      // GitHub blue
            surface: rgb(0x161b22),     // slightly lighter than bg
            border: rgb(0x30363d),      // subtle gray-blue
            error: rgb(0xf85149),       // GitHub red
            warning: rgb(0xd29922),     // amber
            success: rgb(0x3fb950),     // GitHub green
            muted: rgb(0x484f58),       // mid gray for borders
            text_dim: rgb(0x8b949e),    // secondary text
        }
    }
}
```

Next, register the theme so the command launcher can find it. Open `src/theme/mod.rs` and add your module, then insert it into the registry:

```rust
mod github_dark;

pub fn register_all_themes(registry: &mut ThemeRegistry) {
    registry.register(github_dark::GitHubDark);
    // ... existing registrations
}
```

Run `cargo run`, open the command launcher with Cmd+K, type "GitHub Dark", and select it. The entire app switches over in one frame.

When designing your own palette, start with `background` and `foreground` and get the contrast ratio above 7:1 if you can. Then pick `accent` to complement or pop against those two. `surface` should be close to `background` but distinguishable. `border` should be visible without being heavy. The status colors (`error`, `warning`, `success`) can follow convention. `muted` and `text_dim` fill in the gaps.

If you want to test contrast ratios quickly, the WebAIM contrast checker is a reliable tool. GPUI renders text with subpixel antialiasing on macOS, so actual perceived contrast may be slightly higher than the raw math suggests, but the checker gives you a solid baseline.

## Design tokens

The fixed-token approach is one of the better decisions in this codebase. It keeps themes composable and portable. You never have to wonder whether a component expects a raw color or a token. It always expects a token. This contract between the theme system and the component library is what makes it possible to ship 21 themes that all look correct without per-theme patch files or override styles.

See the [themes documentation](/docs/themes/) for the full token reference.
