---
question: "What is the command launcher?"
description: "The Cmd+K command launcher provides fuzzy search across all app actions including theme switching, navigation, and language changes."
category: "Features"
order: 9
---

The command launcher is a Spotlight-style search overlay activated with **Cmd+K** (macOS) or **Ctrl+K** (Linux). It provides fuzzy search across all registered app actions.

## Built-in actions

- **Navigate** to any page in the app
- **Switch themes** by name
- **Change language** between available locales
- **Toggle settings** like dark mode

## Registering custom actions

You can add your own commands to the launcher:

```rust
command_registry::register("my-action", "My Custom Action", |cx| {
    // handle action
});
```

The launcher uses fuzzy matching, so users don't need to type the exact command name. It's the fastest way to access any feature in the app.
