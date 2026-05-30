---
question: "How do I install and run gpui-starter?"
description: "Clone the repository, ensure you have Rust and the required system dependencies, then run cargo run."
category: "Getting Started"
order: 3
---

## Prerequisites

- Rust (2024 edition): [install via rustup](https://rustup.rs)
- macOS: Xcode Command Line Tools (`xcode-select --install`)
- Linux: Vulkan headers and a compatible GPU driver

## Quick start

```bash
git clone https://github.com/hmziqrs/gpui-boilerplate.git gpui-app
cd gpui-app
cargo run
```

The first build takes a few minutes to compile GPUI and dependencies. Subsequent builds are fast thanks to incremental compilation.

## System dependencies

On **Ubuntu/Debian**, you may need:

```bash
sudo apt install libvulkan-dev libx11-dev libxcb1-dev libxcb-keysyms1-dev
```

On **macOS**, the Xcode Command Line Tools are sufficient. No additional dependencies are required.
