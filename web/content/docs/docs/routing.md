---
title: "Routing and navigation"
description: "Multi-page navigation using Rust enums, sidebar, and deep link parsing"
---

## Page enum

The sidebar is driven by the `Page` enum in `src/sidebar.rs`. Each variant corresponds to a top-level view in the app.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Page {
    Home,
    Form,
    Settings,
    Notifications,
    Diagnostics,
    About,
}
```

`Page` provides three methods:

| Method | Return type | Purpose |
|--------|-------------|---------|
| `title(&self)` | `&'static str` | Human-readable label for sidebar and header |
| `icon(&self)` | `IconName` | Icon rendered next to the sidebar entry |
| `all()` | `&'static [Page]` | Ordered list used to build the sidebar menu |

## AppRoute

`src/routes.rs` defines `AppRoute`, a two-level routing type. It wraps `Page` for standard navigation and adds sub-routes for specific destinations within a page.

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppRoute {
    Page(Page),
    SettingsNotifications,
}
```

`AppRoute::page_for_render()` maps every variant to a `Page` that the render function can match on. `SettingsNotifications` maps to `Page::Settings`, since it is a detail view inside the settings page.

| Method | Return type | Purpose |
|--------|-------------|---------|
| `page(page)` | `Self` | Constructor for standard pages |
| `page_for_render(&self)` | `Page` | Collapses sub-routes to their parent page |
| `title(&self)` | `&'static str` | Header title for the current route |
| `to_url(&self)` | `String` | Serializes to a deep link URL |
| `parse_deep_link(input)` | `Result<Self, AppError>` | Parses a deep link string into a route |

The default route is `AppRoute::Page(Page::Home)`.

## Render dispatch

`AppRoot` in `src/root.rs` stores an `active_route: AppRoute` field. The `active_page_view` method pattern-matches on the resolved `Page` and returns the corresponding entity as `AnyView`:

```rust
fn active_page_view(&self) -> AnyView {
    match self.active_route.page_for_render() {
        Page::Home => self.home_page.clone().into(),
        Page::Form => self.form_page.clone().into(),
        Page::Settings => self.settings_page.clone().into(),
        Page::Notifications => self.notifications_page.clone().into(),
        Page::Diagnostics => self.diagnostics_page.clone().into(),
        Page::About => self.about_page.clone().into(),
    }
}
```

## Changing pages

`set_route` updates `active_route`, persists the new route to the config file, and calls `cx.notify()` to trigger a re-render:

```rust
fn set_route(&mut self, route: AppRoute, cx: &mut Context<Self>) {
    if self.active_route == route {
        return;
    }
    self.active_route = route.clone();
    crate::app_state::update_config(cx, |config| {
        config.active_route = route;
    });
    cx.notify();
}
```

Because the active route is stored in `AppConfig`, the last viewed page is restored on the next launch.

Sidebar items call `set_route` on click:

```rust
SidebarMenuItem::new(page.title())
    .icon(Icon::new(page.icon()).small())
    .active(active_page == *page)
    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
        this.set_route(AppRoute::page(*page), cx);
    }))
```

## Deep link handling

The app registers the `gpui-starter://` URL scheme. When the OS opens a URL with this scheme, the following happens:

1. `single_instance::preflight()` inspects CLI args for any argument starting with `gpui-starter://`.
2. If the app is already running, the deep link is forwarded to the primary instance via IPC (local socket) or a queue file fallback.
3. The primary instance emits `AppEventKind::DeepLinkReceived(link)` into the global `AppEventQueue`.
4. `AppRoot` observes the event queue and calls `AppRoute::parse_deep_link` on the URL:

```rust
cx.observe_global::<events::AppEventQueue>(|this, cx| {
    for event in events::drain(cx) {
        match event.kind {
            AppEventKind::Navigate(route) => this.set_route(route, cx),
            AppEventKind::DeepLinkReceived(link) => {
                match AppRoute::parse_deep_link(&link) {
                    Ok(route) => this.set_route(route, cx),
                    Err(err) => events::emit_error(err, cx),
                }
            }
            // ...
        }
    }
})
.detach();
```

`parse_deep_link` validates the scheme and maps the URL host and path segments to route variants:

| URL | Route |
|-----|-------|
| `gpui-starter://home` | `Page(Home)` |
| `gpui-starter://form` | `Page(Form)` |
| `gpui-starter://settings` | `Page(Settings)` |
| `gpui-starter://settings/notifications` | `SettingsNotifications` |
| `gpui-starter://notifications` | `Page(Notifications)` |
| `gpui-starter://diagnostics` | `Page(Diagnostics)` |
| `gpui-starter://about` | `Page(About)` |

URLs with an unsupported scheme or unknown host return `AppError::InvalidDeepLink`.

## Adding a new page

Adding a route requires changes in three files.

**1. Add a `Page` variant** in `src/sidebar.rs`:

```rust
pub enum Page {
    Home,
    Form,
    Settings,
    Notifications,
    Diagnostics,
    About,
    Profile, // new
}
```

Update the `title`, `icon`, and `all` methods to include the variant.

**2. Add an `AppRoute` mapping** in `src/routes.rs` (if you need a deep link):

```rust
// In to_url:
Self::Page(Page::Profile) => "gpui-starter://profile".to_string(),

// In parse_deep_link:
("profile", []) => Ok(Self::Page(Page::Profile)),
```

**3. Add a render branch** in `src/root.rs`:

Create an entity field on `AppRoot`, initialize it in `new`, and add a match arm in `active_page_view`:

```rust
Page::Profile => self.profile_page.clone().into(),
```

The sidebar menu is built from `Page::all()`, so the new entry appears automatically once the variant is added.
