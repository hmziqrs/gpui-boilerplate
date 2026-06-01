use crate::core::*;
use crate::test_support::{error_message, resource};

// ── placeholder_data accessors ────────────────────────────────────────────

#[test]
fn placeholder_data_is_accessible() {
    let mut resource = resource();
    assert_eq!(resource.placeholder_data(), None);

    resource.set_placeholder_data(Some("placeholder"));
    assert_eq!(resource.placeholder_data(), Some(&"placeholder"));

    resource.set_placeholder_data(None);
    assert_eq!(resource.placeholder_data(), None);
}

#[test]
fn display_data_returns_data_over_placeholder() {
    let mut resource = resource();
    resource.set_placeholder_data(Some("placeholder"));
    resource.apply_success("actual", 100);

    assert_eq!(resource.display_data(), Some(&"actual"));
}

#[test]
fn display_data_returns_placeholder_when_no_data() {
    let mut resource = resource();
    resource.set_placeholder_data(Some("placeholder"));

    assert_eq!(resource.data(), None);
    assert_eq!(resource.display_data(), Some(&"placeholder"));
}

#[test]
fn display_data_returns_none_when_neither_set() {
    let resource = resource();
    assert_eq!(resource.display_data(), None);
}

// ── previous_data tracking ────────────────────────────────────────────────

#[test]
fn success_stores_previous_data() {
    let mut resource = resource();
    resource.apply_success("first", 100);

    assert_eq!(resource.previous_data(), None, "no previous before first success");

    resource.apply_success("second", 200);

    assert_eq!(resource.data(), Some(&"second"));
    assert_eq!(resource.previous_data(), Some(&"first"));
}

#[test]
fn success_optional_stores_previous_data() {
    let mut resource = resource();
    resource.apply_success("first", 100);

    resource.apply_success_optional(Some("second"), 200);

    assert_eq!(resource.data(), Some(&"second"));
    assert_eq!(resource.previous_data(), Some(&"first"));
}

#[test]
fn success_optional_none_stores_previous_data() {
    let mut resource = resource();
    resource.apply_success("first", 100);

    resource.apply_success_optional(None, 200);

    assert_eq!(resource.data(), None);
    assert_eq!(resource.previous_data(), Some(&"first"));
}

#[test]
fn previous_data_is_none_on_first_success() {
    let mut resource = resource();
    resource.apply_success("initial", 100);

    assert_eq!(resource.previous_data(), None);
}

// ── rollback_to_previous ──────────────────────────────────────────────────

#[test]
fn rollback_restores_previous_data() {
    let mut resource = resource();
    resource.apply_success("original", 100);
    resource.apply_success("updated", 200);

    assert_eq!(resource.data(), Some(&"updated"));
    assert_eq!(resource.previous_data(), Some(&"original"));

    let rolled_back = resource.rollback_to_previous();

    assert!(rolled_back);
    assert_eq!(resource.data(), Some(&"original"));
    assert_eq!(resource.status(), QueryStatus::Success);
    assert_eq!(resource.previous_data(), None, "previous_data cleared after rollback");
}

#[test]
fn rollback_returns_false_when_no_previous() {
    let mut resource = resource();
    resource.apply_success("only", 100);

    assert_eq!(resource.previous_data(), None);

    let rolled_back = resource.rollback_to_previous();

    assert!(!rolled_back);
    assert_eq!(resource.data(), Some(&"only"));
}

// ── reset clears placeholder and previous ─────────────────────────────────

#[test]
fn reset_clears_placeholder_and_previous() {
    let mut resource = resource();
    resource.apply_success("first", 100);
    resource.apply_success("second", 200);
    resource.set_placeholder_data(Some("placeholder"));

    assert_eq!(resource.previous_data(), Some(&"first"));
    assert_eq!(resource.placeholder_data(), Some(&"placeholder"));

    resource.reset();

    assert_eq!(resource.placeholder_data(), None);
    assert_eq!(resource.previous_data(), None);
    assert_eq!(resource.data(), None);
    assert_eq!(resource.status(), QueryStatus::Idle);
}

// ── placeholder_data lifecycle ────────────────────────────────────────────

#[test]
fn placeholder_data_survives_loading() {
    let mut resource = resource();
    resource.set_placeholder_data(Some("placeholder"));

    resource.begin_loading(RequestId::scoped(1, 1), 100);

    assert_eq!(resource.status(), QueryStatus::LoadingEmpty);
    assert_eq!(resource.data(), None);
    assert_eq!(resource.display_data(), Some(&"placeholder"));
}

#[test]
fn placeholder_data_ignored_after_success() {
    let mut resource = resource();
    resource.set_placeholder_data(Some("placeholder"));
    resource.begin_loading(RequestId::scoped(1, 1), 100);

    resource.apply_success("real_data", 200);

    assert_eq!(resource.display_data(), Some(&"real_data"));
    assert_eq!(resource.placeholder_data(), Some(&"placeholder"));
}

#[test]
fn failure_with_data_does_not_overwrite_previous_data() {
    // apply_failure_with_data sets data directly without storing previous
    let mut resource = resource();
    resource.apply_success("original", 100);
    // After this, previous_data = None (no prior data field)
    assert_eq!(resource.previous_data(), None);

    resource.apply_failure_with_data("error_data", "something went wrong");

    assert_eq!(resource.status(), QueryStatus::Failure);
    assert_eq!(resource.data(), Some(&"error_data"));
    // apply_failure_with_data does NOT update previous_data
    assert_eq!(resource.previous_data(), None);
}

#[test]
fn failure_does_not_overwrite_previous_data() {
    let mut resource = resource();
    resource.apply_success("v1", 100);
    resource.apply_success("v2", 200);
    // previous_data = Some("v1")

    resource.apply_failure("oops");

    assert_eq!(resource.data(), Some(&"v2"), "failure preserves current data");
    assert_eq!(resource.previous_data(), Some(&"v1"), "failure does not touch previous_data");
    assert_eq!(error_message(&resource), Some("oops"));
}

// ── set_data / clear_data (optimistic updates) ──────────────────────────────

#[test]
fn set_data_stores_previous() {
    let mut resource = resource();
    resource.apply_success("original", 100);

    resource.set_data("optimistic");

    assert_eq!(resource.data(), Some(&"optimistic"));
    assert_eq!(
        resource.previous_data(),
        Some(&"original"),
        "set_data should store old data in previous_data"
    );
}

#[test]
fn set_data_replaces_current() {
    let mut resource = resource();
    resource.apply_success("old", 100);

    resource.set_data("new");

    assert_eq!(resource.data(), Some(&"new"));
}

#[test]
fn set_data_on_empty_resource() {
    let mut resource = resource();
    assert_eq!(resource.data(), None);

    resource.set_data("first");

    assert_eq!(resource.data(), Some(&"first"));
    assert_eq!(
        resource.previous_data(),
        None,
        "previous_data should be None when there was no prior data"
    );
}

#[test]
fn clear_data_stores_previous() {
    let mut resource = resource();
    resource.apply_success("existing", 100);

    resource.clear_data();

    assert_eq!(resource.data(), None);
    assert_eq!(
        resource.previous_data(),
        Some(&"existing"),
        "clear_data should store old data in previous_data"
    );
}

#[test]
fn clear_data_on_empty_is_noop() {
    let mut resource = resource();
    assert_eq!(resource.data(), None);

    resource.clear_data();

    assert_eq!(resource.data(), None);
    assert_eq!(resource.previous_data(), None);
}

#[test]
fn rollback_after_set_data() {
    let mut resource = resource();
    resource.apply_success("original", 100);
    resource.set_data("optimistic");

    let rolled_back = resource.rollback_to_previous();

    assert!(rolled_back);
    assert_eq!(resource.data(), Some(&"original"));
    assert_eq!(resource.status(), QueryStatus::Success);
    assert_eq!(resource.previous_data(), None);
}

#[test]
fn rollback_after_clear_data() {
    let mut resource = resource();
    resource.apply_success("original", 100);
    resource.clear_data();

    assert_eq!(resource.data(), None);
    assert_eq!(resource.previous_data(), Some(&"original"));

    let rolled_back = resource.rollback_to_previous();

    assert!(rolled_back);
    assert_eq!(resource.data(), Some(&"original"));
    assert_eq!(resource.status(), QueryStatus::Success);
}

#[test]
fn double_set_data_keeps_original_previous() {
    let mut resource = resource();
    resource.apply_success("original", 100);

    resource.set_data("optimistic_1");
    assert_eq!(resource.previous_data(), Some(&"original"));

    resource.set_data("optimistic_2");
    assert_eq!(
        resource.previous_data(),
        Some(&"optimistic_1"),
        "second set_data should overwrite previous_data with the first optimistic value"
    );
    assert_eq!(resource.data(), Some(&"optimistic_2"));
}

#[test]
fn complete_success_after_optimistic_update() {
    let mut resource = resource();
    resource.apply_success("original", 100);
    resource.set_data("optimistic");
    assert_eq!(resource.data(), Some(&"optimistic"));

    // Now the real request completes with the true data
    resource.apply_success("real_data", 200);

    assert_eq!(resource.data(), Some(&"real_data"));
    assert_eq!(
        resource.previous_data(),
        Some(&"optimistic"),
        "apply_success should store the optimistic value in previous_data"
    );
}

#[test]
fn complete_failure_after_optimistic_rollback() {
    let mut resource = resource();
    resource.apply_success("original", 100);
    resource.set_data("optimistic");

    // The mutation fails — the request was never started for real,
    // so we just apply failure and rollback
    resource.apply_failure("mutation failed");

    assert_eq!(resource.status(), QueryStatus::Failure);
    assert_eq!(resource.data(), Some(&"optimistic"), "failure preserves current data");
    assert_eq!(resource.previous_data(), Some(&"original"), "failure does not touch previous_data");

    // Rollback to original
    let rolled_back = resource.rollback_to_previous();

    assert!(rolled_back);
    assert_eq!(resource.data(), Some(&"original"));
    assert_eq!(resource.status(), QueryStatus::Success);
}

#[test]
fn set_data_does_not_change_status() {
    let mut resource = resource();
    resource.apply_success("original", 100);
    assert_eq!(resource.status(), QueryStatus::Success);

    resource.set_data("optimistic");
    assert_eq!(
        resource.status(),
        QueryStatus::Success,
        "set_data should not change status"
    );

    // Also test while loading
    resource.begin_loading(RequestId::scoped(1, 1), 200);
    assert_eq!(resource.status(), QueryStatus::LoadingWithData);

    resource.set_data("optimistic_2");
    assert_eq!(
        resource.status(),
        QueryStatus::LoadingWithData,
        "set_data should not change loading status"
    );
}
