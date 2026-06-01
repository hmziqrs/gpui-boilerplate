//! The `use_query` hook — ergonomic query subscription for GPUI components.
//!
//! # Usage
//!
//! ```ignore
//! use gpui_query::hook::use_query;
//! use gpui_query::{CachePolicy, QueryKey, RequestPolicy};
//!
//! struct MyView {
//!     users: gpui::Entity<gpui_query::QueryResource<Vec<User>>>,
//!     _subscription: gpui::Subscription,
//! }
//!
//! impl MyView {
//!     fn new(cx: &mut gpui::Context<Self>) -> Self {
//!         let (users, _subscription) = use_query(
//!             QueryKey::from(["users"]),
//!             CachePolicy::Ttl { ttl_ms: 60_000 },
//!             RequestPolicy::LatestWins,
//!             || async {
//!                 let resp = reqwest::get("/api/users").await?;
//!                 let users: Vec<User> = resp.json().await?;
//!                 Ok(users)
//!             },
//!             cx,
//!         );
//!         Self { users, _subscription }
//!     }
//! }
//! ```

mod options;

pub use options::QueryOptions;

use gpui::{AppContext, BorrowAppContext, Context, Entity, Subscription};

use crate::client::QueryClient;
use crate::core::{QueryKey, QueryResource, QueryStatus};

/// Subscribe to a query resource and automatically re-render when it changes.
///
/// Call this in your component's constructor (not in `render`). It:
///
/// 1. Gets or creates a [`QueryResource`] entity from the global [`QueryClient`]
/// 2. Calls `cx.observe()` so your component re-renders on state changes
/// 3. Starts an async fetch if the resource is idle
///
/// # Returns
///
/// A tuple of `(Entity<QueryResource<T, E>>, Subscription)`:
/// - Store the entity to read state during render
/// - Store the subscription to keep the observation alive
pub fn use_query<T, E, C, F, Fut>(
    key: QueryKey,
    cache_policy: crate::core::CachePolicy,
    request_policy: crate::core::RequestPolicy,
    fetcher: F,
    cx: &mut Context<C>,
) -> (Entity<QueryResource<T, E>>, Subscription)
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    C: 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    let (entity, subscription) = use_query_manual(key, cache_policy, request_policy, cx);

    // Start fetch if resource is idle
    let should_fetch = entity.read(cx).status() == QueryStatus::Idle;
    if should_fetch {
        let weak = entity.downgrade();
        cx.spawn(async move |_this, cx| {
            let result = fetcher().await;
            let now_ms = current_time_ms();
            let entity = match weak.upgrade() {
                Some(e) => e,
                None => return Ok::<_, ()>(()),
            };
            entity.update(cx, |resource, cx| {
                if let Some(guard) = resource.accept_current_request(
                    resource
                        .active_request_id()
                        .unwrap_or(crate::core::RequestId::scoped(0, 0)),
                ) {
                    match result {
                        Ok(data) => {
                            resource.complete_success(&guard, data, now_ms);
                        }
                        Err(error) => {
                            resource.complete_failure(&guard, error);
                        }
                    }
                }
                cx.notify();
            });
            Ok::<_, ()>(())
        })
        .detach();
    }

    (entity, subscription)
}

/// Lower-level hook that sets up the entity and observation without starting a fetch.
///
/// Use this when you need full control over when and how fetching happens.
pub fn use_query_manual<T, E, C>(
    key: QueryKey,
    cache_policy: crate::core::CachePolicy,
    request_policy: crate::core::RequestPolicy,
    cx: &mut Context<C>,
) -> (Entity<QueryResource<T, E>>, Subscription)
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    C: 'static,
{
    let entity = if cx.has_global::<QueryClient>() {
        cx.update_global::<QueryClient, _>(|client, cx| {
            client.resource_with_policies::<T, E>(key, cache_policy, request_policy, cx)
        })
    } else {
        cx.new(|_| QueryResource::new(key, cache_policy, request_policy))
    };

    let subscription = cx.observe(&entity, |_, _, cx| {
        cx.notify();
    });

    (entity, subscription)
}

/// Initiate a fetch on an existing query entity.
///
/// Call this when you want to refetch (e.g., on button click or timer).
pub fn fetch_query<T, E, C, F, Fut>(
    entity: &Entity<QueryResource<T, E>>,
    fetcher: F,
    cx: &mut Context<C>,
) where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    C: 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
{
    let weak = entity.downgrade();
    cx.spawn(async move |_this, cx| {
        let result = fetcher().await;
        let now_ms = current_time_ms();
        let entity = match weak.upgrade() {
            Some(e) => e,
            None => return Ok::<_, ()>(()),
        };
        entity.update(cx, |resource, cx| {
            if let Some(guard) = resource.accept_current_request(
                resource
                    .active_request_id()
                    .unwrap_or(crate::core::RequestId::scoped(0, 0)),
            ) {
                match result {
                    Ok(data) => {
                        resource.complete_success(&guard, data, now_ms);
                    }
                    Err(error) => {
                        resource.complete_failure(&guard, error);
                    }
                }
            }
            cx.notify();
        });
        Ok::<_, ()>(())
    })
    .detach();
}

/// Returns current time as milliseconds since UNIX epoch.
pub fn current_time_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
