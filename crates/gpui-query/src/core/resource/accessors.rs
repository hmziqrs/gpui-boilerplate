use crate::core::{
    CachePolicy, QueryKey, QueryStatus, QueryTimestamp, RequestId, RequestPolicy,
};

use super::QueryResource;

impl<T, E> QueryResource<T, E> {
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
}
