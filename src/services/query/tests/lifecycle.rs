use super::{test_support::{error_message, resource}, *};

#[test]
fn begin_loading_uses_empty_state_without_cached_data() {
    let mut resource = resource();

    let status = resource.begin_loading(RequestId::scoped(1, 7), 100);

    assert_eq!(status, QueryStatus::LoadingEmpty);
    assert_eq!(resource.active_request_id(), Some(RequestId::scoped(1, 7)));
}

#[test]
fn begin_loading_uses_with_data_state_when_cache_exists() {
    let mut resource = resource();
    resource.apply_success("cached", 50);

    let status = resource.begin_loading(RequestId::scoped(1, 8), 100);

    assert_eq!(status, QueryStatus::LoadingWithData);
    assert_eq!(resource.data(), Some(&"cached"));
}

#[test]
fn cancellation_is_logical_and_clears_active_request() {
    let mut resource = resource();
    resource.begin_loading(RequestId::scoped(1, 1), 100);

    assert!(resource.cancel("cancelled"));
    assert_eq!(resource.status(), QueryStatus::Cancelled);
    assert_eq!(resource.cancelled_count(), 1);
    assert_eq!(resource.active_request_id(), None);
}

#[test]
fn accepting_current_request_rejects_stale_request_id() {
    let mut resource = resource();
    resource.begin_loading(RequestId::scoped(1, 2), 100);

    assert!(
        resource
            .accept_current_request(RequestId::scoped(1, 1))
            .is_none()
    );
    assert_eq!(resource.active_request_id(), Some(RequestId::scoped(1, 2)));
    assert_eq!(resource.ignored_results(), 1);
}

#[test]
fn stale_current_completion_keeps_newer_loading_state() {
    let mut resource = resource();
    resource.begin_loading(RequestId::scoped(1, 1), 100);
    resource.begin_loading(RequestId::scoped(1, 2), 200);

    assert!(!resource.complete_current_success(RequestId::scoped(1, 1), "stale", 300));

    assert_eq!(resource.status(), QueryStatus::LoadingEmpty);
    assert_eq!(resource.data(), None);
    assert_eq!(resource.active_request_id(), Some(RequestId::scoped(1, 2)));
    assert_eq!(resource.ignored_results(), 1);
}

#[test]
fn failure_preserves_previous_data() {
    let mut resource = resource();
    resource.apply_success("previous", 100);
    resource.begin_loading(RequestId::scoped(1, 2), 200);

    resource.apply_failure("failed");

    assert_eq!(resource.status(), QueryStatus::Failure);
    assert_eq!(resource.data(), Some(&"previous"));
    assert_eq!(error_message(&resource), Some("failed"));
    assert_eq!(resource.last_updated_at_ms(), Some(100));
    assert!(!resource.should_short_circuit_cache(1_101));
}

#[test]
fn failure_does_not_renew_stale_ttl_cache() {
    let mut resource = resource();
    resource.apply_success("previous", 1_000);
    resource.begin_loading(RequestId::scoped(1, 2), 2_501);

    resource.apply_failure("failed");

    assert_eq!(resource.data(), Some(&"previous"));
    assert_eq!(resource.last_updated_at_ms(), Some(1_000));
    assert!(!resource.should_short_circuit_cache(2_502));
}

#[test]
fn failure_with_data_replaces_previous_data() {
    let mut resource = resource();
    resource.apply_success("previous", 100);

    resource.apply_failure_with_data("latest failure body", "failed");

    assert_eq!(resource.status(), QueryStatus::Failure);
    assert_eq!(resource.data(), Some(&"latest failure body"));
    assert_eq!(error_message(&resource), Some("failed"));
    assert_eq!(resource.last_updated_at_ms(), Some(100));
    assert!(!resource.should_short_circuit_cache(1_101));
}

#[test]
fn optional_success_can_complete_without_data() {
    let mut resource = resource();
    resource.begin_loading(RequestId::scoped(1, 1), 100);

    resource.apply_success_optional(None, 200);

    assert_eq!(resource.status(), QueryStatus::Success);
    assert_eq!(resource.data(), None);
    assert_eq!(resource.active_request_id(), None);
    assert!(!resource.should_short_circuit_cache(201));
}

#[test]
fn optional_success_none_clears_previous_data() {
    let mut resource = resource();
    resource.apply_success("previous", 100);
    resource.begin_loading(RequestId::scoped(1, 1), 200);

    resource.apply_success_optional(None, 300);

    assert_eq!(resource.status(), QueryStatus::Success);
    assert_eq!(resource.data(), None);
    assert_eq!(resource.last_updated_at_ms(), Some(300));
    assert!(!resource.should_short_circuit_cache(301));
}

#[test]
fn cancel_without_active_request_is_noop() {
    let mut resource = resource();

    assert!(!resource.cancel("cancelled"));
    assert_eq!(resource.status(), QueryStatus::Idle);
    assert_eq!(resource.cancelled_count(), 0);
}

#[test]
fn cancel_during_revalidation_preserves_cached_data_and_timestamp() {
    let mut resource = resource();
    resource.apply_success("cached", 1_000);
    resource.begin_loading(RequestId::scoped(1, 1), 1_500);

    assert!(resource.cancel("cancelled"));

    assert_eq!(resource.status(), QueryStatus::Cancelled);
    assert_eq!(resource.data(), Some(&"cached"));
    assert_eq!(resource.last_updated_at_ms(), Some(1_000));
}

#[test]
fn invalidate_while_loading_does_not_cancel_active_request() {
    let mut resource = resource();
    resource.apply_success("cached", 1_000);
    resource.begin_loading(RequestId::scoped(1, 1), 1_500);

    resource.invalidate();

    assert_eq!(resource.data(), Some(&"cached"));
    assert_eq!(resource.active_request_id(), Some(RequestId::scoped(1, 1)));
    assert_eq!(resource.last_updated_at_ms(), None);
}

#[test]
fn reset_keeps_key_and_policies_but_clears_runtime_state() {
    let mut resource = resource();
    resource.apply_success("previous", 100);
    resource.record_cache_hit();
    resource.mark_ignored_result();
    resource.begin_loading(RequestId::scoped(1, 1), 200);
    resource.cancel("cancelled");

    resource.reset();

    assert_eq!(resource.key().as_str(), "demo");
    assert_eq!(resource.status(), QueryStatus::Idle);
    assert_eq!(resource.data(), None);
    assert_eq!(resource.error(), None);
    assert_eq!(resource.active_request_id(), None);
    assert_eq!(resource.started_at_ms(), None);
    assert_eq!(resource.last_updated_at_ms(), None);
    assert_eq!(resource.cache_hits(), 0);
    assert_eq!(resource.cancelled_count(), 0);
    assert_eq!(resource.ignored_results(), 0);
}
