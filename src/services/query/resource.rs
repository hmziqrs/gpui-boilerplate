use serde::{Deserialize, Serialize};

use super::{
    CachePolicy, QueryBeginResult, QueryError, QueryFetchMode, QueryKey, QueryStatus,
    QueryTimestamp, RequestGuard, RequestId, RequestPolicy, RequestSequencer,
};

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

    pub(super) fn begin_loading(&mut self, request_id: RequestId, now_ms: u128) -> QueryStatus {
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

    pub(super) fn record_cache_hit(&mut self) {
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
            Some(RequestGuard::new(request_id))
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

    pub(super) fn apply_success(&mut self, data: T, now_ms: u128) {
        self.status = QueryStatus::Success;
        self.data = Some(data);
        self.error = None;
        self.active_request_id = None;
        self.last_updated_at = Some(QueryTimestamp::from(now_ms));
    }

    pub(super) fn apply_failure(&mut self, error: impl Into<E>) {
        self.status = QueryStatus::Failure;
        self.error = Some(error.into());
        self.active_request_id = None;
    }

    pub(super) fn apply_success_optional(&mut self, data: Option<T>, now_ms: u128) {
        self.status = QueryStatus::Success;
        self.data = data;
        self.error = None;
        self.active_request_id = None;
        self.last_updated_at = Some(QueryTimestamp::from(now_ms));
    }

    pub(super) fn apply_failure_with_data(&mut self, data: T, error: impl Into<E>) {
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
