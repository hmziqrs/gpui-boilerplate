//! Integration tests for the Client layer using GPUI's `TestAppContext`.

use gpui::TestAppContext;

use crate::client::{BucketDefaults, QueryBucket, QueryBucketTrait, QueryClient};
use crate::core::*;

// ── Test fixtures ──────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
struct User {
    id: u32,
    name: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Post {
    id: u32,
    title: String,
}

fn default_user() -> User {
    User {
        id: 1,
        name: "Alice".into(),
    }
}

fn default_post() -> Post {
    Post {
        id: 1,
        title: "Hello World".into(),
    }
}

// ── QueryBucket tests ──────────────────────────────────────────────────

#[gpui::test]
fn bucket_creates_and_deduplicates_resources(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut bucket: QueryBucket<User> = QueryBucket::new(BucketDefaults {
            cache_policy: CachePolicy::Ttl { ttl_ms: 60_000 },
            request_policy: RequestPolicy::LatestWins,
            gc_time_ms: 300_000,
        });

        let key = QueryKey::from("user:1");
        let e1 = bucket.resource(key.clone(), cx);
        assert_eq!(bucket.count(), 1);

        // Same key returns the same entity
        let e2 = bucket.resource(key.clone(), cx);
        assert_eq!(bucket.count(), 1);
        assert_eq!(e1.entity_id(), e2.entity_id());

        // Different key creates a new entity
        let e3 = bucket.resource(QueryKey::from("user:2"), cx);
        assert_eq!(bucket.count(), 2);
        assert_ne!(e1.entity_id(), e3.entity_id());
    });
}

#[gpui::test]
fn bucket_begin_request_for_starts_loading(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut bucket: QueryBucket<User> = QueryBucket::new(BucketDefaults {
            cache_policy: CachePolicy::NoCache,
            request_policy: RequestPolicy::LatestWins,
            gc_time_ms: 300_000,
        });

        let key = QueryKey::from("user:1");
        bucket.resource(key.clone(), cx);

        let result = bucket.begin_request_for(&key, 1_000, QueryFetchMode::Normal, cx);
        assert!(matches!(result, Some(QueryBeginResult::Started { .. })));

        // Verify the entity is loading
        let entity = bucket.resources.get(&key).unwrap();
        assert!(entity.read(cx).is_loading());
    });
}

#[gpui::test]
fn bucket_begin_request_for_unknown_key_returns_none(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut bucket: QueryBucket<User> = QueryBucket::new(BucketDefaults {
            cache_policy: CachePolicy::NoCache,
            request_policy: RequestPolicy::LatestWins,
            gc_time_ms: 300_000,
        });

        let result =
            bucket.begin_request_for(&QueryKey::from("nonexistent"), 1_000, QueryFetchMode::Normal, cx);
        assert!(result.is_none());
    });
}

#[gpui::test]
fn bucket_gc_removes_stale_idle_resources(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut bucket: QueryBucket<User> = QueryBucket::new(BucketDefaults {
            cache_policy: CachePolicy::NoCache,
            request_policy: RequestPolicy::LatestWins,
            gc_time_ms: 1_000, // 1 second GC time
        });

        // Create resource with stale data
        let stale_key = QueryKey::from("stale_user");
        let entity = bucket.resource(stale_key.clone(), cx);
        entity.update(cx, |r, _| r.apply_success(default_user(), 100));
        assert_eq!(bucket.count(), 1);

        // GC at t=2000: age = 1900 > 1000 → collected
        bucket.gc(cx, 2_000, 1_000);
        assert_eq!(bucket.count(), 0);
    });
}

#[gpui::test]
fn bucket_gc_preserves_fresh_resources(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut bucket: QueryBucket<User> = QueryBucket::new(BucketDefaults {
            cache_policy: CachePolicy::NoCache,
            request_policy: RequestPolicy::LatestWins,
            gc_time_ms: 1_000,
        });

        let entity = bucket.resource(QueryKey::from("fresh_user"), cx);
        entity.update(cx, |r, _| r.apply_success(default_user(), 1_500));

        // GC at t=2000: age = 500 < 1000 → kept
        bucket.gc(cx, 2_000, 1_000);
        assert_eq!(bucket.count(), 1);
    });
}

#[gpui::test]
fn bucket_gc_preserves_resources_with_active_requests(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut bucket: QueryBucket<User> = QueryBucket::new(BucketDefaults {
            cache_policy: CachePolicy::NoCache,
            request_policy: RequestPolicy::LatestWins,
            gc_time_ms: 1_000,
        });

        let key = QueryKey::from("loading_user");
        bucket.resource(key.clone(), cx);
        bucket.begin_request_for(&key, 1_000, QueryFetchMode::Normal, cx);

        // GC at t=10000: resource is old but has an active request → kept
        bucket.gc(cx, 10_000, 1_000);
        assert_eq!(bucket.count(), 1);
    });
}

#[gpui::test]
fn bucket_invalidate_matching_uses_prefix_filter(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut bucket: QueryBucket<User> = QueryBucket::new(BucketDefaults {
            cache_policy: CachePolicy::Ttl { ttl_ms: 60_000 },
            request_policy: RequestPolicy::LatestWins,
            gc_time_ms: 300_000,
        });

        let u1_key = QueryKey::from(["users", "1"]);
        let u2_key = QueryKey::from(["users", "2"]);
        let u3_key = QueryKey::from(["admins", "1"]);

        let e1 = bucket.resource(u1_key.clone(), cx);
        let e2 = bucket.resource(u2_key.clone(), cx);
        let _ = bucket.resource(u3_key.clone(), cx);

        // Populate with cached data
        e1.update(cx, |r, _| r.apply_success(default_user(), 1_000));
        e2.update(cx, |r, _| {
            r.apply_success(
                User {
                    id: 2,
                    name: "Bob".into(),
                },
                1_000,
            )
        });

        // Invalidate all "users" keys
        let prefix = QueryKey::from(["users"]);
        bucket.invalidate_matching(&QueryKeyFilter::Prefix(&prefix), cx);

        // User resources: cache expired, data still present
        let e1 = bucket.resources.get(&u1_key).unwrap();
        assert!(e1.read(cx).data().is_some(), "data should remain after invalidate");
        assert!(
            !e1.read(cx).is_cache_fresh(1_500),
            "cache should be stale after invalidate"
        );

        let e2 = bucket.resources.get(&u2_key).unwrap();
        assert!(!e2.read(cx).is_cache_fresh(1_500));

        // Admin resource: unaffected
        let e3 = bucket.resources.get(&u3_key).unwrap();
        // No data was set for admins, so cache was never fresh
        assert!(!e3.read(cx).is_cache_fresh(1_500));
    });
}

#[gpui::test]
fn bucket_reset_matching_clears_all_state(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut bucket: QueryBucket<User> = QueryBucket::new(BucketDefaults {
            cache_policy: CachePolicy::Ttl { ttl_ms: 60_000 },
            request_policy: RequestPolicy::LatestWins,
            gc_time_ms: 300_000,
        });

        let key = QueryKey::from("user:1");
        let entity = bucket.resource(key.clone(), cx);
        entity.update(cx, |r, _| r.apply_success(default_user(), 1_000));

        assert!(entity.read(cx).data().is_some());

        bucket.reset_matching(&QueryKeyFilter::All, cx);

        assert!(entity.read(cx).data().is_none());
        assert_eq!(entity.read(cx).status(), QueryStatus::Idle);
    });
}

// ── QueryClient tests ──────────────────────────────────────────────────

#[gpui::test]
fn client_stores_multiple_types_in_separate_buckets(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut client = QueryClient::new(CachePolicy::NoCache, RequestPolicy::LatestWins);

        let user = client.resource::<User, QueryError>(QueryKey::from(["users", "1"]), cx);
        let post = client.resource::<Post, QueryError>(QueryKey::from(["posts", "1"]), cx);

        assert_eq!(client.bucket_count(), 2);
        assert_eq!(client.total_count(), 2);
        assert_ne!(user.entity_id(), post.entity_id());
    });
}

#[gpui::test]
fn client_deduplicates_same_key_same_type(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut client = QueryClient::new(CachePolicy::NoCache, RequestPolicy::LatestWins);

        let key = QueryKey::from(["users", "1"]);
        let e1 = client.resource::<User, QueryError>(key.clone(), cx);
        let e2 = client.resource::<User, QueryError>(key.clone(), cx);

        assert_eq!(client.total_count(), 1);
        assert_eq!(e1.entity_id(), e2.entity_id());
    });
}

#[gpui::test]
fn client_invalidate_queries_prefix_match_across_types(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut client =
            QueryClient::new(CachePolicy::Ttl { ttl_ms: 60_000 }, RequestPolicy::LatestWins);

        let u1 = client.resource::<User, QueryError>(QueryKey::from(["users", "1"]), cx);
        let u2 = client.resource::<User, QueryError>(QueryKey::from(["users", "2"]), cx);
        let p1 = client.resource::<Post, QueryError>(QueryKey::from(["posts", "1"]), cx);

        u1.update(cx, |r, _| r.apply_success(default_user(), 1_000));
        u2.update(cx, |r, _| {
            r.apply_success(
                User {
                    id: 2,
                    name: "Bob".into(),
                },
                1_000,
            )
        });
        p1.update(cx, |r, _| r.apply_success(default_post(), 1_000));

        // Invalidate all "users" — posts unaffected
        let prefix = QueryKey::from(["users"]);
        client.invalidate_queries(&QueryKeyFilter::Prefix(&prefix), cx);

        assert!(!u1.read(cx).is_cache_fresh(1_500), "user:1 cache should be stale");
        assert!(!u2.read(cx).is_cache_fresh(1_500), "user:2 cache should be stale");
        assert!(p1.read(cx).is_cache_fresh(1_500), "posts:1 cache should still be fresh");
    });
}

#[gpui::test]
fn client_invalidate_queries_all_matches_everything(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut client =
            QueryClient::new(CachePolicy::Ttl { ttl_ms: 60_000 }, RequestPolicy::LatestWins);

        let u = client.resource::<User, QueryError>(QueryKey::from("user"), cx);
        let p = client.resource::<Post, QueryError>(QueryKey::from("post"), cx);

        u.update(cx, |r, _| r.apply_success(default_user(), 1_000));
        p.update(cx, |r, _| r.apply_success(default_post(), 1_000));

        client.invalidate_queries(&QueryKeyFilter::All, cx);

        assert!(!u.read(cx).is_cache_fresh(1_500));
        assert!(!p.read(cx).is_cache_fresh(1_500));
    });
}

#[gpui::test]
fn client_gc_removes_stale_across_types(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut client =
            QueryClient::new(CachePolicy::NoCache, RequestPolicy::LatestWins).with_gc_time(1_000);

        let u = client.resource::<User, QueryError>(QueryKey::from("old_user"), cx);
        u.update(cx, |r, _| r.apply_success(default_user(), 100));

        let p = client.resource::<Post, QueryError>(QueryKey::from("fresh_post"), cx);
        p.update(cx, |r, _| r.apply_success(default_post(), 1_800));

        assert_eq!(client.total_count(), 2);

        // GC at t=2000: user age=1900 > 1000 (collected), post age=200 < 1000 (kept)
        client.gc(cx, 2_000);
        assert_eq!(client.total_count(), 1);

        // Verify the survivor is the post
        assert!(
            client.contains::<Post, QueryError>(&QueryKey::from("fresh_post")),
            "fresh post should survive GC"
        );
        assert!(
            !client.contains::<User, QueryError>(&QueryKey::from("old_user")),
            "stale user should be collected"
        );
    });
}

#[gpui::test]
fn client_reset_queries_clears_state(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut client =
            QueryClient::new(CachePolicy::Ttl { ttl_ms: 60_000 }, RequestPolicy::LatestWins);

        let u = client.resource::<User, QueryError>(QueryKey::from("user:1"), cx);
        u.update(cx, |r, _| r.apply_success(default_user(), 1_000));

        assert!(u.read(cx).data().is_some());

        client.reset_queries(&QueryKeyFilter::All, cx);

        assert!(u.read(cx).data().is_none());
        assert_eq!(u.read(cx).status(), QueryStatus::Idle);
    });
}

// ── Full lifecycle test ────────────────────────────────────────────────

#[gpui::test]
fn full_lifecycle_idle_to_loading_to_success_to_gc(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut client =
            QueryClient::new(CachePolicy::Ttl { ttl_ms: 1_000 }, RequestPolicy::LatestWins)
                .with_gc_time(5_000);

        // 1. Create resource
        let key = QueryKey::from(["users", "42"]);
        let entity = client.resource::<User, QueryError>(key.clone(), cx);
        assert_eq!(entity.read(cx).status(), QueryStatus::Idle);
        assert_eq!(client.total_count(), 1);

        // 2. Start request directly on the entity
        let sequencer = &mut RequestSequencer::new();
        entity.update(cx, |r, _| {
            let result = r.begin_request(sequencer, 1_000, QueryFetchMode::Normal);
            assert!(matches!(result, QueryBeginResult::Started { .. }));
        });
        assert!(entity.read(cx).is_loading());

        // 3. Complete with success at t=1_200
        let request_id = entity.read(cx).active_request_id().unwrap();
        let success = entity.update(cx, |r, _| {
            r.complete_current_success(
                request_id,
                User {
                    id: 42,
                    name: "Carol".into(),
                },
                1_200,
            )
        });
        assert!(success);
        assert_eq!(entity.read(cx).status(), QueryStatus::Success);
        assert_eq!(entity.read(cx).data().unwrap().name, "Carol");
        assert!(entity.read(cx).is_cache_fresh(1_500));

        // 4. GC before gc_time (age = 2_800 - 1_200 = 1_600 < 5_000) → kept
        client.gc(cx, 2_800);
        assert_eq!(client.total_count(), 1);

        // 5. GC after gc_time (age = 10_000 - 1_200 = 8_800 > 5_000) → collected
        client.gc(cx, 10_000);
        assert_eq!(client.total_count(), 0);
    });
}

#[gpui::test]
fn invalidated_resource_survives_gc_because_timestamp_is_cleared(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut client =
            QueryClient::new(CachePolicy::Ttl { ttl_ms: 1_000 }, RequestPolicy::LatestWins)
                .with_gc_time(1_000);

        let key = QueryKey::from("user:1");
        let entity = client.resource::<User, QueryError>(key.clone(), cx);
        entity.update(cx, |r, _| r.apply_success(default_user(), 100));

        // Invalidate clears last_updated_at → GC can't determine age → resource kept
        client.invalidate_queries(&QueryKeyFilter::All, cx);
        assert!(entity.read(cx).data().is_some(), "data survives invalidation");
        assert!(entity.read(cx).last_updated_at_ms().is_none(), "timestamp cleared");

        client.gc(cx, 100_000);
        assert_eq!(client.total_count(), 1, "invalidated resource survives GC");
    });
}
