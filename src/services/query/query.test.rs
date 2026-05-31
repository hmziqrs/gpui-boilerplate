use super::*;

fn resource() -> QueryResource<&'static str> {
    QueryResource::new(
        "demo",
        CachePolicy::Ttl { ttl_ms: 1_000 },
        RequestPolicy::LatestWins,
    )
}

#[test]
fn request_sequencer_allocates_monotonic_request_ids() {
    let mut sequencer = RequestSequencer::new();

    assert_eq!(sequencer.next_request().value(), 1);
    assert_eq!(sequencer.next_request().value(), 2);
}

#[test]
fn fresh_ttl_resource_can_short_circuit_cache() {
    let mut resource = resource();
    resource.apply_success("cached", 1_000);

    assert!(resource.should_short_circuit_cache(1_500));
}

#[test]
fn stale_ttl_resource_cannot_short_circuit_cache() {
    let mut resource = resource();
    resource.apply_success("cached", 1_000);

    assert!(!resource.should_short_circuit_cache(2_001));
}

#[test]
fn stale_while_revalidate_resource_stays_fresh_but_does_not_short_circuit() {
    let mut resource = QueryResource::new(
        "demo",
        CachePolicy::StaleWhileRevalidate { ttl_ms: 1_000 },
        RequestPolicy::LatestWins,
    );
    resource.apply_success("cached", 1_000);

    assert!(resource.is_cache_fresh(1_500));
    assert!(!resource.should_short_circuit_cache(1_500));
}

#[test]
fn begin_loading_uses_empty_state_without_cached_data() {
    let mut resource = resource();

    let status = resource.begin_loading(RequestId::new(7), 100);

    assert_eq!(status, QueryStatus::LoadingEmpty);
    assert_eq!(resource.active_request_id, Some(RequestId::new(7)));
}

#[test]
fn begin_loading_uses_with_data_state_when_cache_exists() {
    let mut resource = resource();
    resource.apply_success("cached", 50);

    let status = resource.begin_loading(RequestId::new(8), 100);

    assert_eq!(status, QueryStatus::LoadingWithData);
    assert_eq!(resource.data, Some("cached"));
}

#[test]
fn cancellation_is_logical_and_clears_active_request() {
    let mut resource = resource();
    resource.begin_loading(RequestId::new(1), 100);

    assert!(resource.cancel("cancelled"));
    assert_eq!(resource.status, QueryStatus::Cancelled);
    assert_eq!(resource.cancelled_count, 1);
    assert_eq!(resource.active_request_id, None);
}

#[test]
fn clear_current_request_rejects_stale_request_id() {
    let mut resource = resource();
    resource.begin_loading(RequestId::new(2), 100);

    assert!(!resource.clear_current_request(RequestId::new(1)));
    assert_eq!(resource.active_request_id, Some(RequestId::new(2)));
}

#[test]
fn failure_preserves_previous_data() {
    let mut resource = resource();
    resource.apply_success("previous", 100);
    resource.begin_loading(RequestId::new(2), 200);

    resource.apply_failure("failed", 300);

    assert_eq!(resource.status, QueryStatus::Failure);
    assert_eq!(resource.data, Some("previous"));
    assert_eq!(resource.error.as_deref(), Some("failed"));
}

#[test]
fn reset_keeps_key_and_policies_but_clears_runtime_state() {
    let mut resource = resource();
    resource.apply_success("previous", 100);
    resource.record_cache_hit();
    resource.mark_ignored_result();

    resource.reset();

    assert_eq!(resource.key.as_str(), "demo");
    assert_eq!(resource.status, QueryStatus::Idle);
    assert_eq!(resource.data, None);
    assert_eq!(resource.cache_hits, 0);
    assert_eq!(resource.ignored_results, 0);
}
