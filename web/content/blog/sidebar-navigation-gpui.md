---
title: "Sidebar navigation in GPUI: routing without a router"
description: "How gpui-starter implements multi-page navigation using Rust enums and pattern matching instead of a web-style router."
date: 2026-05-07
tags: [GPUI, Rust, desktop]
draft: false
---

Desktop apps don't have URLs. There is no browser history or address bar. When you click a sidebar item, something different has to happen than what a web framework would do.

In gpui-starter, navigation is built around a Rust enum. No router library, no path matching. Types and pattern matching.

## The Page enum

Every screen in the app is a variant of a single enum:

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum Page {
    Home,
    Settings,
    Profile,
    About,
}
```

That is the entire routing table. When the app needs to know "what page are we on," it reads this value. When it needs to switch pages, it writes a new value. The enum lives inside the app's main [entity](/docs/architecture/), which means GPUI's reactivity system handles the rest.

If you add a new screen, you add a variant. The compiler will then tell you every place you forgot to handle it. This is one of the better properties of modeling routes as an enum: exhaustiveness checking.

## Rendering from the active page

The sidebar and the content area share the same entity. When the entity's page field changes, both views re-render. The content area uses a match statement:

```rust
fn render_content(&self, cx: &ViewContext<Self>) -> impl IntoElement {
    let page = self.page.clone();

    div()
        .flex_1()
        .child(match page {
            Page::Home => self.render_home(cx),
            Page::Settings => self.render_settings(cx),
            Page::Profile => self.render_profile(cx),
            Page::About => self.render_about(cx),
        })
}
```

No conditional rendering chain, no template switching, no route resolver. The match block maps each variant to a render function. If you add a `Page::Dashboard` variant and forget to add it here, Rust refuses to compile.

This pattern works because GPUI renders on every frame when an entity is marked dirty. Changing the page value marks the entity as changed, the framework schedules a re-render, and the match block picks the right view.

## The sidebar module

The sidebar in gpui-starter is its own module. It renders a list of items, each bound to a `Page` variant. Clicking an item calls `cx.emit(PageEvent::Navigate(page))` or directly mutates the entity, depending on the architecture you pick.

A simplified version looks like this:

```rust
fn render_sidebar(&self, cx: &ViewContext<Self>) -> impl IntoElement {
    let active = self.page.clone();

    div()
        .w(px(240.0))
        .h_full()
        .bg(cx.theme().sidebar)
        .flex()
        .flex_col()
        .children(
            [
                (Page::Home, "Home", Icon::Home),
                (Page::Settings, "Settings", Icon::Settings),
                (Page::Profile, "Profile", Icon::User),
                (Page::About, "About", Icon::Info),
            ]
            .map(|(page, label, icon)| {
                let is_active = active == page;
                SidebarItem::new(page, label, icon, is_active)
            }),
        )
}
```

The `SidebarItem` component handles hover states, active indicators, and the click handler. When clicked, it calls:

```rust
cx.update(&mut entity, |state, cx| {
    state.page = new_page;
    cx.notify();
});
```

That `cx.notify()` call is what triggers the re-render. GPUI does not diff a virtual DOM. It re-runs the view's render function and draws the result. The match block in `render_content` picks the new variant, and the new view appears.

## How page changes propagate

GPUI uses an entity system. Think of it as a reactive store with type safety. Each entity owns some state and exposes it through `ViewContext`. When you mutate state and call `cx.notify()`, GPUI knows that view needs to re-render on the next frame.

For navigation, the flow is:

1. User clicks a sidebar item.
2. The click handler updates `state.page` to the new variant.
3. `cx.notify()` marks the view as dirty.
4. On the next frame, GPUI calls the view's `render` method.
5. The match block in `render_content` sees the new variant and renders the corresponding view.

No event bus. No prop drilling. The entity is the source of truth, and `cx.notify()` is the signal that something changed.

If you want multiple views to react to page changes, you can subscribe to the entity. Any view that reads `self.page` during render will automatically stay in sync because it re-renders when the entity notifies.

## Deep links: parsing URLs into Page variants

Desktop apps can receive URLs. macOS and Linux both support registering custom URL schemes. When the OS sends your app a URL like `myapp://settings`, you need to turn that string into a `Page` variant.

gpui-starter handles this with a `FromStr` implementation:

```rust
impl FromStr for Page {
    type Err = ParsePageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim_start_matches("myapp://").trim_end_matches('/') {
            "" | "home" => Ok(Page::Home),
            "settings" => Ok(Page::Settings),
            "profile" => Ok(Page::Profile),
            "about" => Ok(Page::About),
            other => Err(ParsePageError::Unknown(other.to_string())),
        }
    }
}
```

When the app receives a deep link, it parses the URL into a `Page`, updates the entity, and calls `cx.notify()`. The same render path handles it. Deep links and sidebar clicks end up in the same place: a new value in the page field.

This also makes testing straightforward. You can test routing by asserting that `Page::from_str("myapp://settings") == Ok(Page::Settings)`. No need to simulate clicks or render a UI.

## Why this works for desktop

Web frameworks need routers because URLs are the primary navigation mechanism. The browser's back button, bookmarks, and shared links all depend on URLs existing.

Desktop apps have different constraints. Navigation is driven by user interaction within the app window. There is no back button in the traditional sense (though you could build one by keeping a `Vec<Page>` history). Bookmarks don't exist. Deep links are a bonus, not the primary path.

An enum plus a match statement covers the common case. You get compile-time guarantees that every page is handled and a single source of truth for what pages exist. Deep link support comes naturally by implementing `FromStr`.

The sidebar module in gpui-starter demonstrates this pattern end to end. Check the [architecture](/docs/architecture/) docs for how entities fit into the larger app structure, or start from [getting started](/docs/getting-started/) to see the full setup.
