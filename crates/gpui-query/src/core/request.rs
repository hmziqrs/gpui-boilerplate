use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RequestId {
    scope_id: u64,
    sequence: u64,
}

impl RequestId {
    /// Create a request id with explicit scope and sequence.
    /// Primarily for testing; prefer [`RequestSequencer::next_request`] in production.
    pub fn scoped(scope_id: u64, sequence: u64) -> Self {
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
    pub(crate) scope_id: u64,
    pub(crate) next_request_id: u64,
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

    pub(super) fn elapsed_since(self, earlier: Self) -> Option<u128> {
        self.0.checked_sub(earlier.0)
    }
}

impl From<u128> for QueryTimestamp {
    fn from(value: u128) -> Self {
        Self::from_millis(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestGuard {
    request_id: RequestId,
}

impl RequestGuard {
    pub(super) fn new(request_id: RequestId) -> Self {
        Self { request_id }
    }

    pub fn request_id(self) -> RequestId {
        self.request_id
    }
}
