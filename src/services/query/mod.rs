use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QueryKey(String);

impl QueryKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for QueryKey {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for QueryKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestId(u64);

impl RequestId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSequencer {
    next_request_id: u64,
}

impl Default for RequestSequencer {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestSequencer {
    pub fn new() -> Self {
        Self { next_request_id: 1 }
    }

    pub fn next_request(&mut self) -> RequestId {
        let request_id = RequestId::new(self.next_request_id);
        self.next_request_id += 1;
        request_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryStatus {
    Idle,
    LoadingEmpty,
    LoadingWithData,
    Success,
    Failure,
    Cancelled,
}

impl QueryStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::LoadingEmpty => "Loading empty",
            Self::LoadingWithData => "Loading with data",
            Self::Success => "Success",
            Self::Failure => "Failure",
            Self::Cancelled => "Cancelled",
        }
    }

    pub fn is_loading(self) -> bool {
        matches!(self, Self::LoadingEmpty | Self::LoadingWithData)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CachePolicy {
    NoCache,
    Ttl { ttl_ms: u64 },
    StaleWhileRevalidate { ttl_ms: u64 },
}

impl CachePolicy {
    pub fn label(self) -> String {
        match self {
            Self::NoCache => "No cache".to_string(),
            Self::Ttl { ttl_ms } => format!("Cache TTL {}s", ttl_ms / 1_000),
            Self::StaleWhileRevalidate { ttl_ms } => {
                format!("Stale-while-revalidate {}s", ttl_ms / 1_000)
            }
        }
    }

    pub fn can_short_circuit(self) -> bool {
        matches!(self, Self::Ttl { .. })
    }

    pub fn ttl_ms(self) -> Option<u64> {
        match self {
            Self::NoCache => None,
            Self::Ttl { ttl_ms } | Self::StaleWhileRevalidate { ttl_ms } => Some(ttl_ms),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestPolicy {
    LatestWins,
    IgnoreWhileLoading,
}

impl RequestPolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::LatestWins => "Latest wins",
            Self::IgnoreWhileLoading => "Ignore while loading",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryResource<T> {
    pub key: QueryKey,
    pub status: QueryStatus,
    pub data: Option<T>,
    pub error: Option<String>,
    pub active_request_id: Option<RequestId>,
    pub cache_policy: CachePolicy,
    pub request_policy: RequestPolicy,
    pub started_at_ms: Option<u128>,
    pub last_updated_at_ms: Option<u128>,
    pub cache_hits: u64,
    pub cancelled_count: u64,
    pub ignored_results: u64,
}

impl<T> QueryResource<T> {
    pub fn new(
        key: impl Into<QueryKey>,
        cache_policy: CachePolicy,
        request_policy: RequestPolicy,
    ) -> Self {
        Self {
            key: key.into(),
            status: QueryStatus::Idle,
            data: None,
            error: None,
            active_request_id: None,
            cache_policy,
            request_policy,
            started_at_ms: None,
            last_updated_at_ms: None,
            cache_hits: 0,
            cancelled_count: 0,
            ignored_results: 0,
        }
    }

    pub fn is_loading(&self) -> bool {
        self.status.is_loading()
    }

    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    pub fn cache_age_ms(&self, now_ms: u128) -> Option<u128> {
        self.last_updated_at_ms
            .map(|updated_at| now_ms.saturating_sub(updated_at))
    }

    pub fn is_cache_fresh(&self, now_ms: u128) -> bool {
        self.has_data()
            && self
                .cache_policy
                .ttl_ms()
                .zip(self.cache_age_ms(now_ms))
                .map(|(ttl_ms, age_ms)| age_ms <= ttl_ms as u128)
                .unwrap_or(false)
    }

    pub fn should_short_circuit_cache(&self, now_ms: u128) -> bool {
        self.cache_policy.can_short_circuit() && self.is_cache_fresh(now_ms)
    }

    pub fn begin_loading(&mut self, request_id: RequestId, now_ms: u128) -> QueryStatus {
        let status = if self.has_data() {
            QueryStatus::LoadingWithData
        } else {
            QueryStatus::LoadingEmpty
        };
        self.status = status;
        self.active_request_id = Some(request_id);
        self.started_at_ms = Some(now_ms);
        self.error = None;
        status
    }

    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
        self.status = QueryStatus::Success;
        self.error = None;
    }

    pub fn is_current_request(&self, request_id: RequestId) -> bool {
        self.active_request_id == Some(request_id)
    }

    pub fn clear_current_request(&mut self, request_id: RequestId) -> bool {
        if self.is_current_request(request_id) {
            self.active_request_id = None;
            true
        } else {
            false
        }
    }

    pub fn apply_success(&mut self, data: T, now_ms: u128) {
        self.status = QueryStatus::Success;
        self.data = Some(data);
        self.error = None;
        self.active_request_id = None;
        self.last_updated_at_ms = Some(now_ms);
    }

    pub fn apply_failure(&mut self, error: impl Into<String>, now_ms: u128) {
        self.status = QueryStatus::Failure;
        self.error = Some(error.into());
        self.active_request_id = None;
        self.last_updated_at_ms = Some(now_ms);
    }

    pub fn apply_terminal(
        &mut self,
        status: QueryStatus,
        data: Option<T>,
        error: Option<String>,
        now_ms: u128,
    ) {
        self.status = status;
        if let Some(data) = data {
            self.data = Some(data);
        }
        self.error = error;
        self.active_request_id = None;
        self.last_updated_at_ms = Some(now_ms);
    }

    pub fn cancel(&mut self, reason: impl Into<String>) -> bool {
        if self.active_request_id.is_none() {
            return false;
        }

        self.active_request_id = None;
        self.status = QueryStatus::Cancelled;
        self.error = Some(reason.into());
        self.cancelled_count += 1;
        true
    }

    pub fn mark_ignored_result(&mut self) {
        self.ignored_results += 1;
    }

    pub fn invalidate(&mut self) {
        self.last_updated_at_ms = None;
    }

    pub fn reset(&mut self) {
        self.status = QueryStatus::Idle;
        self.data = None;
        self.error = None;
        self.active_request_id = None;
        self.started_at_ms = None;
        self.last_updated_at_ms = None;
        self.cache_hits = 0;
        self.cancelled_count = 0;
        self.ignored_results = 0;
    }
}

#[cfg(test)]
#[path = "query.test.rs"]
mod query_test;
