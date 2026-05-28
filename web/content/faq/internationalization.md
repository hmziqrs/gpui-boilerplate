---
question: "How does internationalization work?"
description: "gpui-starter uses Mozilla's Fluent system via the es-fluent crate for type-safe, plural-aware translations."
category: "Features"
order: 7
---

gpui-starter uses **Mozilla's Fluent** localization system through the `es-fluent` Rust crate. Fluent handles the complexities of natural language — plural rules, gender agreement, and grammatical cases.

## Translation files

Strings are defined in `.ftl` files in `src/i18n/`:

```
# en.ftl
welcome = Welcome to {$app}!
items-count = { $count ->
    [one] {$count} item
   *[other] {$count} items
}
```

## Using translations

```rust
use crate::i18n::t;

div().child(t("welcome", cx))
```

## Switching languages

Languages can be changed at runtime through the command launcher. The change takes effect immediately.

## Adding a language

1. Create a new `.ftl` file in `src/i18n/`
2. Translate all keys from `en.ftl`
3. Register the locale in the app configuration

See the [i18n documentation](/docs/i18n/) for details.
