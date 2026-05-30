---
question: "Does gpui-starter support undo and redo?"
description: "Yes, gpui-starter includes a full undo/redo stack with keyboard shortcuts."
category: "Features"
order: 12
---

Yes. The `undo_stack` module provides a command-pattern undo/redo system built on a pair of stacks (`past` and `future`). Every reversible operation is recorded as an `UndoEntry` that stores its kind, a human-readable label, and a timestamp.

## Keyboard shortcuts

- **Cmd+Z** (macOS) / **Ctrl+Z** (Linux) undoes the last entry.
- **Cmd+Y** (macOS) / **Ctrl+Y** (Linux) redoes it.

The Edit menu updates its Undo and Redo items dynamically based on whether the stacks are non-empty.

## How it works

Recording an entry pushes it onto the `past` stack and clears the `future` stack, matching standard branch-on-write semantics. Undoing pops from `past`, applies the inverse action, then pushes onto `future`. Redoing reverses the process. An `applying` guard prevents re-recording while an undo or redo is in flight.

The stacks live in a GPUI global (`UndoState`) and have no fixed size limit. They grow with use until the app exits. If you need bounded memory, add a cap in `UndoModel::record` and drop the oldest entry when `past.len()` exceeds your threshold.

## Adding undo support to custom operations

Add a new variant to the `UndoKind` enum, implement its inverse in `apply_inverse` and its forward in `apply_forward`, then call a recording function (similar to `record_theme_mode_change`) at the call site where the mutation happens.
