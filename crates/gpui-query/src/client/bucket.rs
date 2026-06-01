use std::any::TypeId;
use std::collections::HashMap;

use gpui::{App, AppContext, Entity};

use crate::core::{
    CachePolicy, QueryBeginResult, QueryFetchMode, QueryKey, QueryKeyFilter, QueryResource,
    RequestPolicy, RequestSequencer,
};

/// Type-erased trait for bulk operations that don't need to know `T` or `E`.
///
/// Each [`QueryBucket`] implements this trait. The [`QueryClient`](super::QueryClient)
/// stores buckets as `Box<dyn QueryBucketTrait>` so it can iterate across all
/// type-partitioned buckets without knowing their concrete types.
pub trait QueryBucketTrait: Send + Sync {
    /// Invalidate (expire cache for) all resources matching the filter.
    fn invalidate_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App);

    /// Reset (clear all state for) all resources matching the filter.
    fn reset_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App);

    /// Remove resources that are idle and older than `gc_time_ms`.
    /// Resources with active requests are never collected.
    fn gc(&mut self, cx: &mut App, now_ms: u128, gc_time_ms: u64);

    /// Total number of resources in this bucket.
    fn count(&self) -> usize;
}

/// Default policies applied when creating new resources.
#[derive(Clone, Copy, Debug)]
pub struct BucketDefaults {
    pub cache_policy: CachePolicy,
    pub request_policy: RequestPolicy,
    pub gc_time_ms: u64,
}

/// A typed bucket storing [`QueryResource`] entities for one specific `(T, E)` type pair.
///
/// Each bucket also owns the [`RequestSequencer`]s for its resources, eliminating
/// the need for consumers to manage sequencers separately.
pub struct QueryBucket<T: 'static, E: 'static = crate::core::QueryError> {
    pub(crate) resources: HashMap<QueryKey, Entity<QueryResource<T, E>>>,
    sequencers: HashMap<QueryKey, RequestSequencer>,
    defaults: BucketDefaults,
}

impl<T: 'static, E: 'static> QueryBucket<T, E> {
    pub fn new(defaults: BucketDefaults) -> Self {
        Self {
            resources: HashMap::new(),
            sequencers: HashMap::new(),
            defaults,
        }
    }

    /// Get or create a [`QueryResource`] entity for the given key with default policies.
    pub fn resource(&mut self, key: QueryKey, cx: &mut App) -> Entity<QueryResource<T, E>> {
        self.resource_with_policies(
            key,
            self.defaults.cache_policy,
            self.defaults.request_policy,
            cx,
        )
    }

    /// Get or create a [`QueryResource`] entity with explicit policies.
    pub fn resource_with_policies(
        &mut self,
        key: QueryKey,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
        cx: &mut App,
    ) -> Entity<QueryResource<T, E>> {
        if let Some(entity) = self.resources.get(&key) {
            return entity.clone();
        }
        let entity = cx.new(|_| QueryResource::new(key.clone(), cache_policy, request_policy));
        self.resources.insert(key.clone(), entity.clone());
        self.sequencers.insert(key, RequestSequencer::new());
        entity
    }

    /// Begin a request on the resource for `key`, using the bucket's co-located sequencer.
    ///
    /// Returns `None` if the key doesn't exist in this bucket.
    pub fn begin_request_for(
        &mut self,
        key: &QueryKey,
        now_ms: u128,
        fetch_mode: QueryFetchMode,
        cx: &mut App,
    ) -> Option<QueryBeginResult> {
        let entity = self.resources.get(key).cloned()?;
        let sequencer = self.sequencers.entry(key.clone()).or_default();
        Some(entity.update(cx, |resource, _| resource.begin_request(sequencer, now_ms, fetch_mode)))
    }
}

impl<T: 'static, E: 'static> QueryBucketTrait for QueryBucket<T, E> {
    fn invalidate_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        for (key, entity) in &self.resources {
            if filter.matches(key) {
                let entity = entity.clone();
                entity.update(cx, |resource, _| {
                    resource.invalidate();
                });
            }
        }
    }

    fn reset_matching(&mut self, filter: &QueryKeyFilter, cx: &mut App) {
        for (key, entity) in &self.resources {
            if filter.matches(key) {
                let entity = entity.clone();
                entity.update(cx, |resource, _| {
                    resource.reset();
                });
            }
        }
    }

    fn gc(&mut self, cx: &mut App, now_ms: u128, gc_time_ms: u64) {
        self.resources.retain(|_key, entity| {
            let r = entity.read(cx);
            if r.active_request_id().is_some() {
                return true;
            }
            let Some(last) = r.last_updated_at_ms() else {
                return true;
            };
            let age = now_ms.saturating_sub(last);
            age <= gc_time_ms as u128
        });
        self.sequencers
            .retain(|key, _| self.resources.contains_key(key));
    }

    fn count(&self) -> usize {
        self.resources.len()
    }
}

// ── Type-erased storage helper ─────────────────────────────────────────

/// A type-erased bucket stored in [`QueryClient`](super::QueryClient).
/// Knows its `TypeId` for safe downcasting.
pub(crate) struct ErasedBucket {
    pub type_id: TypeId,
    pub bucket: Box<dyn QueryBucketTrait>,
}

impl ErasedBucket {
    pub fn new_typed<T: Clone + Send + Sync + 'static, E: Clone + Send + Sync + 'static>(
        bucket: QueryBucket<T, E>,
    ) -> Self {
        Self {
            type_id: TypeId::of::<(T, E)>(),
            bucket: Box::new(bucket),
        }
    }

    pub fn downcast_ref<T: 'static, E: 'static>(&self) -> Option<&QueryBucket<T, E>> {
        if self.type_id == TypeId::of::<(T, E)>() {
            // SAFETY: TypeId check guarantees the concrete type
            Some(unsafe {
                &*(self.bucket.as_ref() as *const dyn QueryBucketTrait as *const QueryBucket<T, E>)
            })
        } else {
            None
        }
    }

    pub fn downcast_mut<T: 'static, E: 'static>(&mut self) -> Option<&mut QueryBucket<T, E>> {
        if self.type_id == TypeId::of::<(T, E)>() {
            // SAFETY: TypeId check guarantees the concrete type
            Some(unsafe {
                &mut *(self.bucket.as_mut() as *mut dyn QueryBucketTrait as *mut QueryBucket<T, E>)
            })
        } else {
            None
        }
    }
}
