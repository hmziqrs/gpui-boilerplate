//! Layer 0: Transport-agnostic query lifecycle primitives.
//!
//! `QueryResource` owns the cache/request state for one resource. Callers start
//! work with `begin_request`, then complete it with the returned `RequestId`.
//! Completion methods reject stale request ids, so cancelled or replaced async
//! work cannot overwrite newer state.
//!
//! This module depends only on `serde` — zero framework coupling.

mod error;
mod key;
pub mod key_filter;
mod policy;
mod request;
mod resource;
mod status;

pub use error::{QueryError, QueryErrorKind};
pub use key::QueryKey;
pub use key_filter::QueryKeyFilter;
pub use policy::{CachePolicy, QueryBeginResult, QueryFetchMode, RequestPolicy};
pub use request::{QueryTimestamp, RequestGuard, RequestId, RequestSequencer};
pub use resource::QueryResource;
pub use status::QueryStatus;
