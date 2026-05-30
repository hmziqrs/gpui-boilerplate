---
title: "Getting started with GPUI: a new paradigm for Rust desktop apps"
description: "A walkthrough of building your first desktop application with GPUI, Zed's GPU-accelerated UI framework for Rust."
date: 2026-05-09
tags: [GPUI, Rust, desktop]
draft: false
---

GPUI is a GPU-accelerated UI framework built by the [Zed](https://zed.dev) team. It's fast, expressive, and written entirely in Rust. If you've been waiting for a serious alternative to Electron or Qt for building desktop apps, GPUI is worth your attention.

This post walks through what GPUI is, why its approach is different from most UI frameworks, and how to get a working app running in minutes. By the end you'll have a solid mental model for how GPUI components work, how async state fits in, and where to go next.

## Why GPUI?

Most desktop frameworks render UI on the CPU. That works fine for simple forms, but it starts to break down once you have complex layouts, animations, or large lists. The CPU has plenty of general-purpose work to do already. Offloading rendering to the GPU frees it up and gives you smooth frame rates without much effort.

GPUI uses a retained-mode architecture. If you've used SwiftUI or Flutter's widget tree, the idea is similar. You describe your UI as a tree of elements. The framework holds onto that tree between frames and updates only the parts that change. This is different from immediate-mode frameworks like Dear ImGui, which re-draw the entire UI every frame. Retained mode tends to perform better for real applications because the framework can skip unchanged subtrees and batch GPU draw calls efficiently.

The API is declarative. You write code that describes what the interface should look like given the current state, not step-by-step instructions for how to draw it. When state changes, you call `cx.notify()` and GPUI re-renders the affected component. There is no virtual DOM diffing, no reactivity graph, and no macro magic. Just Rust structs and method calls.

Other things worth knowing up front: GPUI has first-class async support through its entity system, it ships with a built-in theme system, and it does not involve any JavaScript. Everything runs as native Rust.

## Setting up a new project

The fastest way to get started is with the boilerplate project:

```bash
git clone https://github.com/hmziqrs/gpui-boilerplate.git
cd gpui-app
cargo run
```

That's it. You'll see a working desktop app with a sidebar, multiple pages, theme switching, and i18n support. The boilerplate gives you enough structure to start building without having to wire up the window, event loop, and navigation from scratch.

GPUI requires Metal on macOS. Linux support is in progress using Vulkan. Windows is not yet supported. If you are on macOS, everything should work out of the box with a recent Rust toolchain.

## Project structure

A typical gpui-starter project looks like this:

```
src/
├── app.rs          # AppRoot: the main window
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

The `app.rs` file is the entry point. It creates the main window, sets up navigation, and initializes global state. Think of it as the root of your component tree.

The `page/` directory holds individual views. Each page is a Rust struct that implements `Render`. Navigation between pages is handled through GPUI's context system, which lets you push and pop views from a central stack.

The `theme/` directory contains theme definitions. GPUI themes are just Rust modules that set colors, font sizes, border radii, and other visual tokens. The boilerplate ships with 21 themes including Catppuccin, Nord, and Solarized, so you can pick something that looks reasonable without designing from scratch.

The `i18n/` directory uses Fluent (`.ftl`) files for localization. If you don't need multiple languages you can ignore it, but the plumbing is there if your app needs it.

The `command.rs` file implements a command palette triggered by Cmd+K. This is a pattern borrowed from VS Code and Zed itself. It reads well as a reference for how to wire up keyboard shortcuts and modal overlays in GPUI.

## Your first component

GPUI components are Rust structs that implement `Render`. Here is a simple counter:

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

A few things to notice. The component state lives in plain fields on the struct. The `render` method returns a tree of elements built with method chaining. `cx.listener` creates a callback that has mutable access to the component's state. After modifying state, you call `cx.notify()` to tell GPUI that this component needs to re-render.

There are no hooks, no dependency arrays, and no build-time code generation. The mental model is straightforward: your struct holds the state, `render` describes the UI for that state, and `cx.notify()` triggers a re-render when state changes.

## Async state and side effects

Real applications need to fetch data, read files, and do work that takes time. GPUI handles this through its entity system. You can spawn async tasks from a component and update state when they complete.

Here is an example that fetches data from an API and displays it:

```rust
struct UserProfile {
    user: Option<String>,
    loading: bool,
}

impl UserProfile {
    fn new(cx: &mut ViewContext<Self>) -> Self {
        let mut profile = Self {
            user: None,
            loading: true,
        };
        profile.fetch_user(cx);
        profile
    }

    fn fetch_user(&mut self, cx: &mut ViewContext<Self>) {
        self.loading = true;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let response = reqwest::get("https://api.example.com/user")
                .await
                .unwrap()
                .text()
                .await
                .unwrap();

            this.update(cx, |this, cx| {
                this.user = Some(response);
                this.loading = false;
                cx.notify();
            }).ok();
        }).detach();
    }
}

impl Render for UserProfile {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div().child(if self.loading {
            "Loading...".into_any_element()
        } else {
            div().child(format!("User: {}", self.user.as_deref().unwrap_or("Unknown")))
                .into_any_element()
        })
    }
}
```

The `cx.spawn` call runs an async block on GPUI's runtime. When the async work finishes, `this.update` gives you mutable access back on the main thread, where it is safe to modify state and call `cx.notify()`. This pattern keeps async work off the UI thread while avoiding locks or channels for state updates.

If you have used something like Swift's async/await with `@MainActor`, the model should feel familiar. GPUI guarantees that entity updates run on the main thread, so you never need to worry about concurrent access to your component state.

## What to try next

Once you have the boilerplate running and understand the basics of `Render` and `cx.notify()`, here are some good next steps.

Try adding a new page to the `page/` directory. Copy `home.rs` as a starting point, implement `Render` with your own layout, and register the page in the router inside `app.rs`. This teaches you how navigation works and how components compose.

Pick a theme from the `theme/` directory and modify it. Change some colors, adjust spacing tokens, run the app, and see what happens. Theme changes are applied at runtime, so you get fast feedback.

Build a small data-driven view. Fetch something from a public API using the async pattern shown above, and render it in a list. This exercises the full cycle: async work, state update, re-render.

Read the [architecture guide](/docs/architecture/) to understand how GPUI's entity and context systems work under the hood. That will make the callback patterns and `cx.spawn` feel less magical and more like the natural consequences of the design.

If you want to customize the look of your app, dive into [themes](/docs/themes/) for a full reference of available tokens and how to define your own.
