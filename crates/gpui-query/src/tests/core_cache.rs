use crate::core::*;
use crate::test_support::resource;

#[test]
fn fresh_ttl_resource_can_short_circuit_cache() {
    let mut resource = resource();
    resource.apply_success("cached", 1_000);

    assert!(resource.should_short_circuit_cache(1_500));
}

#[test]
fn ttl_cache_is_fresh_at_exact_ttl_and_stale_one_ms_later() {
    let mut resource = resource();
    resource.apply_success("cached", 1_000);

    assert!(resource.is_cache_fresh(2_000));
    assert!(!resource.is_cache_fresh(2_001));
}

#[test]
fn cache_timestamp_from_the_future_is_not_fresh() {
    let mut resource = resource();
    resource.apply_success("cached", 10_000);

    assert!(!resource.is_cache_fresh(9_999));
}

#[test]
fn stale_ttl_resource_cannot_short_circuit_cache() {
    let mut resource = resource();
    resource.apply_success("cached", 1_000);

    assert!(!resource.should_short_circuit_cache(2_001));
}

#[test]
fn stale_while_revalidate_resource_stays_fresh_but_does_not_short_circuit() {
    let mut resource: QueryResource<&'static str> = QueryResource::new(
        "demo",
        CachePolicy::StaleWhileRevalidate { ttl_ms: 1_000 },
        RequestPolicy::LatestWins,
    );
    resource.apply_success("cached", 1_000);

    assert!(resource.is_cache_fresh(1_500));
    assert!(!resource.should_short_circuit_cache(1_500));
}

#[test]
fn no_cache_resource_is_never_fresh() {
    let mut resource: QueryResource<&'static str> =
        QueryResource::new("demo", CachePolicy::NoCache, RequestPolicy::LatestWins);
    resource.apply_success("cached", 1_000);

    assert!(!resource.is_cache_fresh(1_000));
    assert!(!resource.should_short_circuit_cache(1_000));
}

#[test]
fn invalidation_expires_data_without_removing_it() {
    let mut resource = resource();
    resource.apply_success("cached", 1_000);

    resource.invalidate();

    assert_eq!(resource.data(), Some(&"cached"));
    assert!(!resource.is_cache_fresh(1_001));
}

#[test]
fn begin_request_short_circuits_fresh_ttl_cache() {
    let mut resource = resource();
    let mut sequencer = RequestSequencer::new();
    resource.apply_success("cached", 1_000);

    let result = resource.begin_request(&mut sequencer, 1_500, QueryFetchMode::Normal);

    assert_eq!(result, QueryBeginResult::CacheHit);
    assert_eq!(resource.cache_hits(), 1);
    assert_eq!(resource.active_request_id(), None);
}

#[test]
fn forced_begin_request_bypasses_fresh_ttl_cache() {
    let mut resource = resource();
    let mut sequencer = RequestSequencer::new();
    resource.apply_success("cached", 1_000);

    let result = resource.begin_request(&mut sequencer, 1_500, QueryFetchMode::Force);

    assert_eq!(
        result,
        QueryBeginResult::Started {
            request_id: RequestId::scoped(1, 1),
            status: QueryStatus::LoadingWithData,
            replaced_request_id: None,
        }
    );
}

#[test]
fn ignore_while_loading_policy_rejects_duplicate_begin_request() {
    let mut resource: QueryResource<&'static str> = QueryResource::new(
        "demo",
        CachePolicy::NoCache,
        RequestPolicy::IgnoreWhileLoading,
    );
    let mut sequencer = RequestSequencer::new();
    let first = resource.begin_request(&mut sequencer, 100, QueryFetchMode::Normal);
    assert!(matches!(first, QueryBeginResult::Started { .. }));

    let duplicate = resource.begin_request(&mut sequencer, 200, QueryFetchMode::Normal);

    assert_eq!(
        duplicate,
        QueryBeginResult::IgnoredWhileLoading {
            active_request_id: RequestId::scoped(1, 1),
        }
    );
}

#[test]
fn latest_wins_begin_request_replaces_active_request() {
    let mut resource = resource();
    let mut sequencer = RequestSequencer::new();
    resource.begin_request(&mut sequencer, 100, QueryFetchMode::Normal);

    let replacement = resource.begin_request(&mut sequencer, 200, QueryFetchMode::Normal);

    assert_eq!(
        replacement,
        QueryBeginResult::Started {
            request_id: RequestId::scoped(1, 2),
            status: QueryStatus::LoadingEmpty,
            replaced_request_id: Some(RequestId::scoped(1, 1)),
        }
    );
    assert_eq!(resource.cancelled_count(), 1);
}
