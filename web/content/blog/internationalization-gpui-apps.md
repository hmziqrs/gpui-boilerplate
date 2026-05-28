---
title: "Internationalization in GPUI apps with es-fluent"
description: "How gpui-starter handles multi-language support using Mozilla's Fluent system and the es-fluent Rust crate."
date: 2025-04-20
tags: [GPUI, i18n, Rust]
draft: false
---

Building an app for a global audience means supporting multiple languages. gpui-starter includes first-class i18n support using Mozilla's [Fluent](https://projectfluent.org/) system through the `es-fluent` Rust crate.

## Why Fluent?

Fluent is Mozilla's localization system, designed to handle the complexities of natural language that simpler systems (like JSON key-value maps) can't:

- **Plural rules** — "1 item" vs "2 items" vs "0 items" (varies by language)
- **Gender agreement** — adjectives and verbs change based on the subject's gender
- **Grammatical cases** — some languages have 6+ noun cases
- **Message attributes** — a single key can have variants for different contexts

## How it works

Translation files live in `src/i18n/` as `.ftl` (Fluent) files:

```
# src/i18n/en.ftl
app-title = gpui-starter
welcome = Welcome to {$app}!
page-home = Home
page-settings = Settings
items-count = { $count ->
    [one] {$count} item
   *[other] {$count} items
}
```

```
# src/i18n/zh-CN.ftl
app-title = gpui-starter
welcome = 欢迎来到 {$app}！
page-home = 首页
page-settings = 设置
items-count = { $count ->
    [other] {$count} 个项目
}
```

## Using translations in Rust

The `es-fluent` crate generates type-safe accessors at compile time:

```rust
use crate::i18n::t;

fn render_header(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
    div()
        .child(t("page-home", cx))
        .child(t("page-settings", cx))
}
```

The `t()` function looks up the current locale and returns the translated string. If a key is missing in the current locale, it falls back to English.

## Switching languages

Like themes, the language can be changed at runtime through the command launcher:

```rust
fn set_locale(cx: &mut AppContext, locale: &str) {
    cx.set_global::<Locale>(Locale::new(locale));
    cx.refresh();
}
```

The locale change takes effect immediately — all text in the app updates on the next frame.

## Adding a new language

To add support for a new language:

1. Create a new `.ftl` file in `src/i18n/`
2. Translate all keys from `en.ftl`
3. Register the locale in the app configuration
4. Run `cargo run` and select the language from the launcher

See the [i18n documentation](/docs/i18n/) for the complete reference.
