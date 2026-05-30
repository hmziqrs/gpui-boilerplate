---
title: "State management in GPUI: entities, globals, and context"
description: "How GPUI handles application state through entities, global singletons, and the context system."
date: 2026-05-17
tags: [GPUI, Rust, architecture]
draft: false
---

State management in most UI frameworks means choosing between state management libraries. Redux, MobX, Zustand, Signals, Recoil. GPUI has none of that. It has three mechanisms: `Entity<T>`, `Global`, and `Context`. That's the whole story. The hard part is knowing which one to reach for and when.

## The three mechanisms

`Entity<T>` is a window-scoped reference to a Rust struct. It is how you hold state that a component or a window owns and mutates. You create one with `cx.new()`, read it with `entity.read(cx)`, and mutate it with `entity.update(cx, |state, cx| ...)`. Every entity lives as long as something holds a strong reference to it. When the last reference drops, the entity is gone.

`Global` is app-scoped singleton state. Any type that implements the `Global` trait can be stored once and accessed from anywhere: `cx.global::<MyConfig>()`. Use it for things that exist once and never move between windows. Configuration, feature flags, shared service handles. Not for things that change every frame.

`Context<T>` is not state itself. It is the handle that GPUI gives you when it calls into your code. `App` at the top level. `Window` for window-specific operations. `Context<T>` inside a component's methods. The context is where you call `cx.notify()`, `cx.spawn()`, `cx.subscribe()`, and `cx.observe()`. It is the control plane for side effects and lifecycle.

## Hot, warm, and cold state

Not all state deserves the same treatment. The [architecture guide](/docs/architecture/) defines three tiers.

Hot state changes rapidly and is currently visible. The text in an active editor. The selected row in a list. The loading spinner on a button. This belongs in an `Entity<T>` owned by the active window or component. It gets created when the user opens the thing, mutated while they interact with it, and dropped when they close it.

Warm state has medium churn and potentially large cardinality. A catalog of items. A history index. Environment metadata. This should live in a normalized value store keyed by typed IDs, not as a bag of entities.

Cold state is large, archived, or disk-backed. Full response bodies. Stream logs. Anything that doesn't fit in memory comfortably. This belongs in SQLite and blob files. You load it on demand and keep only a preview in RAM.

The common mistake is treating everything as hot. Every item in a list gets its own entity. Every cached response gets held in memory. This works fine in a prototype. It falls apart at scale.

## Why Vec<Entity<Item>> is a trap

Here is the pattern that causes the most pain in GPUI apps:

```rust
// Do not do this
struct AppState {
    collections: Vec<Entity<Collection>>,
}
```

Every entity in that vector carries a reference-counted handle, an allocation in GPUI's entity store, and a subscription to the render loop. Multiply that by hundreds or thousands of items and you are burning memory and CPU on things the user is not looking at.

The correct approach is to keep the catalog as a value type and materialize entities only for the active item:

```rust
struct AppState {
    catalog: CollectionCatalog,           // value type, ID-keyed
    active_editor: Option<Entity<CollectionEditor>>,
}
```

`CollectionCatalog` is just a `HashMap<CollectionId, Collection>` or a `Vec<Collection>` with an index. It has no entity overhead. When the user opens a collection for editing, you create an `Entity<CollectionEditor>` from the relevant data. When they close it, the entity drops and everything cleans up.

## Cx.notify() and when not to call it

`cx.notify()` tells GPUI that something changed and the component needs to re-render. GPUI calls `render()` on the next frame. This is the only re-render trigger. There is no diffing, no dependency tracking, no proxy object that notices mutations. You call `cx.notify()` yourself, or you don't get a new frame.

This explicitness is a feature. You always know why a re-render happened: you asked for it. The danger is asking too often.

The first rule: never call `cx.notify()` from inside `render()`. That creates a feedback loop. Render calls notify, notify schedules another render, render calls notify again. The app freezes or burns CPU at 100%.

The second rule: batch your notifies during high-throughput updates. If a stream is pushing messages at 200 per second, calling `cx.notify()` per message saturates the render loop. Drain the available messages, then notify once:

```rust
cx.spawn(async move |mut cx| {
    loop {
        let msg = rx.recv().await?;
        entity.update(&mut cx, |this, _| this.buffer.push_back(msg)).ok();
        while let Ok(msg) = rx.try_recv() {
            entity.update(&mut cx, |this, _| this.buffer.push_back(msg)).ok();
        }
        entity.update(&mut cx, |_, cx| cx.notify()).ok();
    }
}).detach();
```

One notify per batch. The UI stays responsive. The buffer stays current.

## WeakEntity in async closures

Async closures outlive the scope where they are created. If you capture a strong `Entity<T>` in an async block and the user closes the window while the async work is running, the entity cannot be dropped. The strong reference in the closure keeps it alive. This is a leak.

Use `WeakEntity<T>` instead:

```rust
fn fetch_data(&mut self, cx: &mut Context<Self>) {
    let weak = cx.entity().downgrade();

    cx.spawn(async move |mut cx| {
        let data = fetch_from_api().await;

        weak.update(&mut cx, |state, cx| {
            state.data = Some(data);
            cx.notify();
        }).ok(); // .ok() intentionally swallows the error
    }).detach();
}
```

`WeakEntity::update()` returns `Result`. If the entity was already dropped, you get `Err` and the closure never runs. The `.ok()` call discards that error because a dropped entity during shutdown is normal behavior, not a bug.

Strong `Entity<T>` is acceptable in short-lived scoped flows where you can guarantee the entity outlives the closure. Anywhere else, use `WeakEntity`.

## Subscription lifecycle and the detach() trap

Subscriptions connect entity events to handler closures. You create one with `cx.subscribe(&entity, handler)`. The return value is a `Subscription` handle. As long as that handle exists, the subscription is active. When the handle drops, the subscription cancels.

This means you must store the `Subscription` somewhere if you want it to survive past the current function. Forgetting to store it is the most common subscription bug:

```rust
fn setup(&mut self, cx: &mut Context<Self>) {
    // Bug: subscription is dropped immediately
    cx.subscribe(&self.data_source, |this, source, event, cx| {
        this.handle_event(event, cx);
    });
}
```

That subscription fires zero times. The `Subscription` value is created and dropped in the same expression.

You have two options. Store the subscription on your struct so it lives as long as the component:

```rust
struct MyView {
    subs: Vec<Subscription>,
}

fn setup(&mut self, cx: &mut Context<Self>) {
    let sub = cx.subscribe(&self.data_source, |this, source, event, cx| {
        this.handle_event(event, cx);
    });
    self.subs.push(sub);
}
```

Or call `.detach()` if the subscription should live for the entire lifetime of the entity and you don't need to cancel it:

```rust
fn setup(&mut self, cx: &mut Context<Self>) {
    cx.subscribe(&self.data_source, |this, source, event, cx| {
        this.handle_event(event, cx);
    }).detach();
}
```

`.detach()` consumes the `Subscription` and transfers ownership to GPUI's internal tracking. The subscription stays alive until the entity it is attached to is dropped. It returns `()`, so you cannot store it. That is the point: it is a one-way trip.

The trap is calling `.detach()` and then wanting to cancel the subscription later. You can't. There is no handle to hold. If you need cancellation, store the `Subscription` and drop it manually.

## Putting it together

State in GPUI is just Rust structs. There is no magic runtime, no proxy layer intercepting your reads and writes. You own a struct, you mutate it through `entity.update()`, and you tell GPUI to re-render with `cx.notify()`. The framework does not try to be clever on your behalf.

This means the hard problems are the same ones you face in any Rust program: ownership, lifetime, and choosing the right data structure. GPUI gives you the tools. You make the decisions.

For the full state design guidelines including memory budgets, streaming backpressure, and cancellation patterns, see the [architecture docs](/docs/architecture/).
