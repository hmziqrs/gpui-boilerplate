---
title: "Internationalization in GPUI apps with es-fluent"
description: "How gpui-starter handles multi-language support using Mozilla's Fluent system and the es-fluent Rust crate."
date: 2026-05-01
tags: [GPUI, i18n, Rust]
draft: false
---

Building an app for a global audience means supporting multiple languages. gpui-starter includes first-class i18n support using Mozilla's [Fluent](https://projectfluent.org/) system through the `es-fluent` Rust crate.

## Why not just use JSON?

Most i18n setups in the Rust ecosystem reach for a JSON file or an i18n JSON crate. That works fine if your app only needs to swap out static strings. The moment you need plural forms, gender agreement, or grammatical case, a flat key-value map starts to fight you. You end up embedding logic in your Rust code that belongs in the translation files themselves.

Fluent was designed by Mozilla to solve exactly this. Translation files use a `.ftl` format that is not a data format at all. It is a small language. A single Fluent message can branch on plural categories, interpolate variables, and carry attributes for different contexts, all without touching your Rust code.

Consider plurals. English has two plural categories: "one" and "other." Russian has four. Arabic has six. A JSON approach forces you to invent your own plural key scheme for every language and then maintain it. Fluent already knows the plural rules for every locale. You just write the selector and it picks the right branch.

## Plural rules with selectors

Here is a basic English plural:

```
items-count = { $count ->
    [one] {$count} item
   *[other] {$count} items
}
```

The asterisk marks the default variant. When `$count` is 1, Fluent picks `[one]`. For any other number it falls back to `*[other]`.

Now the same message in simplified Chinese. Chinese does not distinguish singular from plural in the way English does, so the file only needs the default:

```
items-count = { $count ->
   *[other] {$count} 个项目
}
```

You do not need to teach Chinese about English plural rules. Each locale file declares only the categories its language uses. If you later add Polish, which has "one," "few," "many," and "other," you just add those branches in the Polish `.ftl` file. No Rust code changes.

A more elaborate example that mixes a variable with a selectable count:

```
unread-messages = { $count ->
    [one] {$user} has {$count} unread message
   *[other] {$user} has {$count} unread messages
}
```

The translator controls the sentence structure entirely from the `.ftl` file. The Rust side just passes `$count` and `$user` as arguments.

## Gender agreement in practice

Some languages change adjectives or verb forms based on the grammatical gender of a noun. Fluent handles this with the same selector mechanism used for plurals.

```
greeting = { $gender ->
    [masculine] Bienvenido, {$name}
    [feminine] Bienvenida, {$name}
   *[other] Bienvenidx, {$name}
}
```

The caller passes a `gender` variable. The translator decides what "masculine" and "feminine" mean for that specific string. Your Rust code does not need to know anything about Spanish adjective agreement. It just supplies the data and lets the translation file do the work.

This separation matters. A JSON approach would require either separate keys per gender (`greeting.masculine`, `greeting.feminine`) or sprintf-style format strings that the translator cannot rearrange. Fluent puts the branching in the translation file where the translator can see it and edit it.

## How it works in gpui-starter

Translation files live in the `i18n/` directory at the project root, organized by locale:

```
i18n/
  en/
    gpui-starter.ftl
  zh-CN/
    gpui-starter.ftl
```

Each `.ftl` file contains all the Fluent messages for that locale. The English file is the source of truth. Other locales translate from it.

## Compile-time safety with es-fluent

The `es-fluent` crate and its companion `es-fluent-manager-embedded` do something most i18n libraries do not. They embed your `.ftl` files at compile time and generate type-safe accessors from them.

In `src/i18n.rs`, the setup looks like this:

```rust
use es_fluent::FluentLocalizer as _;

es_fluent_manager_embedded::define_i18n_module!();

static I18N: OnceLock<EmbeddedI18n> = OnceLock::new();

pub fn init_i18n(lang: LanguageIdentifier) -> Result<(), String> {
    let i18n = EmbeddedI18n::try_new_with_language(lang)
        .map_err(|e| e.to_string())?;
    let _ = I18N.set(i18n);
    Ok(())
}
```

The `define_i18n_module!()` macro scans your `.ftl` files at build time and produces the `EmbeddedI18n` type. If a Fluent message references a variable that does not exist, or if the `.ftl` syntax is malformed, you get a compile error. You find out about a broken translation before the app ever runs.

The `localize` function then becomes straightforward:

```rust
pub fn localize(id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>) -> String {
    i18n().localize(id, args)
        .unwrap_or_else(|| id.to_string())
}
```

Pass a message ID and optional arguments. Get back a translated string. If the ID is missing, you get the ID itself as a fallback, which makes debugging easier than getting an empty string.

Using it in a view is a single function call:

```rust
fn render_header(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
    div()
        .child(crate::i18n::localize("page-home", None))
        .child(crate::i18n::localize("page-settings", None))
}
```

No string formatting, no match statements on locale codes, no manual plural logic scattered through your views.

## How runtime language switching works

gpui-starter stores the current locale in a GPUI global called `LocaleState`. When the user picks a new language from the menu or command launcher, the `set_locale` function runs:

```rust
pub fn set_locale(locale: &str, cx: &mut App) {
    rust_i18n::set_locale(locale);
    let _ = crate::i18n::i18n().select_language(
        locale.parse()
            .unwrap_or_else(|_| es_fluent::unic_langid::langid!("en")),
    );
    cx.set_global::<LocaleState>(
        LocaleState(SharedString::from(locale.to_string()))
    );
    crate::app_state::update_config(cx, |config| {
        config.locale = locale.to_string();
    });
    cx.refresh_windows();
}
```

Three things happen here. First, `select_language` tells the `EmbeddedI18n` instance to start resolving messages from the new locale's `.ftl` file. Second, the locale is persisted to the app config so it survives restarts. Third, `cx.refresh_windows()` tells GPUI to re-render every open window.

Because every view calls `localize` on each render, the re-render pulls fresh strings from the newly selected locale. The switch takes effect on the very next frame. There is no cache to invalidate and no state to migrate. The system is pull-based: views ask for text when they draw, and the i18n layer returns whatever the current locale says.

## Adding a new language

To add support for a new language:

1. Create a new directory under `i18n/` named with the locale code (for example `fr-FR`).
2. Copy `en/gpui-starter.ftl` into it and translate every message.
3. Register the locale in `src/app.rs` by adding a variant to the `Languages` enum annotated with `#[es_fluent_language]`.
4. Add a menu item in `src/menus.rs` that dispatches the `SelectLocale` action with the new locale code.
5. Run `cargo build`. If the `.ftl` file has syntax errors, the compiler will tell you.

See the [i18n documentation](/docs/i18n/) for the complete reference.
