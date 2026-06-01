//! The `QueryClient` — a GPUI [`Global`] that manages a registry of
//! [`QueryResource`](crate::core::QueryResource) entities, partitioned by type.
//!
//! # Type-partitioned storage
//!
//! Because Rust's type system requires `Entity<QueryResource<User>>` and
//! `Entity<QueryResource<Post>>` to be stored separately, the client uses
//! [`TypeId`] to partition resources into type-specific [`QueryBucket`]s.
//! Bulk operations (invalidate, reset, GC) operate through a type-erased
//! trait ([`QueryBucketTrait`]).
//!
//! # Quick start
//!
//! ```ignore
//! use gpui_query::client::QueryClient;
//! use gpui_query::{CachePolicy, QueryKey, RequestPolicy};
//!
//! // In your app setup:
//! cx.set_global(QueryClient::new(
//!     CachePolicy::Ttl { ttl_ms: 60_000 },
//!     RequestPolicy::LatestWins,
//! ));
//!
//! // In a component:
//! let client = cx.global_mut::<QueryClient>();
//! let user_entity = client.resource::<User, QueryError>(
//!     QueryKey::from(["users", "42"]),
//!     cx,
//! );
//! ```

mod bucket;

use std::any::TypeId;
use std::collections::HashMap;

use gpui::{App, Entity, Global};

use crate::core::{
    CachePolicy, QueryFetchMode, QueryKey, QueryKeyFilter, QueryResource, QuerySignal, RequestId,
    RequestPolicy,
};

pub use bucket::{BucketDefaults, QueryBucket, QueryBucketTrait};

/// App-wide query registry. Implements [`Global`] so it's accessible from any GPUI context.
///
/// Stores resources in type-partitioned buckets. Use [`resource`](Self::resource)`::<T, E>()`
/// for typed access and [`invalidate_queries`](Self::invalidate_queries)`()`
/// for bulk operations.
pub struct QueryClient {
    buckets: HashMap<TypeId, bucket::ErasedBucket>,
    default_cache_policy: CachePolicy,
    default_request_policy: RequestPolicy,
    default_gc_time_ms: u64,
}

impl Global for QueryClient {}

impl QueryClient {
    /// Create a new `QueryClient` with the given default policies.
    pub fn new(
        default_cache_policy: CachePolicy,
        default_request_policy: RequestPolicy,
    ) -> Self {
        Self {
            buckets: HashMap::new(),
            default_cache_policy,
            default_request_policy,
            default_gc_time_ms: 5 * 60 * 1_000, // 5 minutes default
        }
    }

    /// Set the default garbage collection time (in milliseconds).
    pub fn with_gc_time(mut self, gc_time_ms: u64) -> Self {
        self.default_gc_time_ms = gc_time_ms;
        self
    }

    // ── Typed access ───────────────────────────────────────────────────

    /// Get or create a [`QueryResource<T, E>`] entity for the given key.
    ///
    /// Uses the client's default cache and request policies.
    pub fn resource<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &mut self,
        key: QueryKey,
        cx: &mut App,
    ) -> Entity<QueryResource<T, E>> {
        let type_id = TypeId::of::<(T, E)>();
        self.ensure_bucket::<T, E>(type_id);
        self.buckets
            .get_mut(&type_id)
            .unwrap()
            .downcast_mut::<T, E>()
            .unwrap()
            .resource(key, cx)
    }

    /// Get or create a [`QueryResource<T, E>`] entity with explicit policies.
    pub fn resource_with_policies<
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        &mut self,
        key: QueryKey,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        cx: &mut App,
    ) -> Entity<QueryResource<T, E>> {
        let type_id = TypeId::of::<(T, E)>();
        self.ensure_bucket::<T, E>(type_id);
        self.buckets
            .get_mut(&type_id)
            .unwrap()
            .downcast_mut::<T, E>()
            .unwrap()
            .resource_with_policies(key, cache_policy, request_policy, cx)
    }

    /// Imperatively fetch a query without a component subscription.
    ///
    /// Creates the resource if it doesn't exist, begins a request, and returns
    /// the [`Entity`] and [`RequestId`]. The caller is responsible for completing
    /// the request by calling complete methods on the resource entity via
    /// `cx.update()`.
    ///
    /// Returns `None` (short-circuits) when:
    /// - The cache is fresh (`CacheHit`)
    /// - A request is already loading and `IgnoreWhileLoading` is set
    pub fn fetch_query<T, E>(
        &mut self,
        key: QueryKey,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        now_ms: u128,
        cx: &mut App,
    ) -> Option<(Entity<QueryResource<T, E>>, RequestId)>
    where
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    {
        self.fetch_query_inner(key, cache_policy, request_policy, now_ms, QueryFetchMode::Normal, cx)
    }

    /// Force-fetch a query, bypassing cache freshness checks.
    ///
    /// Behaves like [`fetch_query`](Self::fetch_query) but always starts a new
    /// request even when the cache is fresh. Still respects `IgnoreWhileLoading`
    /// if a request is already in flight.
    pub fn force_fetch_query<T, E>(
        &mut self,
        key: QueryKey,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        now_ms: u128,
        cx: &mut App,
    ) -> Option<(Entity<QueryResource<T, E>>, RequestId)>
    where
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    {
        self.fetch_query_inner(key, cache_policy, request_policy, now_ms, QueryFetchMode::Force, cx)
    }

    /// Check if a resource exists for the given key and type.
    pub fn contains<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &self,
        key: &QueryKey,
    ) -> bool {
        let type_id = TypeId::of::<(T, E)>();
        self.buckets
            .get(&type_id)
            .and_then(|b| b.downcast_ref::<T, E>())
            .map(|b| b.resources.contains_key(key))
            .unwrap_or(false)
    }

    /// Cancel the active request for a resource, also cancelling its signal.
    ///
    /// Returns `true` if there was an active request to cancel, `false` if the
    /// resource was idle or didn't exist.
    pub fn cancel_query<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &mut self,
        key: &QueryKey,
        error: E,
        cx: &mut App,
    ) -> bool {
        let type_id = TypeId::of::<(T, E)>();
        let Some(erased) = self.buckets.get_mut(&type_id) else {
            return false;
        };
        let bucket = erased.downcast_mut::<T, E>().unwrap();
        let Some(entity) = bucket.resources.get(key).cloned() else {
            return false;
        };
        entity.update(cx, |resource, _| resource.cancel(error))
    }

    /// Returns a clone of the cancellation signal for the resource at `key`,
    /// if the resource exists and has an active signal.
    pub fn signal_for<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &self,
        key: &QueryKey,
        cx: &App,
    ) -> Option<QuerySignal> {
        let type_id = TypeId::of::<(T, E)>();
        self.buckets
            .get(&type_id)
            .and_then(|b| b.downcast_ref::<T, E>())
            .and_then(|bucket| bucket.signal_for(key, cx))
    }

    /// Optimistically set data on a resource without completing a request.
    ///
    /// The current data is stored in `previous_data` for potential rollback
    /// via [`rollback_query_data`](Self::rollback_query_data).
    /// Returns `true` if the resource was found and updated, `false` otherwise.
    pub fn set_query_data<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &mut self,
        key: &QueryKey,
        data: T,
        cx: &mut App,
    ) -> bool {
        let type_id = TypeId::of::<(T, E)>();
        let Some(erased) = self.buckets.get_mut(&type_id) else {
            return false;
        };
        let bucket = erased.downcast_mut::<T, E>().unwrap();
        bucket.set_data_for(key, data, cx)
    }

    /// Roll back optimistically set data on a resource.
    ///
    /// Returns `true` if the resource was found and had previous data to restore.
    /// Returns `false` if the resource wasn't found or had no previous data.
    pub fn rollback_query_data<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        &mut self,
        key: &QueryKey,
        cx: &mut App,
    ) -> bool {
        let type_id = TypeId::of::<(T, E)>();
        let Some(erased) = self.buckets.get_mut(&type_id) else {
            return false;
        };
        let bucket = erased.downcast_mut::<T, E>().unwrap();
        bucket.rollback_data_for(key, cx)
    }

    // ── Bulk operations (type-erased) ──────────────────────────────────

    /// Invalidate (expire cache) for all resources matching the filter.
    ///
    /// Does **not** cancel active requests — only clears cache freshness
    /// so the next access will trigger a refetch.
    pub fn invalidate_queries(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        for erased in self.buckets.values_mut() {
            erased.bucket.invalidate_matching(filter, cx);
        }
    }

    /// Reset (clear all state) for all resources matching the filter.
    pub fn reset_queries(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        for erased in self.buckets.values_mut() {
            erased.bucket.reset_matching(filter, cx);
        }
    }

    /// Garbage-collect idle resources older than the GC time.
    pub fn gc(&mut self, cx: &mut App, now_ms: u128) {
        let gc_time_ms = self.default_gc_time_ms;
        for erased in self.buckets.values_mut() {
            erased.bucket.gc(cx, now_ms, gc_time_ms);
        }
    }

    /// Total number of resources across all type buckets.
    pub fn total_count(&self) -> usize {
        self.buckets.values().map(|b| b.bucket.count()).sum()
    }

    /// Number of type buckets.
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    // ── Private helpers ────────────────────────────────────────────────

    fn fetch_query_inner<T, E>(
        &mut self,
        key: QueryKey,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        now_ms: u128,
        fetch_mode: QueryFetchMode,
        cx: &mut App,
    ) -> Option<(Entity<QueryResource<T, E>>, RequestId)>
    where
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<(T, E)>();
        self.ensure_bucket::<T, E>(type_id);
        self.buckets
            .get_mut(&type_id)
            .unwrap()
            .downcast_mut::<T, E>()
            .unwrap()
            .fetch(&key, cache_policy, request_policy, now_ms, fetch_mode, cx)
    }

    fn ensure_bucket<
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    >(
        &mut self,
        type_id: TypeId,
    ) {
        self.buckets.entry(type_id).or_insert_with(|| {
            bucket::ErasedBucket::new_typed::<T, E>(QueryBucket::new(BucketDefaults {
                cache_policy: self.default_cache_policy,
                request_policy: self.default_request_policy,
                gc_time_ms: self.default_gc_time_ms,
            }))
        });
    }
}
