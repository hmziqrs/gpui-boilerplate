---
question: "How do keyboard shortcuts work?"
description: "gpui-starter supports configurable keyboard shortcuts for common actions."
category: "Features"
order: 11
---

gpui-starter ships with keyboard shortcuts for the actions you use most. Shortcuts are split into two categories: in-app key bindings and a global hotkey for bringing the window forward.

## Built-in shortcuts

- **Cmd+K** or **/** opens the command launcher with fuzzy search
- **Cmd+Z** / **Cmd+Y** undoes and redoes recent changes like theme switches
- **Cmd+Q** (macOS) or **Alt+F4** (Linux) quits the app
- **Escape** dismisses overlays such as the launcher

On macOS, a system-wide **Alt+Space** hotkey can bring the app to the foreground even when it is not focused. This hotkey uses the `global_hotkey` crate and is managed in the `shortcuts` module.

## How shortcuts are registered

In-app bindings are registered with `cx.bind_keys()` inside `app::init`. Each call to `KeyBinding::new` maps a key combination to an action type:

```rust
cx.bind_keys([
    KeyBinding::new("cmd-k", ToggleSearch, None),
    KeyBinding::new("/", ToggleSearch, None),
]);
```

The third argument is an optional context. Bindings with `None` apply globally. Context-scoped bindings, like the arrow keys in the launcher overlay, pass a context string so they only fire when that overlay is active.

## Customizing shortcuts

You can add your own bindings by calling `cx.bind_keys()` during initialization with your action type and preferred key combination. The global hotkey accelerator defaults to `Alt+Space` and can be toggled on or off from the Settings page under the global shortcut option. The setting persists across restarts through the app configuration file.
