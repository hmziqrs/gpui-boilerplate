//! Transport-agnostic query lifecycle primitives.
//!
//! `QueryResource` owns the cache/request state for one resource. Callers start
//! work with `begin_request`, then complete it with the returned `RequestId`.
//! Completion methods reject stale request ids, so cancelled or replaced async
//! work cannot overwrite newer state.

mod error;
mod key;
mod policy;
mod request;
mod resource;
mod status;

#[allow(unused_imports)]
pub use error::{QueryError, QueryErrorKind};
pub use key::QueryKey;
pub use policy::{CachePolicy, QueryBeginResult, QueryFetchMode, RequestPolicy};
pub use request::{QueryTimestamp, RequestGuard, RequestId, RequestSequencer};
pub use resource::QueryResource;
pub use status::QueryStatus;

#[cfg(test)]
mod test_support;

#[cfg(test)]
#[path = "tests/cache.rs"]
mod cache_test;

#[cfg(test)]
#[path = "tests/lifecycle.rs"]
mod lifecycle_test;

#[cfg(test)]
#[path = "tests/request.rs"]
mod request_test;
