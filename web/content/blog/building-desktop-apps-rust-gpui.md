---
title: "Why Rust + GPUI for desktop apps"
description: "The case for building desktop applications with Rust and GPUI instead of Electron, Tauri, or Qt."
date: 2026-05-05
tags: [Rust, desktop, architecture]
draft: false
---

The desktop app landscape is dominated by Electron. It's easy to use, has a massive ecosystem, and ships everywhere. But it comes with real costs: bloated binaries, high memory usage, and a constant battle with performance.

Rust + GPUI offers a fundamentally different approach.

## The problem with Electron

A typical Electron app bundles an entire Chromium instance. Hello World is 150MB+. Slack uses 1GB of RAM with a few workspaces open. VS Code, probably the best Electron app ever built, still struggles with large files and gets noticeably sluggish when extensions pile up.

These aren't edge cases. They're architectural consequences of running a web browser as your UI framework.

The numbers tell the story. A minimal Electron app starts around 130MB on disk and consumes 80 to 120MB of RAM at idle. Compare that with a native app doing the same thing at 5 to 15MB. The gap only widens as your app grows. Discord routinely sits at 500MB. Notion hovers around 400MB for a single workspace. Figma's desktop client, which does real GPU-accelerated rendering through WebGL, still carries the overhead of an entire Chromium process underneath.

The root cause is redundancy. Every Electron window runs its own renderer process. Each one carries a V8 JavaScript engine, a DOM implementation, a CSS parser, a layout engine, a compositor, and a networking stack. You're not just running your app. You're running a copy of Chrome for every window. On a machine with 8GB of RAM, three Electron apps can consume a quarter of available memory before the user has done anything.

## Why not Tauri?

Tauri improves on Electron by using the system webview instead of Chromium. This cuts binary size from hundreds of megabytes down to single digits and reduces idle memory to 30 or 40MB. Those are real wins.

But you're still writing HTML, CSS, and JavaScript. The same web stack that makes Electron apps feel sluggish on native interactions. WebView2 on Windows, WebKit on macOS, WebKitGTK on Linux. Each has its own rendering quirks, its own JavaScript performance characteristics, its own set of bugs you'll need to work around. Cross-platform consistency becomes an exercise in wrangling three different browsers.

Tauri is a good choice for simple apps. Tools that are basically forms with a native frame around them. But for anything with complex layouts, real-time rendering, or heavy UI interactions, you'll hit the same webview performance ceiling. The DOM is not designed for 60fps rendering of thousands of elements. CSS layout is fast enough for web pages but introduces unpredictable latency when you need tight frame budgets. And JavaScript's single-threaded model means your UI logic competes with your business logic for the same execution time.

## GPUI's approach

GPUI renders directly to the GPU. Not through a webview, not through a native widget toolkit, but straight to Metal on macOS and Vulkan elsewhere through a thin abstraction layer. There is no DOM, no CSS engine, no JavaScript runtime between your code and the pixels on screen.

The render pipeline works in three phases. First, layout: GPUI computes sizes and positions using a flexbox-based system, similar to how the web works but without the legacy baggage. Second, prepaint: hitboxes get created, text runs get shaped, and the framework prepares spatial data for interaction. Third, paint: quads, text, and decorations get batched into GPU draw calls.

```rust
impl Element for CounterButton {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn request_layout(
        &mut self, _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window, cx: &mut App,
    ) -> (LayoutId, ()) {
        let layout_id = window.request_layout(
            Style {
                size: size(px(120.), px(36.)),
                ..default()
            },
            vec![],
            cx,
        );
        (layout_id, ())
    }

    fn prepaint(
        &mut self, _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>, _: &mut (),
        window: &mut Window, _: &mut App,
    ) -> Hitbox {
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self, _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>, _: &mut (),
        hitbox: &mut Hitbox,
        window: &mut Window, cx: &mut App,
    ) {
        window.paint_quad(paint_quad(
            bounds, Corners::all(px(6.)),
            cx.theme().background,
        ));
        window.on_mouse_event({
            let hitbox = hitbox.clone();
            move |event: &MouseDownEvent, phase, window, cx| {
                if hitbox.is_hovered(window) && phase.bubble() {
                    cx.stop_propagation();
                }
            }
        });
    }
}
```

This direct pipeline means single-digit millisecond render times for complex UIs. No garbage collector pauses. No JIT warmup period. No layout thrashing from a DOM that was designed for documents, not applications. Text rendering goes through proper font shaping pipelines. Memory usage stays predictable because there are no hidden allocation spikes from a browser engine deciding to cache something.

## The Rust advantage

Using Rust isn't just about performance. It's about correctness, and correctness is what matters most in software people rely on every day.

The type system eliminates entire classes of bugs at compile time. Null pointer exceptions, which account for a staggering share of production crashes in C++ and Java applications, simply cannot happen. The `Option<T>` type forces you to handle the absence of a value explicitly. The compiler won't let you forget.

```rust
fn load_config(path: &Path) -> Result<Config, AppError> {
    let raw = fs::read_to_string(path)
        .map_err(AppError::Io)?;
    let config: Config = toml::from_str(&raw)
        .map_err(AppError::Parse)?;
    // No null checks needed. No defensive coding.
    // If this function returns, the config is valid.
    Ok(config)
}
```

Data races, the source of some of the hardest bugs to reproduce and fix in concurrent code, are ruled out by the borrow checker. If your code compiles, it is data-race free. Not "probably" data-race free. Not "data-race free if you followed the conventions correctly." Guaranteed, mathematically, by the type system.

This matters enormously for desktop apps. A crash in a background thread that corrupts user data isn't an annoyance. It's a trust violation. Users don't care whether the bug was in your code or in a third-party library. They just know your app ate their work. Rust's safety guarantees extend that protection across your entire dependency tree, because the same rules apply to every crate you pull in.

There's also the practical benefit of fearless refactoring. In large codebases, the fear of breaking something undocumented is a real drag on velocity. Rust turns that fear into a compile error. Change a function signature, and the compiler shows you every call site that needs updating. Reorder a struct field, and every pattern match that destructures it flags immediately. The feedback loop is tight and deterministic.

And there's no garbage collector. No runtime pause, no stop-the-world event, no unpredictable latency spike right when the user is dragging a slider or typing in a search box. Rust's ownership model means memory gets freed at deterministic, predictable points. For UI work, this is a significant advantage. Frame budgets are measured in single-digit milliseconds. A 10ms GC pause is the difference between smooth and stuttering.

## The tradeoff

GPUI is newer and less mature than Electron or Qt. The ecosystem is smaller. The learning curve is steeper if you're not familiar with Rust, and steeper still if you're not used to thinking about UI without a DOM.

Documentation is still growing. You'll occasionally need to read source code to understand how something works. The community is active but small compared to the web development world, so Stack Overflow won't have answers for every question.

But the fundamentals are solid. GPUI powers Zed, a production code editor used by thousands of developers daily. Code editors are among the most demanding desktop applications: they need to handle large files, real-time syntax highlighting, multi-cursor editing, project-wide search, extension systems, and terminal emulators, all running simultaneously without dropping frames. If GPUI handles that workload, it can handle most things you'd throw at it.

## Getting started

If you're curious, [gpui-starter](/docs/getting-started/) gives you a working desktop app in under five minutes. No configuration, no boilerplate. Just `cargo run`.
