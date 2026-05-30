#![allow(dead_code)]

use gpui::{App, Entity, TestAppContext, VisualTestContext, Window, WindowOptions};

// ---------------------------------------------------------------------------
// Window / view helpers
// ---------------------------------------------------------------------------

/// Open a minimal GPUI window and return a [`VisualTestContext`] bound to it.
///
/// This is the recommended entry-point for any test that needs to render a
/// component or dispatch actions through a focus handle.
///
/// # Example
///
/// ```ignore
/// #[gpui::test]
/// fn test_my_view(cx: &mut TestAppContext) {
///     let mut vx = support::rendering::open_visual_context(cx);
///     // ... use vx ...
/// }
/// ```
pub fn open_visual_context(cx: &mut TestAppContext) -> VisualTestContext {
    let window = cx.update(|cx| {
        cx.open_window(WindowOptions::default(), |_, _cx| {
            // A minimal div is enough for most tests; the caller can swap in
            // their own root view if needed.
            gpui::div().id("test-root")
        })
        .expect("failed to open test window")
    });

    VisualTestContext::from_window(window.into(), cx)
}

/// Open a window whose root is the given entity.
///
/// Returns the [`VisualTestContext`] ready for assertions.
pub fn open_window_with_root<T: 'static>(
    cx: &mut TestAppContext,
    build_root: impl FnOnce(&mut Window, &mut gpui::Context<T>) -> T,
) -> (Entity<T>, VisualTestContext) {
    let entity = cx.update(|cx| {
        cx.open_window(WindowOptions::default(), |window, cx| {
            cx.new(|cx| build_root(window, cx))
        })
        .expect("failed to open test window with root")
    });

    let visual = VisualTestContext::from_window(entity.into(), cx);
    (entity, visual)
}

// ---------------------------------------------------------------------------
// Rendering assertions
// ---------------------------------------------------------------------------

/// Assert that a window's root entity satisfies a predicate after all pending
/// work has completed.
///
/// Call [`cx.run_until_parked()`] internally so detached tasks settle before
/// the assertion runs.
pub fn assert_after_settle<T>(
    entity: &Entity<T>,
    cx: &mut TestAppContext,
    predicate: impl FnOnce(&T, &App) -> bool,
    msg: &str,
) {
    cx.run_until_parked();
    let ok = entity.read_with(cx, |t, app| predicate(t, app));
    assert!(ok, "{msg}");
}

// ---------------------------------------------------------------------------
// Testing pattern documentation
// ---------------------------------------------------------------------------

/// GPUI Testing Patterns for the Boilerplate
/// ==========================================
///
/// This module provides lightweight helpers for testing GPUI views.  The
/// recommended workflow is:
///
/// 1. **Unit / logic tests** -- no GPUI context needed.
///    Use plain `#[test]` functions.  See `src/testing.rs` for fake
///    implementations of telemetry, connectivity, and storage.
///
/// 2. **Entity / state tests** -- use `#[gpui::test]` with
///    [`TestAppContext`].  Create entities, update them, and assert on
///    their state:
///
///    ```ignore
///    #[gpui::test]
///    fn test_entity(cx: &mut TestAppContext) {
///        let entity = cx.new(|_cx| MyState::default());
///        entity.update(cx, |s, _cx| s.value = 42);
///        assert_eq!(
///            entity.read_with(cx, |s, _| s.value),
///            42,
///        );
///    }
///    ```
///
/// 3. **View / rendering tests** -- use [`open_visual_context`] or
///    [`open_window_with_root`] to get a [`VisualTestContext`], then
///    dispatch actions or read rendered state:
///
///    ```ignore
///    #[gpui::test]
///    fn test_view(cx: &mut TestAppContext) {
///        let (entity, mut vx) = support::rendering::open_window_with_root(
///            cx,
///            |window, cx| MyView::new(window, cx),
///        );
///        // Dispatch an action, read state, etc.
///    }
///    ```
///
/// 4. **Async tests** -- use `#[gpui::test] async fn` and
///    `cx.run_until_parked()` to settle detached tasks before asserting.
///
/// Feature flag: add to `Cargo.toml` if you need GPUI's internal
/// test-support features:
///
/// ```toml
/// [features]
/// test-support = ["gpui/test-support"]
/// ```
#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn open_visual_context_creates_window(cx: &mut TestAppContext) {
        let _visual = open_visual_context(cx);
        // If we got here without panicking, the window was created.
    }
}
