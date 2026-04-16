# GPUI Performance: Preventing Render Loops and Idle CPU

A pattern reference distilled from production render-loop investigations.
Each section is a named failure mode, its cause, and the canonical fix.

---

## Core Rule: render() is a pure projection

`render()` must read state and return elements — nothing else.
Any mutation inside `render()` that causes `cx.notify()` creates a feedback loop.

```rust
// BAD — entity.update() inside render schedules another render
fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    self.table.update(cx, |state, cx| {
        state.set_rows(self.compute_rows());
        state.refresh(cx); // cx.notify() → render → cx.notify() → ...
    });
    div()
}

// BETTER — dirty flag makes it fire once per data change, not every frame
// This is a narrow, acceptable exception for one-time initialization.
// Prefer pushing data from the event handler itself (see Pattern 1).
fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    if self.rows_dirty {
        self.rows_dirty = false; // cleared BEFORE update so re-render won't re-enter
        self.table.update(cx, |state, cx| {
            state.set_rows(self.cached_rows.clone());
            state.refresh(cx);
        });
    }
    div()
}
```

The prohibited list inside `render()` is broader than just `entity.update()`:
`set_value`, `cx.notify()`, `cx.subscribe()`, `cx.observe()`, `cx.spawn()`, and any entity
`update()` that emits events. Each can trigger handlers that schedule another render.
`cx.subscribe()` and `cx.observe()` are especially easy to miss — called every frame, they
accumulate subscriptions without bound.

---

## Pattern 1 — Dirty Flags for One-Time Data Push

The cleanest approach is to push data from the event handler that caused the change —
no render involvement at all:

```rust
fn on_data_received(&mut self, data: Data, cx: &mut Context<Self>) {
    self.rows = build_rows(&data);
    self.table.update(cx, |t, cx| { t.set_rows(self.rows.clone()); t.refresh(cx); });
    cx.notify();
}
```

When that isn't feasible (e.g., the data must be computed from render context), use a
dirty flag. The flag is set in the event handler, cleared in render before the push:

```rust
struct MyView {
    data_dirty: bool,
    table: Entity<TableState>,
}

// In render — narrow exception; flag cleared before update so re-renders won't re-enter
if std::mem::take(&mut self.data_dirty) {
    self.table.update(cx, |t, cx| { t.set_rows(self.rows.clone()); t.refresh(cx); });
}

// In the event handler
fn on_data_received(&mut self, data: Data, cx: &mut Context<Self>) {
    self.rows = build_rows(&data);
    self.data_dirty = true;
    cx.notify();
}
```

**Key:** `std::mem::take` reads and clears atomically. Clearing BEFORE the update means
a re-render triggered by the update won't re-enter the block.

---

## Pattern 2 — Guard Every notify() Against No-Op Changes

Every `cx.notify()` schedules a full re-render. Only call it when state actually changed.

```rust
// BAD — re-renders even when value is identical
fn on_something(&mut self, new_val: String, cx: &mut Context<Self>) {
    self.value = new_val;
    cx.notify();
}

// GOOD
fn on_something(&mut self, new_val: String, cx: &mut Context<Self>) {
    if self.value != new_val {
        self.value = new_val;
        cx.notify();
    }
}
```

Same rule for `cx.observe()` callbacks that reload external data:

```rust
cx.observe(&session, |this, _, cx| {
    let new_catalog = load_catalog();
    if this.catalog != new_catalog {   // equality guard
        this.catalog = new_catalog;
        cx.notify();
    }
});
```

---

## Pattern 3 — Bidirectional Sync Must Be Event-Driven

When two UI elements stay in sync (e.g., a URL bar and a params editor), each direction
must live in its own event handler. Never centralize sync in `render()`.

```rust
// BAD — sync in render creates: render → set_value → Change → notify → render → ...
fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    self.sync_inputs_from_model(w, cx); // calls set_value → emits Change → cx.notify()
    div()
}

// GOOD — each direction handled in its own subscription
fn new(cx: &mut Context<Self>) -> Self {
    // Direction 1: url_input → params editor
    cx.subscribe_in(&url_input, cx, |this, _, event, w, cx| {
        if let InputEvent::Change(val) = event {
            this.sync_params_from_url(&val, w, cx);
        }
    });
    // Direction 2: params editor → url_input (guarded)
    cx.subscribe_in(&params_editor, cx, |this, _, event, w, cx| {
        if let ParamsEvent::Changed = event {
            let new_url = this.build_url_from_params();
            if this.model.url != new_url {
                this.url_input.update(cx, |s, cx| s.set_value(new_url.clone(), w, cx));
                this.model.url = new_url;
            }
        }
    });
    // Initial population via dirty flag, not in render
    Self { draft_dirty: true, ... }
}
```

**`ReentrancyGuard` is a mitigation, not a cure.** A guard can suppress immediate
re-entrancy during a one-time init sync, but if the root cause (sync running in render)
is not removed, the deferred `cx.notify()` the guard emits fires on the next frame and
restarts the cycle. Fix the root cause first; treat the guard as a last-resort safety net.

---

## Pattern 4 — Subscription Cleanup on Row Rebuild

When you rebuild a list of child entities (e.g., KV editor rows), clear the old subscriptions
first. Dropped entity handles become no-ops but the `Subscription` objects still live in memory.

```rust
struct MyView {
    rows: Vec<RowEntity>,
    row_subs: Vec<Subscription>, // separate from long-lived subscriptions
}

fn rebuild_rows(&mut self, data: &[Row], cx: &mut Context<Self>) {
    self.row_subs.clear(); // drop old Subscription objects first
    self.rows = data.iter().map(|row| {
        let input = cx.new(|cx| InputState::new(cx, row.value.clone()));
        self.row_subs.push(cx.subscribe_in(&input, cx, Self::on_row_changed));
        input
    }).collect();
}
```

Without the `clear()`, each rebuild adds 2N new subscriptions forever.

---

## Pattern 5 — Observe Precisely, Not Broadly

Observe the entity that owns the data you care about. Observing a wide entity (e.g., a parent
view) and doing expensive work (DB reads, full tree rebuilds) on every notification is the
fastest way to create per-keystroke SQLite queries.

```rust
// BAD — AppRoot observes every RequestTabView, reloads catalog on each keystroke
cx.observe(&request_tab, |this, _, cx| {
    this.catalog = load_workspace_catalog(); // 5 SQLite queries per keypress
    cx.notify();
});

// GOOD — reload catalog only when the workspace tree actually mutated
fn on_request_saved(&mut self, ...) {
    self.catalog = load_workspace_catalog();
    cx.notify();
}
// Incidental RequestTabView notifications (typing, focus) never touch the catalog
```

If you must observe broadly, gate on a revision or identity field:

```rust
cx.observe(&request_tab, |this, tab, cx| {
    let revision = tab.read(cx).revision();
    if this.last_revision == revision { return; }
    this.last_revision = revision;
    this.catalog = load_workspace_catalog();
    cx.notify();
});
```

---

## Pattern 6 — External Side Effects in Render

External calls inside `render()` — webview loads, file reads, D-Bus calls — bypass GPUI's
change detection and run every frame.

```rust
// BAD — repaints WKWebView even when HTML is identical
fn render_preview(&mut self, cx: &mut Context<Self>) -> Div {
    self.webview.update(cx, |w, _| w.load_html(&self.html)); // every render
    div()
}

// GOOD — cache and compare
fn render_preview(&mut self, cx: &mut Context<Self>) -> Div {
    if self.last_html.as_deref() != Some(&self.html) {
        self.last_html = Some(self.html.clone());
        self.webview.update(cx, |w, _| w.load_html(&self.html));
    }
    div()
}
```

Same principle applies to file watchers: debounce the callback and check whether the loaded
content actually differs from the currently applied value before notifying observers.

---

## Pattern 7 — Async Task Hygiene

**Task lifecycle must be explicit.** Dropping a `Task<T>` handle cancels the task silently.
Either store it on the owning entity (long-lived operations) or call `.detach()` (true
fire-and-forget). Never accidentally drop a Task by letting it fall out of scope.

```rust
struct MyView {
    inflight: Option<Task<()>>, // stored → cancels when reassigned or entity drops
}

fn start_op(&mut self, cx: &mut Context<Self>) {
    self.inflight = Some(cx.spawn(async move { /* ... */ }));
    // detach for true fire-and-forget:
    // cx.spawn(async move { /* ... */ }).detach();
}
```

When consuming a channel in a spawned task, break on entity drop and guard with
`operation_id` to drop stale results from cancelled operations:

```rust
while let Some(event) = rx.recv().await {
    if let Err(_) = entity.update(cx, |this, cx| {
        if this.active_id != operation_id { return; } // stale operation guard
        this.process(event, cx);
    }) {
        break; // entity dropped — stop consuming the channel
    }
}
```

---

## Pattern 8 — Batch Notify for High-Throughput Streams

Per-message `cx.notify()` in a WebSocket or streaming HTTP response causes UI invalidation
on every incoming frame. At high throughput this saturates the render loop.

```rust
// BAD — notify on every message
while let Some(msg) = stream.next().await {
    entity.update(cx, |this, cx| {
        this.messages.push(msg);
        cx.notify(); // fires 100s of times/sec under load
    }).ok();
}

// GOOD — batch into a ring buffer, notify on flush cadence
// Network reader → bounded channel → UI flush task
let (tx, mut rx) = mpsc::channel::<Message>(256);

// Flush task: drain up to N messages, then notify once
cx.spawn(async move {
    loop {
        let msg = rx.recv().await?;
        entity.update(cx, |this, _| this.buffer.push(msg)).ok();
        // drain any already-queued messages without extra awaits
        while let Ok(msg) = rx.try_recv() {
            entity.update(cx, |this, _| this.buffer.push(msg)).ok();
        }
        entity.update(cx, |_, cx| cx.notify()).ok(); // one notify per batch
    }
}).detach();
```

Also keep the visible message buffer bounded (ring buffer, not `Vec`) to prevent RSS growth
during long sessions.

---

## Pattern 9 — Entity Reentrancy

Do not re-enter a mutable update on the same entity from within its own active update path.
GPUI will panic or silently no-op depending on context. Defer follow-up work instead:

```rust
// BAD — calls entity.update() on self from within self's update closure
fn do_work(&mut self, cx: &mut Context<Self>) {
    self.helper(cx); // if helper calls cx.update on Self, that's a re-entrant update
}

// GOOD — schedule follow-up via cx.notify() or a spawned task
fn do_work(&mut self, cx: &mut Context<Self>) {
    self.state = next_state;
    cx.notify(); // render will observe the new state; no re-entrant update needed
}
```

---

## Checklist

Before shipping a new view or subscriber:

- [ ] `render()` contains no `entity.update()`, `cx.notify()`, `cx.subscribe()`, `cx.observe()`, `cx.spawn()`, or external I/O
- [ ] Every `cx.notify()` is inside an `if value_changed` guard
- [ ] Bidirectional sync uses event handlers, not render; no `ReentrancyGuard` as a substitute fix
- [ ] Row-rebuild helpers call `subscriptions.clear()` before creating new rows
- [ ] Observers read only from the narrowest entity that carries the changed data
- [ ] External side effects (webview, file, DB) are cached and compared before re-applying
- [ ] High-throughput streams batch into a bounded buffer and call `cx.notify()` once per flush
- [ ] Every `Task<T>` is either stored on an entity field or explicitly `.detach()`ed
- [ ] Async tasks `break` on entity-drop error and check `operation_id` for staleness
- [ ] No re-entrant mutable updates on the same entity within a single update path
