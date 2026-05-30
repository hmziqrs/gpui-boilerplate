---
title: "Undo and redo in Rust desktop apps"
description: "Building an undo/redo stack for GPUI applications with command pattern and type-safe state snapshots."
date: 2026-05-15
tags: [Rust, GPUI, desktop]
draft: false
---

Desktop users expect Ctrl+Z to work. Not "sometimes," not "in the text field," but everywhere. Pressing undo and getting nothing back feels like the app is lying to you. Web apps get away with skipping it because people have been trained to expect less from browsers. Desktop is different. The bar is higher.

## The command pattern

The standard approach is the command pattern. Each mutation becomes an object that knows how to apply itself and how to reverse itself. You store these objects on a stack. Undo pops the last one and runs its inverse. Redo pops from a second stack and runs it forward.

This works well in Rust because the type system enforces that every variant of your command enum handles both directions. Forget to implement the inverse for a new command and the compiler stops you.

```rust
#[derive(Clone, Debug)]
pub enum UndoKind {
    ThemeMode { before: ThemeMode, after: ThemeMode },
    TogglePanel { panel_id: String, was_open: bool },
    ChangeFontSize { before: f32, after: f32 },
}

fn apply_inverse(kind: &UndoKind, cx: &mut App) {
    match kind {
        UndoKind::ThemeMode { before, .. } => {
            set_theme_mode(*before, cx);
        }
        UndoKind::TogglePanel { panel_id, was_open } => {
            set_panel_visibility(panel_id, *was_open, cx);
        }
        UndoKind::ChangeFontSize { before, .. } => {
            set_font_size(*before, cx);
        }
    }
}
```

Every branch has to exist. There is no `default` escape hatch that silently does nothing.

## How gpui-starter does it

The `undo_stack` module in gpui-starter implements this pattern with two stacks: `past` and `future`. Recording a new entry pushes onto `past` and clears `future` (because any new action invalidates the redo chain). Undo pops from `past`, applies the inverse, and pushes onto `future`. Redo does the opposite.

```rust
#[derive(Clone, Debug, Default)]
pub struct UndoState {
    pub past: Vec<UndoEntry>,
    pub future: Vec<UndoEntry>,
    pub applying: bool,
}

pub fn record_theme_mode_change(
    before: ThemeMode,
    after: ThemeMode,
    cx: &mut App,
) {
    if before == after {
        return;
    }
    let mut model = UndoModel::from_state(snapshot(cx));
    model.record(UndoEntry {
        label: "Switch Theme".into(),
        undo_label: "Undo Theme Switch".into(),
        redo_label: "Redo Theme Switch".into(),
        created_at: AppTimestamp::now(),
        kind: UndoKind::ThemeMode { before, after },
    });
    cx.set_global(model.into_state());
}
```

The `applying` flag prevents re-recording. When undo applies the inverse, that inverse calls back into the same mutation function. Without the flag, you'd record the undo itself as a new operation, creating an infinite loop of state changes.

The whole thing is wired into GPUI's command system. Undo and redo show up in the command launcher (Cmd+K) and their availability is dynamic. If the `past` stack is empty, the undo command is grayed out with "No undo available."

## Cheap snapshots

You don't need to serialize your entire app state on every change. The command pattern avoids that by storing only the delta: what changed and what it was before. A theme switch stores two enums. A font size change stores two floats. A toggle stores a bool. These are bytes, not kilobytes.

If you do need full snapshots (for a text editor, say), consider copy-on-write data structures. Rust's `im` crate provides persistent vectors and hashmaps where cloning is O(1) because the structure shares memory with the previous version. You get snapshot semantics without copying everything.

## Memory limits

A Vec grows until it doesn't. If your app runs for hours and the user makes thousands of changes, that undo stack keeps expanding. You need a cap.

The simplest approach: check the length before pushing and drop the oldest entry when you exceed the limit.

```rust
const MAX_UNDO_HISTORY: usize = 200;

fn record(&mut self, entry: UndoEntry) {
    if self.applying {
        return;
    }
    if self.past.len() >= MAX_UNDO_HISTORY {
        self.past.remove(0);
    }
    self.past.push(entry);
    self.future.clear();
}
```

Two hundred entries is generous. Most users won't undo more than a few steps. If your entries are small (they should be), the memory cost is negligible. If they're large (embedded images, full document copies), lower the cap or compress old entries.

There are smarter strategies: grouping related changes into a single entry, expiring entries after a time limit, or compressing the oldest entries into a single checkpoint. But a fixed cap handles 99% of cases. Ship that first.

## Guard rails that matter

The `before == after` check in `record_theme_mode_change` is easy to overlook. If a user clicks "Dark Mode" when the theme is already dark, you'd record a no-op entry that undoes to the same state. The undo stack would look functional but do nothing. Users hate that more than no undo at all.

GPUI's `Global` trait makes the undo state available everywhere without passing references through half your app. The state lives in the GPUI context and any function with access to `cx` can read or modify it. This is the right tradeoff for a global concern like undo. You wouldn't want each widget maintaining its own history.

The `can_undo` and `can_redo` functions return `Option<String>` instead of `bool` because the string feeds directly into the command palette's disabled-state tooltip. "No undo available" is better than a grayed-out button with no explanation.

## Beyond theme switches

gpui-starter currently only records theme changes as undoable operations. That's a starting point, not the ceiling. Any mutation that a user can trigger and would want to reverse is a candidate: panel visibility, font size, locale changes, layout rearrangements, form submissions.

Each one is a new variant on `UndoKind`, an `apply_inverse` match arm, and a recording call at the mutation site. The pattern scales linearly. There are no architectural surprises waiting when you add the tenth undoable action.

For a deeper look at how gpui-starter organizes its modules and context systems, see the [architecture guide](/docs/architecture/).
