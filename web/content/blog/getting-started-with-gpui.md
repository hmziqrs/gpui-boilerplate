---
title: "Getting started with GPUI: a new paradigm for Rust desktop apps"
description: "A walkthrough of building your first desktop application with GPUI, Zed's GPU-accelerated UI framework for Rust."
date: 2025-05-10
tags: [GPUI, Rust, desktop]
draft: false
---

GPUI is a GPU-accelerated UI framework built by the [Zed](https://zed.dev) team. It's fast, expressive, and written entirely in Rust. If you've been waiting for a serious alternative to Electron or Qt for building desktop apps, GPUI is worth your attention.

## Why GPUI?

Traditional desktop frameworks render UI on the CPU. GPUI renders everything on the GPU using a retained-mode architecture similar to SwiftUI. This means:

- **Smooth 60fps+ rendering** even with complex layouts
- **Declarative API** where you describe *what* the UI should look like, not *how* to draw it
- **First-class async support** through GPUI's entity system
- **Zero JavaScript** — everything is Rust

## Setting up a new project

The fastest way to get started is with gpui-starter:

```bash
git clone https://github.com/hmziqrs/gpui-boilerplate.git
cd gpui-app
cargo run
```

That's it. You'll see a working desktop app with a sidebar, multiple pages, theme switching, and i18n support.

## Project structure

A typical gpui-starter project looks like this:

```
src/
├── app.rs          # AppRoot — the main window
├── page/
│   ├── home.rs     # Individual pages
│   ├── settings.rs
│   └── mod.rs
├── theme/
│   ├── mod.rs      # 21 built-in themes
│   └── catppuccin.rs
├── i18n/
│   ├── en.ftl      # English strings
│   └── zh-CN.ftl   # Chinese strings
└── command.rs      # Cmd+K launcher
```

## Your first component

GPUI components are Rust structs that implement `Render`:

```rust
struct Counter {
    count: usize,
}

impl Render for Counter {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_4()
            .child(
                Button::new("increment", "Count +1")
                    .on_click(cx.listener(|this, _, cx| {
                        this.count += 1;
                        cx.notify();
                    }))
            )
            .child(format!("Count: {}", self.count))
    }
}
```

No virtual DOM diffing. No reactivity graphs. Just Rust structs and method calls.

## What's next?

Check out the [architecture guide](/docs/architecture/) to understand how GPUI's entity and context systems work under the hood. Or dive into [themes](/docs/themes/) to customize the look of your app.
