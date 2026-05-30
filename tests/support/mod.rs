#![allow(dead_code)]

pub mod rendering;

// ---------------------------------------------------------------------------
// Re-exports
// ---------------------------------------------------------------------------

pub use gpui::{App, Entity, TestAppContext, Window};
pub use rendering::*;

// ---------------------------------------------------------------------------
// Test-app factory
// ---------------------------------------------------------------------------

/// Create a [`TestAppContext`] with the minimum boilerplate set up so that
/// integration tests can exercise app-level code.
///
/// # Example
///
/// ```ignore
/// #[gpui::test]
/// fn my_test(cx: &mut TestAppContext) {
///     let cx = support::create_test_app(cx);
///     // ... interact with cx ...
/// }
/// ```
pub fn create_test_app(cx: &mut TestAppContext) -> &mut TestAppContext {
    // Initialise gpui-component helpers (fonts, default theme, etc.) so that
    // views which depend on `ActiveTheme` or `Root` compile without panicking.
    cx.update(|cx| {
        gpui_component::init(cx);
    });
    cx
}

// ---------------------------------------------------------------------------
// Common assertions
// ---------------------------------------------------------------------------

/// Assert that an [`Entity`] field matches the expected value.
///
/// Shorthand for `entity.read_with(cx, |t, _| t.field)` followed by an
/// `assert_eq!`.
pub fn assert_entity_field<T, V>(
    entity: &Entity<T>,
    cx: &TestAppContext,
    read: impl FnOnce(&T, &App) -> V,
    expected: V,
) where
    V: PartialEq + std::fmt::Debug,
{
    let actual = entity.read_with(cx, |t, cx| read(t, cx));
    assert_eq!(actual, expected, "entity field assertion failed");
}

/// Assert that an [`Entity`] field satisfies the given predicate.
pub fn assert_entity_satisfies<T>(
    entity: &Entity<T>,
    cx: &TestAppContext,
    predicate: impl FnOnce(&T, &App) -> bool,
    msg: &str,
) {
    let ok = entity.read_with(cx, |t, cx| predicate(t, cx));
    assert!(ok, "{msg}");
}
