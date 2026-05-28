---
question: "What is GPUI?"
description: "GPUI is a GPU-accelerated UI framework for Rust, built by the Zed team for building high-performance desktop applications."
category: "Getting Started"
order: 1
---

GPUI is a **GPU-accelerated UI framework** for Rust, developed by the [Zed](https://zed.dev) team. It renders user interfaces directly to the GPU (Metal on macOS, Vulkan elsewhere), providing smooth 60fps+ performance without the overhead of web browsers or native widget toolkits.

## Key characteristics

- **Retained mode** — you describe *what* the UI looks like, GPUI handles rendering
- **Declarative API** — similar in spirit to SwiftUI or Jetpack Compose
- **Async-first** — built-in support for async operations through GPUI's entity system
- **Pure Rust** — no JavaScript, no C++ bindings, no FFI overhead

GPUI powers Zed, a production code editor used by thousands of developers. The framework is battle-tested for real-world desktop application workloads.
