---
title: "Why Rust + GPUI for desktop apps"
description: "The case for building desktop applications with Rust and GPUI instead of Electron, Tauri, or Qt."
date: 2025-05-05
tags: [Rust, desktop, architecture]
draft: false
---

The desktop app landscape is dominated by Electron. It's easy to use, has a massive ecosystem, and ships everywhere. But it comes with real costs: bloated binaries, high memory usage, and a constant battle with performance.

Rust + GPUI offers a fundamentally different approach.

## The problem with Electron

A typical Electron app bundles an entire Chromium instance. Hello World is 150MB+. Slack uses 1GB of RAM. VS Code — probably the best Electron app ever built — still struggles with large files.

These aren't edge cases. They're architectural consequences of running a web browser as your UI framework.

## Why not Tauri?

Tauri improves on Electron by using the system webview instead of Chromium. This cuts binary size and memory usage significantly. But you're still writing HTML, CSS, and JavaScript — the same web stack that makes Electron apps feel sluggish on native interactions.

Tauri is great for simple apps. But for anything with complex layouts, real-time rendering, or heavy UI interactions, you'll hit the same webview performance ceiling.

## GPUI's approach

GPUI renders directly to the GPU. Not through a webview, not through a native widget toolkit — directly to Metal/Vulkan/DirectX through a thin abstraction layer.

This means:

- **Single-digit millisecond render times** for complex UIs
- **Consistent frame budgets** — no GC pauses, no JIT warmup
- **Native text rendering** with proper font shaping
- **Predictable memory usage** — no hidden allocation spikes

## The Rust advantage

Using Rust isn't just about performance. It's about correctness:

- **No null pointer exceptions** — the type system catches them at compile time
- **No data races** — the borrow checker guarantees thread safety
- **Fearless refactoring** — if it compiles, it probably works
- **Minimal runtime** — no garbage collector, no event loop overhead

For a desktop app that users rely on daily, these guarantees matter. Crashes and hangs aren't annoyances — they're trust violations.

## The tradeoff

GPUI is newer and less mature than Electron or Qt. The ecosystem is smaller. The learning curve is steeper if you're not familiar with Rust.

But the fundamentals are solid. GPUI powers Zed — a production code editor used by thousands of developers daily. The framework is battle-tested where it matters most.

## Getting started

If you're curious, [gpui-starter](/docs/getting-started/) gives you a working desktop app in under five minutes. No configuration, no boilerplate — just `cargo run`.
