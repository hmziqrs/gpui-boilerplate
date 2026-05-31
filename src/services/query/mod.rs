//! Transport-agnostic query lifecycle primitives.
//!
//! `QueryResource` owns the cache/request state for one resource. Callers start
//! work with `begin_request`, then complete it with the returned `RequestId`.
//! Completion methods reject stale request ids, so cancelled or replaced async
//! work cannot overwrite newer state.

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RequestId {
    scope_id: u64,
    sequence: u64,
}

impl RequestId {
    fn scoped(scope_id: u64, sequence: u64) -> Self {
        Self { scope_id, sequence }
    }

    pub fn value(self) -> u64 {
        self.sequence
    }

    pub fn scope_id(self) -> u64 {
        self.scope_id
    }

    pub fn label(self) -> String {
        format!("{}:{}", self.scope_id, self.sequence)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSequencer {
    scope_id: u64,
    next_request_id: u64,
}

impl Default for RequestSequencer {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestSequencer {
    pub fn new() -> Self {
        Self {
            scope_id: 1,
            next_request_id: 1,
        }
    }

    pub fn next_request(&mut self) -> RequestId {
        let request_id = RequestId::scoped(self.scope_id, self.next_request_id);
        if self.next_request_id == u64::MAX {
            self.advance_scope();
        } else {
            self.next_request_id += 1;
        }
        request_id
    }

    pub fn advance_scope(&mut self) {
        self.scope_id = self.scope_id.checked_add(1).unwrap_or(1);
        self.next_request_id = 1;
    }

    pub fn is_current_scope(&self, request_id: RequestId) -> bool {
        request_id.scope_id == self.scope_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct QueryTimestamp(u128);

impl QueryTimestamp {
    pub fn from_millis(value: u128) -> Self {
        Self(value)
    }

    pub fn as_millis(self) -> u128 {
        self.0
    }

    fn elapsed_since(self, earlier: Self) -> Option<u128> {
        self.0.checked_sub(earlier.0)
    }
}

impl From<u128> for QueryTimestamp {
    fn from(value: u128) -> Self {
        Self::from_millis(value)
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueryFetchMode {
    #[default]
    Normal,
    Force,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryBeginResult {
    Started {
        request_id: RequestId,
        status: QueryStatus,
        replaced_request_id: Option<RequestId>,
    },
    CacheHit,
    IgnoredWhileLoading {
        active_request_id: RequestId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestGuard {
    request_id: RequestId,
}

impl RequestGuard {
    pub fn request_id(self) -> RequestId {
        self.request_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryErrorKind {
    Cancelled,
    Response,
    Transport,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryError {
    kind: QueryErrorKind,
    message: String,
}

impl QueryError {
    pub fn new(kind: QueryErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::new(QueryErrorKind::Cancelled, message)
    }

    pub fn response(message: impl Into<String>) -> Self {
        Self::new(QueryErrorKind::Response, message)
    }

    pub fn transport(message: impl Into<String>) -> Self {
        Self::new(QueryErrorKind::Transport, message)
    }

    pub fn unknown(message: impl Into<String>) -> Self {
        Self::new(QueryErrorKind::Unknown, message)
    }

    pub fn kind(&self) -> QueryErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl AsRef<str> for QueryError {
    fn as_ref(&self) -> &str {
        self.message()
    }
}

impl From<String> for QueryError {
    fn from(value: String) -> Self {
        Self::unknown(value)
    }
}

impl From<&str> for QueryError {
    fn from(value: &str) -> Self {
        Self::unknown(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryResource<T, E = QueryError> {
    key: QueryKey,
    status: QueryStatus,
    data: Option<T>,
    error: Option<E>,
    active_request_id: Option<RequestId>,
    cache_policy: CachePolicy,
    request_policy: RequestPolicy,
    started_at: Option<QueryTimestamp>,
    last_updated_at: Option<QueryTimestamp>,
    cache_hits: u64,
    cancelled_count: u64,
    ignored_results: u64,
}

impl<T, E> QueryResource<T, E> {
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
            started_at: None,
            last_updated_at: None,
            cache_hits: 0,
            cancelled_count: 0,
            ignored_results: 0,
        }
    }

    pub fn is_loading(&self) -> bool {
        self.status.is_loading()
    }

    pub fn key(&self) -> &QueryKey {
        &self.key
    }

    pub fn status(&self) -> QueryStatus {
        self.status
    }

    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    pub fn error(&self) -> Option<&E> {
        self.error.as_ref()
    }

    pub fn active_request_id(&self) -> Option<RequestId> {
        self.active_request_id
    }

    pub fn cache_policy(&self) -> CachePolicy {
        self.cache_policy
    }

    pub fn request_policy(&self) -> RequestPolicy {
        self.request_policy
    }

    pub fn started_at_ms(&self) -> Option<u128> {
        self.started_at.map(QueryTimestamp::as_millis)
    }

    pub fn last_updated_at_ms(&self) -> Option<u128> {
        self.last_updated_at.map(QueryTimestamp::as_millis)
    }

    pub fn cache_hits(&self) -> u64 {
        self.cache_hits
    }

    pub fn cancelled_count(&self) -> u64 {
        self.cancelled_count
    }

    pub fn ignored_results(&self) -> u64 {
        self.ignored_results
    }

    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    pub fn cache_age_ms(&self, now_ms: u128) -> Option<u128> {
        QueryTimestamp::from(now_ms).elapsed_since(self.last_updated_at?)
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

    pub fn begin_request(
        &mut self,
        sequencer: &mut RequestSequencer,
        now_ms: u128,
        fetch_mode: QueryFetchMode,
    ) -> QueryBeginResult {
        if fetch_mode == QueryFetchMode::Normal && self.should_short_circuit_cache(now_ms) {
            self.record_cache_hit();
            return QueryBeginResult::CacheHit;
        }

        if self.request_policy == RequestPolicy::IgnoreWhileLoading
            && let Some(active_request_id) = self.active_request_id
        {
            return QueryBeginResult::IgnoredWhileLoading { active_request_id };
        }

        let replaced_request_id = self.active_request_id;
        if replaced_request_id.is_some() {
            self.cancelled_count += 1;
        }

        let request_id = sequencer.next_request();
        let status = self.begin_loading(request_id, now_ms);
        QueryBeginResult::Started {
            request_id,
            status,
            replaced_request_id,
        }
    }

    fn begin_loading(&mut self, request_id: RequestId, now_ms: u128) -> QueryStatus {
        let status = if self.has_data() {
            QueryStatus::LoadingWithData
        } else {
            QueryStatus::LoadingEmpty
        };
        self.status = status;
        self.active_request_id = Some(request_id);
        self.started_at = Some(QueryTimestamp::from(now_ms));
        self.error = None;
        status
    }

    fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
        self.status = QueryStatus::Success;
        self.error = None;
    }

    pub fn is_current_request(&self, request_id: RequestId) -> bool {
        self.active_request_id == Some(request_id)
    }

    pub fn accept_current_request(&mut self, request_id: RequestId) -> Option<RequestGuard> {
        if self.is_current_request(request_id) {
            self.active_request_id = None;
            Some(RequestGuard { request_id })
        } else {
            self.mark_ignored_result();
            None
        }
    }

    pub fn complete_current_success(
        &mut self,
        request_id: RequestId,
        data: T,
        now_ms: u128,
    ) -> bool {
        let Some(guard) = self.accept_current_request(request_id) else {
            return false;
        };
        self.complete_success(&guard, data, now_ms);
        true
    }

    pub fn complete_current_failure(&mut self, request_id: RequestId, error: impl Into<E>) -> bool {
        let Some(guard) = self.accept_current_request(request_id) else {
            return false;
        };
        self.complete_failure(&guard, error);
        true
    }

    pub fn complete_current_optional_success(
        &mut self,
        request_id: RequestId,
        data: Option<T>,
        now_ms: u128,
    ) -> bool {
        let Some(guard) = self.accept_current_request(request_id) else {
            return false;
        };
        self.complete_success_optional(&guard, data, now_ms);
        true
    }

    pub fn complete_current_failure_with_data(
        &mut self,
        request_id: RequestId,
        data: T,
        error: impl Into<E>,
    ) -> bool {
        let Some(guard) = self.accept_current_request(request_id) else {
            return false;
        };
        self.complete_failure_with_data(&guard, data, error);
        true
    }

    pub fn complete_success(&mut self, _guard: &RequestGuard, data: T, now_ms: u128) {
        self.apply_success(data, now_ms);
    }

    pub fn complete_failure(&mut self, _guard: &RequestGuard, error: impl Into<E>) {
        self.apply_failure(error);
    }

    pub fn complete_success_optional(
        &mut self,
        _guard: &RequestGuard,
        data: Option<T>,
        now_ms: u128,
    ) {
        self.apply_success_optional(data, now_ms);
    }

    pub fn complete_failure_with_data(
        &mut self,
        _guard: &RequestGuard,
        data: T,
        error: impl Into<E>,
    ) {
        self.apply_failure_with_data(data, error);
    }

    fn apply_success(&mut self, data: T, now_ms: u128) {
        self.status = QueryStatus::Success;
        self.data = Some(data);
        self.error = None;
        self.active_request_id = None;
        self.last_updated_at = Some(QueryTimestamp::from(now_ms));
    }

    fn apply_failure(&mut self, error: impl Into<E>) {
        self.status = QueryStatus::Failure;
        self.error = Some(error.into());
        self.active_request_id = None;
    }

    fn apply_success_optional(&mut self, data: Option<T>, now_ms: u128) {
        self.status = QueryStatus::Success;
        self.data = data;
        self.error = None;
        self.active_request_id = None;
        self.last_updated_at = Some(QueryTimestamp::from(now_ms));
    }

    fn apply_failure_with_data(&mut self, data: T, error: impl Into<E>) {
        self.status = QueryStatus::Failure;
        self.data = Some(data);
        self.error = Some(error.into());
        self.active_request_id = None;
    }

    pub fn cancel(&mut self, reason: impl Into<E>) -> bool {
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
        self.last_updated_at = None;
    }

    pub fn reset(&mut self) {
        self.status = QueryStatus::Idle;
        self.data = None;
        self.error = None;
        self.active_request_id = None;
        self.started_at = None;
        self.last_updated_at = None;
        self.cache_hits = 0;
        self.cancelled_count = 0;
        self.ignored_results = 0;
    }
}

#[cfg(test)]
#[path = "query.test.rs"]
mod query_test;
