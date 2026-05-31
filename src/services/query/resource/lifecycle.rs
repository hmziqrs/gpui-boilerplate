use super::QueryResource;
use crate::services::query::{
    QueryBeginResult, QueryFetchMode, QueryStatus, QueryTimestamp, RequestGuard, RequestId,
    RequestPolicy, RequestSequencer,
};

impl<T, E> QueryResource<T, E> {
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

    pub(in crate::services::query) fn begin_loading(
        &mut self,
        request_id: RequestId,
        now_ms: u128,
    ) -> QueryStatus {
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
