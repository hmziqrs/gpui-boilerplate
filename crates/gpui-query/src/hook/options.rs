use crate::core::{CachePolicy, QueryKey, RequestPolicy};

/// Configuration for a single query, inspired by TanStack Query's `queryOptions()`.
///
/// Use this to define reusable query configurations that can be shared
/// across components.
pub struct QueryOptions<T, E = crate::core::QueryError> {
    /// The hierarchical cache key for this query.
    pub key: QueryKey,
    /// How cached data is treated (TTL, stale-while-revalidate, or no cache).
    pub cache_policy: CachePolicy,
    /// How concurrent requests are handled (latest wins, or ignore duplicates).
    pub request_policy: RequestPolicy,
    /// Garbage collection time in milliseconds. Resources idle longer than this
    /// may be collected by [`QueryClient::gc`](crate::client::QueryClient::gc).
    pub gc_time_ms: u64,
    /// Whether to bypass cache on the next fetch.
    pub force_fetch: bool,
    _marker: std::marker::PhantomData<(T, E)>,
}

impl<T, E> QueryOptions<T, E> {
    /// Create a new query options with the given key and default policies.
    pub fn new(key: impl Into<QueryKey>) -> Self {
        Self {
            key: key.into(),
            cache_policy: CachePolicy::default(),
            request_policy: RequestPolicy::default(),
            gc_time_ms: 5 * 60 * 1_000,
            force_fetch: false,
            _marker: std::marker::PhantomData,
        }
    }

    /// Set the cache policy.
    pub fn cache_policy(mut self, policy: CachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    /// Set the request policy.
    pub fn request_policy(mut self, policy: RequestPolicy) -> Self {
        self.request_policy = policy;
        self
    }

    /// Set the garbage collection time in milliseconds.
    pub fn gc_time_ms(mut self, ms: u64) -> Self {
        self.gc_time_ms = ms;
        self
    }

    /// Force a fresh fetch, bypassing any cache.
    pub fn force(mut self) -> Self {
        self.force_fetch = true;
        self
    }
}

impl Default for CachePolicy {
    fn default() -> Self {
        CachePolicy::Ttl { ttl_ms: 60_000 }
    }
}

impl Default for RequestPolicy {
    fn default() -> Self {
        RequestPolicy::LatestWins
    }
}
