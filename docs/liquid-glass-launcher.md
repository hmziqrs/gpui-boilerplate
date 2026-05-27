# Liquid Glass in the Launcher (macOS 26+)

How we got the ⌘K launcher rendering with the macOS 26 Liquid Glass material
(`NSGlassEffectView`), starting from a stock GPUI window that had no Liquid
Glass support.

This is a working session log, not a clean API design. The patches live in a
**cargo git checkout** and will be wiped by any `cargo update` — see
[Volatility](#volatility-this-is-not-shippable-yet) before depending on this.

---

## TL;DR

| Layer | What we changed |
|---|---|
| `gpui_macos` (patched in cargo checkout) | Swap `NSVisualEffectView` for `NSGlassEffectView` when `WindowBackgroundAppearance::Blurred` is set on macOS 26+; disable the native window shadow; use Clear style with nil tint |
| `src/launcher.rs` | Set `window_background: Blurred`; clear Root's bg with `transparent_black()`; drop the launcher panel's tint; clip rounded corners with `overflow_hidden`; soften remaining internal opaque elements |

---

## Background: what's actually available

GPUI exposes a single window-level blur knob:

```rust
pub enum WindowBackgroundAppearance {
    Opaque,        // default
    Transparent,
    Blurred,       // NSVisualEffectView on macOS (the old frosted blur)
    MicaBackdrop,
    MicaAltBackdrop,
}
```

On macOS, `Blurred` is implemented in `gpui_macos/src/window.rs` by allocating
a custom `NSVisualEffectView` subclass (`BLURRED_VIEW_CLASS`) and inserting it
below the GPUI metal view inside the window's content view. The material used
is `NSVisualEffectMaterial::Selection` — the pre-Tahoe frosted blur.

**There is no `WindowBackgroundAppearance::LiquidGlass` variant.** macOS 26
ships `NSGlassEffectView` in AppKit, but GPUI doesn't call it.

`NSGlassEffectView` header (`<AppKit/NSGlassEffectView.h>`):

```objc
@interface NSGlassEffectView: NSView
@property (nullable, strong) __kindof NSView *contentView;
@property CGFloat cornerRadius;
@property (nullable, copy) NSColor *tintColor;
@property NSGlassEffectViewStyle style;   // 0 = Regular, 1 = Clear
@end
```

Behaviour to know up front:

- The glass effect renders **around** `contentView`. With `contentView = nil`,
  the view renders nothing visible.
- `Regular` style auto-tints to the system NSAppearance (dark glass in dark
  mode, light glass in light mode).
- `Clear` style is thinner and far less colour-biased — closer to "raw glass".

---

## Step 1 — Add window-level Blur to the launcher

The launcher creates a borderless `PopUp` window via `cx.open_window(...)`.
We added the `Blurred` appearance to its `WindowOptions`:

```rust
// src/launcher.rs (inside open_launcher())
let options = WindowOptions {
    window_bounds: Some(WindowBounds::Windowed(bounds)),
    titlebar: None,
    focus: true,
    show: true,
    kind: WindowKind::PopUp,
    is_movable: true,
    is_resizable: false,
    window_background: WindowBackgroundAppearance::Blurred,  // <-- added
    window_min_size: Some(gpui::Size { width: window_w, height: window_h }),
    ..Default::default()
};
```

This is the trigger: it causes `MacWindow::set_background_appearance(Blurred)`
to fire during window setup, which is the only code path that creates the
blur/glass view.

---

## Step 2 — Stop GPUI's Root from painting the theme background

`gpui-component`'s `Root::render` does this (see
`vendor/gpui-component/crates/ui/src/root.rs:504`):

```rust
window_border().shadow_size(self.window_shadow_size).child(
    div()
        .id("root")
        .relative()
        .size_full()
        .bg(cx.theme().background)         // <-- opaque theme color
        .text_color(cx.theme().foreground)
        .refine_style(&self.style)         // <-- our override goes here
        .child(self.view.clone())
        .child(self.tooltip_overlay.clone()),
)
```

Without intervention, this paints `cx.theme().background` opaquely over the
NSVisualEffectView/NSGlassEffectView, so you never see any glass — you see the
theme colour.

`Root` implements `Styled`, and `.refine_style(&self.style)` runs **after**
`.bg(theme.background)`. So chaining `.bg(transparent_black())` on the `Root`
entity wins via the refinement merge:

```rust
let window = cx
    .open_window(options, |window, cx| {
        let launcher_root = cx.new(|cx| LauncherRoot::new(window, cx));
        cx.new(|cx| Root::new(launcher_root, window, cx).bg(transparent_black()))  // <-- here
    })
    .expect("failed to open launcher window");
```

`gpui::transparent_black()` is `Hsla(0, 0, 0, 0)` — fully clear. After this,
the GPUI metal layer is genuinely transparent and the view *behind* it (the
glass view we'll wire up next) becomes visible.

---

## Step 3 — Patch `gpui_macos` to use NSGlassEffectView

This is the load-bearing change. We edit GPUI's macOS window code directly
in the cargo git checkout — see [Volatility](#volatility-this-is-not-shippable-yet).

Find the active checkout:

```bash
grep "git+https://github.com/zed-industries/zed" Cargo.lock | sort -u
```

For our build the active path was:

```
~/.cargo/git/checkouts/zed-a70e2ad075855582/c551ec9/crates/gpui_macos/src/window.rs
```

(Note: cargo can pick a different short-hash checkout than what's pinned in
`Cargo.lock`. Always confirm with `cargo build -v 2>&1 | grep gpui_macos` to
see the hash it actually compiles from. We hit this once — patched the wrong
checkout, build was silent, no glass.)

Inside `set_background_appearance`, the existing macOS 12+ branch creates the
blur view and adds it below the metal view. We replaced that path:

```rust
} else if this.blurred_view.is_none() {
    // LIQUID GLASS EXPERIMENT: kill window shadow so the glass
    // edges aren't framed by a heavy system drop shadow.
    let _: () = msg_send![this.native_window, setHasShadow: NO];

    let content_view = this.native_window.contentView();
    let frame = NSView::bounds(content_view);

    // Look up NSGlassEffectView at runtime — falls back to the old blur
    // view if the class isn't present (e.g. macOS < 26).
    let glass_cls = Class::get("NSGlassEffectView");
    let mut blur_view: id = if let Some(cls) = glass_cls {
        let v: id = msg_send![cls, alloc];
        let v: id = msg_send![v, initWithFrame: frame];
        let _: () = msg_send![v, setCornerRadius: 16f64];
        // Style 0 = Regular (auto-tints to system appearance),
        // 1 = Clear (thinner, minimal vibrancy — closer to raw glass).
        let _: () = msg_send![v, setStyle: 1i64];
        // Explicit nil tintColor: prevents AppKit from inheriting any
        // ambient tint from the window/appearance.
        let _: () = msg_send![v, setTintColor: nil];

        // NSGlassEffectView needs a contentView for the glass to render.
        // An empty layer-backed NSView is enough; the glass effect draws
        // around it within the view's bounds.
        let content: id = msg_send![class!(NSView), alloc];
        let content: id = msg_send![content, initWithFrame: frame];
        let _: () = msg_send![content, setWantsLayer: YES];
        let _: () = msg_send![content, setAutoresizingMask:
            NSViewWidthSizable | NSViewHeightSizable];
        let _: () = msg_send![v, setContentView: content];
        v
    } else {
        let v: id = msg_send![BLURRED_VIEW_CLASS, alloc];
        NSView::initWithFrame_(v, frame)
    };
    blur_view.setAutoresizingMask_(NSViewWidthSizable | NSViewHeightSizable);

    let _: () = msg_send![
        content_view,
        addSubview: blur_view
        positioned: NSWindowOrderingMode::NSWindowBelow
        relativeTo: nil
    ];
    this.blurred_view = Some(blur_view.autorelease());
}
```

### Why each piece

| Thing | Why |
|---|---|
| `Class::get("NSGlassEffectView")` | Runtime lookup — the class only exists on macOS 26+. Falls back to the old `NSVisualEffectView` blur on older OSes. |
| `setCornerRadius: 16` | Matches the launcher panel's rounded look. The glass clips to this radius. |
| `setStyle: 1` (Clear) | Regular style was reading as "dark on dark, light on light", which the user perceived as a theme-based opaque background. Clear is much closer to raw glass with minimal tint. |
| `setTintColor: nil` | Belt-and-braces against ambient appearance bleed into the glass. |
| Layer-backed content view | The fix for "view created but invisible". `NSGlassEffectView` only renders glass when it has a `contentView` — an empty `NSView` with `wantsLayer = YES` is enough. |
| `setHasShadow: NO` | GPUI deliberately preserves the system window shadow (gpui_macos sets background alpha to `0.0001` specifically to keep it). For glass, that heavy drop shadow framed the panel and read as a hard edge over the soft glass. Killing it lets the glass be the only edge. |

### Discovery: why the first attempts looked like nothing

We landed on this through three failed iterations:

1. **First attempt** — alloc `NSGlassEffectView` and stop. The view was created
   but had no `contentView`, so AppKit rendered nothing visible. We thought
   `Class::get` was returning `None` and fell back to the old blur view.
   Confirmed it was actually being created by adding `eprintln!`:
   ```
   [GLASS] NSGlassEffectView class lookup: true
   [GLASS] Creating NSGlassEffectView frame=(620.0, 461.0)
   ```
   Class lookup succeeded, view was created — but invisible.

2. **Second attempt** — added a plain `NSView` `contentView`. Still nothing
   visible. The contentView needs to be **layer-backed** (`setWantsLayer: YES`)
   for the glass effect to have a render target it can latch onto.

3. **Third attempt** — Clear style + nil tint. Regular style produced visible
   glass, but the auto-tint made it read as "the launcher just has a dark/light
   theme background", defeating the point.

---

## Step 4 — Make the launcher panel itself transparent

With glass in place, anything opaque painted in GPUI on top of the metal
layer reads as "background colour over the glass". We stripped or softened
every opaque internal:

```rust
// src/launcher.rs (inside Launcher::render)
v_flex()
    .size_full()
    .rounded(theme.radius_lg)
    .overflow_hidden()              // clip children to the rounded glass edge
    .key_context(CONTEXT)
    .track_focus(&self.focus_handle)
    .focus_trap("launcher", &self.focus_handle)
    /* ... actions ... */
    .child(
        h_flex()
            .px_4().py(px(12.)).gap_3()
            .border_b_1()
            .border_color(theme.border.opacity(0.3))   // soft divider
            .items_center()
            /* search icon + Input */
    )
    .child(
        v_flex()
            .flex_1()
            .overflow_y_scrollbar()
            .py_1()
            .children(filtered.iter().enumerate().map(|(display_ix, &item_ix)| {
                /* ... */
                h_flex()
                    /* ... */
                    .when(is_selected, |el| el.bg(theme.list_active.opacity(0.6)))
                    .when(!is_selected, |el| el.hover(|el| el.bg(theme.list_hover.opacity(0.4))))
                    .child(
                        div()
                            .flex_shrink_0().size_8()
                            .flex().items_center().justify_center()
                            .rounded(theme.radius)
                            .bg(theme.secondary.opacity(0.4))   // glass through icon tile
                            .child(Icon::new(icon).small()),
                    )
                    /* title + subtitle */
            }))
    )
```

Key moves:

- **No `bg` on the outer panel** — the glass is the surface.
- **`overflow_hidden`** on the rounded outer container — without this, the
  search-bar bottom border and hover bg extended past the rounded glass corners
  and produced visible rectangular "tabs" sticking out.
- **Opacity on internal tints** — divider, selected row, hover row, icon tile
  all reduced to `0.3–0.6` so the glass shows through even on hover/selection.

---

## Volatility: this is not shippable yet

The `gpui_macos` edit lives in `~/.cargo/git/checkouts/...`. **Any of these
revert it:**

- `cargo update`
- `cargo clean` (only `-p gpui_macos` rebuilds; full clean clears the artifact
  cache but the source edit usually survives — `cargo update` is the dangerous one)
- Wiping `~/.cargo/git`
- A teammate cloning the repo for the first time

To make this real, pick one:

1. **Local clone of zed + `[patch]`** — clone `zed-industries/zed` to
   `vendor/zed`, add to root `Cargo.toml`:
   ```toml
   [patch."https://github.com/zed-industries/zed"]
   gpui = { path = "vendor/zed/crates/gpui" }
   gpui_macos = { path = "vendor/zed/crates/gpui_macos" }
   gpui_platform = { path = "vendor/zed/crates/gpui_platform" }
   ```
   Apply the same edits to `vendor/zed/crates/gpui_macos/src/window.rs`.

2. **Fork zed** on GitHub, commit the patch, point the workspace deps at the
   fork. Pin a revision so you control when you absorb upstream changes.

3. **Upstream PR** — add a `WindowBackgroundAppearance::LiquidGlass` variant
   (or a `has_shadow: bool` plus a glass-material constant) so it's a real API
   instead of a hijack of `Blurred`. Slow turnaround but the right answer
   long-term.

The minimal upstream-shaped change: add an enum variant and gate the
`NSGlassEffectView` path on it, instead of hijacking `Blurred`. That way apps
that want the old `NSVisualEffectView` blur on macOS 26 still get it.

---

## Caveats and known limitations

- **macOS 26+ only.** On older systems `Class::get("NSGlassEffectView")`
  returns `None` and we fall back to the existing `BLURRED_VIEW_CLASS` blur
  view. The launcher will still work, just without Liquid Glass.
- **`setHasShadow: NO` is global to the Blurred path.** Any GPUI window that
  opts into `Blurred` loses its system shadow. For this app it's only the
  launcher, but a fork should make this opt-in.
- **No `NSAppearance` override.** Glass material inherits system light/dark.
  If you want the launcher to always look light or always dark regardless of
  system theme, the launcher's `NSWindow` needs `setAppearance:` with
  `NSAppearanceNameAqua` / `NSAppearanceNameDarkAqua`.
- **The `eprintln!` glass-trace lines** in `set_background_appearance` are
  debug instrumentation. Remove them (or convert to `tracing::debug!`) before
  upstreaming.
- **Style 1 (Clear) was a deliberate trade.** It gives the cleanest glass look
  but the legibility floor is lower — on a busy desktop background, content
  contrast can suffer. Style 0 (Regular) reads as "themed" but is safer for
  text-heavy UI.

---

## Tuning dead-ends (so you don't repeat them)

Things we tried after the initial "Cool this works" state, all of which made
the glass worse:

- **Style 0 (Regular) without a tint.** In dark system appearance the glass
  picks up heavy dark vibrancy and reads as a near-opaque dark themed panel.
  Not glass-y, just dark.
- **Any `tintColor` value.** Red, white at 0.15, white at any alpha — all of
  them turn the glass into a flat coloured surface that defeats the point.
  The Clear material is calibrated to disappear; tinting it removes that
  calibration. Keep `tintColor` nil.
- **`set_contentLensing: 2`** (private SPI). Name suggested it would push
  refraction into the centre; in practice no visible difference at either
  Clear or Regular style. Either the property does something else or its
  value range is different than guessed.
- **`setAppearance: NSAppearanceNameAqua` on the NSWindow from inside
  `set_background_appearance`.** Hangs the app — likely a reentry loop
  through `viewDidChangeEffectiveAppearance`. If you want a forced
  appearance, call `setAppearance:` from the launcher's `open_launcher`
  *after* `open_window` returns, not from inside the platform layer's
  window-creation path.

Confirmed-good final config (what's in the patch):

```rust
let _: () = msg_send![v, setCornerRadius: 16f64];
let _: () = msg_send![v, setStyle: 1i64];   // Clear
let _: () = msg_send![v, setTintColor: nil];
```

Other private SPI worth exploring later (introspected but not tried):
`_variant`, `_subvariant`, `_subduedState`, `_vibrantBlendingStyleForSubtree`,
`_path` (custom CGPath mask — e.g. for "glass only on the perimeter ring").

---

## Files touched

| Path | Purpose |
|---|---|
| `src/launcher.rs` | `Blurred` window background, transparent Root, panel internals softened, `overflow_hidden` |
| `~/.cargo/git/checkouts/zed-a70e2ad075855582/<hash>/crates/gpui_macos/src/window.rs` | `NSGlassEffectView` swap inside `set_background_appearance`, `setHasShadow: NO` |
