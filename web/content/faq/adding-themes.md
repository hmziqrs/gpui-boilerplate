---
question: "How do I add a custom theme?"
description: "Create a new Rust file in src/theme/, implement the Theme trait, and register it in the theme registry."
category: "Features"
order: 6
---

## Creating a custom theme

1. **Create a new file** in `src/theme/`:

```rust
// src/theme/my_theme.rs
use crate::theme::{Theme, ThemeColors, rgb};

pub struct MyTheme;

impl Theme for MyTheme {
    fn name(&self) -> &str { "My Theme" }
    fn colors(&self) -> ThemeColors {
        ThemeColors {
            background: rgb(0x1a1b26),
            foreground: rgb(0xa9b1d6),
            accent: rgb(0xf59e0b),
            surface: rgb(0x24283b),
            border: rgb(0x3b4261),
            error: rgb(0xf7768e),
            success: rgb(0x9ece6a),
            warning: rgb(0xe0af68),
            muted: rgb(0x565f89),
            text_dim: rgb(0x787c99),
        }
    }
}
```

2. **Register it** in `src/theme/mod.rs` by adding it to the theme registry.

3. **Run the app** and select your theme from the command launcher (Cmd+K).

## Tips

- Use the **semantic token names** — don't hardcode colors in components
- Test your theme with both light and dark content
- The `accent` color is used for buttons, links, and highlighted elements
- `surface` is the card/panel background color
