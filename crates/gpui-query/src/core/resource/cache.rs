use crate::core::{QueryStatus, QueryTimestamp};

use super::QueryResource;

impl<T, E> QueryResource<T, E> {
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

    pub(crate) fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
        self.status = QueryStatus::Success;
        self.error = None;
    }

    pub fn invalidate(&mut self) {
        self.last_updated_at = None;
    }
}
