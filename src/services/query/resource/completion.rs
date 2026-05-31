use super::QueryResource;
use crate::services::query::{QueryStatus, QueryTimestamp, RequestGuard, RequestId};

impl<T, E> QueryResource<T, E> {
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

    pub(in crate::services::query) fn apply_success(&mut self, data: T, now_ms: u128) {
        self.status = QueryStatus::Success;
        self.data = Some(data);
        self.error = None;
        self.active_request_id = None;
        self.last_updated_at = Some(QueryTimestamp::from(now_ms));
    }

    pub(in crate::services::query) fn apply_failure(&mut self, error: impl Into<E>) {
        self.status = QueryStatus::Failure;
        self.error = Some(error.into());
        self.active_request_id = None;
    }

    pub(in crate::services::query) fn apply_success_optional(
        &mut self,
        data: Option<T>,
        now_ms: u128,
    ) {
        self.status = QueryStatus::Success;
        self.data = data;
        self.error = None;
        self.active_request_id = None;
        self.last_updated_at = Some(QueryTimestamp::from(now_ms));
    }

    pub(in crate::services::query) fn apply_failure_with_data(
        &mut self,
        data: T,
        error: impl Into<E>,
    ) {
        self.status = QueryStatus::Failure;
        self.data = Some(data);
        self.error = Some(error.into());
        self.active_request_id = None;
    }
}
