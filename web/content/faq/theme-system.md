---
question: "How does the theme system work?"
description: "Themes are Rust structs implementing a Theme trait with semantic color tokens. Switching is instant via the command launcher."
category: "Features"
order: 5
---

The theme system uses **semantic color tokens** — every color in the app references a named token like `background`, `foreground`, or `accent`, not a raw hex value.

## How themes are defined

Each theme implements the `Theme` trait and returns a `ThemeColors` struct:

```rust
impl Theme for Nord {
    fn name(&self) -> &str { "Nord" }
    fn colors(&self) -> ThemeColors {
        ThemeColors {
            background: rgb(0x2e3440),
            foreground: rgb(0xd8dee9),
            accent: rgb(0x88c0d0),
            // ...
        }
    }
}
```

## Switching themes

Themes can be switched at runtime through the command launcher (Cmd+K). The change takes effect in a single frame — no restart needed.

## Built-in themes

21 themes are included: Catppuccin (4 variants), Dracula, Nord, Gruvbox (2), Tokyo Night (2), Solarized (2), One Dark, Rose Pine (3), Everforest, Kanagawa (2), and default light/dark.

See the [themes documentation](/docs/themes/) for the full list and how to create custom themes.
