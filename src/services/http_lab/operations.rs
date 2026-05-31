use std::{sync::OnceLock, time::Instant};

use gpui::{App, BorrowAppContext as _};

use crate::{
    ids::TaskId,
    services::{
        http_lab::{
            client::run_blocking_action,
            state::{HttpLabState, ResetRequests},
            task_tracking::{HttpTaskUpdate, apply_task_update, cancel_request_flag, register_request_flag},
            transitions::{
                apply_result_to_state, begin_action, cancel_action_in_state, cancel_all_in_state,
            },
            types::{ActionExchange, HttpLabAction},
        },
        query::RequestId,
    },
    tasks::TaskProgress,
};

pub fn initialize(cx: &mut App) {
    cx.set_global(HttpLabState::default());
    crate::capabilities::set(
        "http_lab",
        crate::capabilities::CapabilityStatus::supported_enabled(),
        cx,
    );
}

pub fn snapshot(cx: &App) -> HttpLabState {
    cx.try_global::<HttpLabState>().cloned().unwrap_or_default()
}

pub fn read_state<R>(cx: &App, read: impl FnOnce(&HttpLabState) -> R) -> R {
    if let Some(state) = cx.try_global::<HttpLabState>() {
        read(state)
    } else {
        let fallback = HttpLabState::default();
        read(&fallback)
    }
}

pub fn reset(cx: &mut App) {
    let reset_requests = if cx.try_global::<HttpLabState>().is_some() {
        cx.update_global::<HttpLabState, _>(|state, _cx| state.reset_for_user())
    } else {
        cx.set_global(HttpLabState::default());
        ResetRequests::default()
    };

    for request_id in reset_requests.request_ids {
        cancel_request_flag(request_id);
    }
    for task_id in reset_requests.task_ids {
        crate::tasks::cancel(task_id, "HTTP Lab reset".to_string(), cx);
    }
}

pub fn select_action(action: HttpLabAction, cx: &mut App) {
    cx.update_global::<HttpLabState, _>(|state, _cx| {
        state.selected_action = action;
    });
}

pub fn run_action(action: HttpLabAction, cx: &mut App) {
    let now_ms = now_ms();
    let request_id =
        cx.update_global::<HttpLabState, _>(|state, _cx| begin_action(state, action, now_ms));

    let Some(request_id) = request_id else {
        return;
    };

    let task_id = TaskId::new();
    crate::tasks::start(
        task_id,
        format!("HTTP Lab {}", action.label()),
        TaskProgress::Indeterminate,
        cx,
    );
    cx.update_global::<HttpLabState, _>(|state, _cx| {
        state.inflight_tasks.insert(request_id, task_id);
    });
    let cancellation = register_request_flag(request_id);

    cx.spawn(async move |cx| {
        let result = cx
            .background_executor()
            .spawn(async move { run_blocking_action(action, cancellation) })
            .await;

        cx.update(move |cx| {
            apply_result(action, request_id, result, cx);
        });
    })
    .detach();
}

pub fn cancel_action(action: HttpLabAction, cx: &mut App) {
    cx.update_global::<HttpLabState, _>(|state, _cx| {
        cancel_action_in_state(state, action, "Cancelled by user");
    });
}

pub fn cancel_all(cx: &mut App) {
    cx.update_global::<HttpLabState, _>(|state, _cx| {
        cancel_all_in_state(state, "Cancelled by user");
    });
}

pub(super) fn begin_action(
    state: &mut HttpLabState,
    action: HttpLabAction,
    now_ms: u128,
) -> Option<RequestId> {
    state.selected_action = action;

    if action == HttpLabAction::FullFlow {
        cancel_all_in_state(state, "Cancelled by full flow");
    } else {
        cancel_action_in_state(
            state,
            HttpLabAction::FullFlow,
            "Cancelled by individual request",
        );
    }

    let has_data = state.resource(action).has_data();
    let cache_policy = state.resource(action).cache_policy();
    let begin_result = {
        let HttpLabState {
            resources,
            request_sequencer,
            ..
        } = state;
        let resource = resources.get_mut(&action)?;
        resource.begin_request(request_sequencer, now_ms, QueryFetchMode::Normal)
    };

    match begin_result {
        QueryBeginResult::Started {
            request_id,
            status,
            replaced_request_id,
        } => {
            if replaced_request_id.is_some() {
                record_transition(
                    state,
                    QueryStatus::Cancelled,
                    action.label(),
                    "cancelled by newer request",
                );
            }
            let note = match (cache_policy, has_data) {
                (CachePolicy::StaleWhileRevalidate { .. }, true) => "revalidating cached data",
                _ => "request started",
            };
            record_transition(state, status, action.label(), note);
            Some(request_id)
        }
        QueryBeginResult::CacheHit => {
            record_transition(state, QueryStatus::Success, action.label(), "cache hit");
            None
        }
        QueryBeginResult::IgnoredWhileLoading { .. } => {
            record_transition(
                state,
                state.resource(action).status(),
                action.label(),
                "ignored duplicate while loading",
            );
            None
        }
    }
}

fn apply_result(
    action: HttpLabAction,
    request_id: RequestId,
    result: Result<Vec<ActionExchange>, String>,
    cx: &mut App,
) {
    let now_ms = now_ms();
    let task_update = cx.update_global::<HttpLabState, _>(|state, _cx| {
        apply_result_to_state(state, action, request_id, result, now_ms)
    });
    apply_task_update(task_update, cx);
}

pub(super) fn apply_result_to_state(
    state: &mut HttpLabState,
    action: HttpLabAction,
    request_id: RequestId,
    result: Result<Vec<ActionExchange>, String>,
    now_ms: u128,
) -> Option<HttpTaskUpdate> {
    let task_id = state.inflight_tasks.remove(&request_id);
    remove_request_flag(request_id);
    if !state.request_sequencer.is_current_scope(request_id) {
        return Some(HttpTaskUpdate::cancelled(
            task_id,
            "ignored stale request scope".to_string(),
        ));
    }

    let Some(request_guard) = state
        .resources
        .get_mut(&action)
        .and_then(|resource| resource.accept_current_request(request_id))
    else {
        record_ignored_result(state, action, request_id);
        return Some(HttpTaskUpdate::cancelled(
            task_id,
            format!("ignored stale request {}", request_id.label()),
        ));
    };

    match result {
        Ok(exchanges) => {
            let last_exchange = exchanges.last().map(|(_, exchange)| exchange.clone());
            for (index, (target_action, exchange)) in exchanges.into_iter().enumerate() {
                if action == HttpLabAction::FullFlow && index > 0 {
                    record_transition(
                        state,
                        QueryStatus::LoadingWithData,
                        target_action.label(),
                        "full flow advanced",
                    );
                }
                finish_exchange(state, target_action, exchange, &request_guard, now_ms);
            }

            if action == HttpLabAction::FullFlow {
                finish_flow_resource(state, last_exchange, &request_guard, now_ms);
            }
            Some(HttpTaskUpdate::succeeded(task_id))
        }
        Err(error) => {
            fail_resource(state, action, &request_guard, error.clone());
            Some(HttpTaskUpdate::failed(task_id, error))
        }
    }
}

fn finish_exchange(
    state: &mut HttpLabState,
    action: HttpLabAction,
    exchange: HttpExchange,
    request_guard: &RequestGuard,
    now_ms: u128,
) {
    let status = if exchange.error.is_none() {
        QueryStatus::Success
    } else {
        QueryStatus::Failure
    };
    let error = exchange.error.clone();

    if let Some(resource) = state.resources.get_mut(&action) {
        match error {
            Some(error) => {
                resource.complete_failure_with_data(
                    request_guard,
                    exchange.clone(),
                    QueryError::response(error),
                );
            }
            None => resource.complete_success(request_guard, exchange.clone(), now_ms),
        }
    }

    if let Some(cookie_snapshot) = cookie_snapshot_from_exchange(&exchange) {
        state.cookies = Some(cookie_snapshot);
    }

    record_transition(state, status, action.label(), "response applied");
    push_history(state, exchange);
}

fn finish_flow_resource(
    state: &mut HttpLabState,
    last_exchange: Option<HttpExchange>,
    request_guard: &RequestGuard,
    now_ms: u128,
) {
    if let Some(resource) = state.resources.get_mut(&HttpLabAction::FullFlow) {
        resource.complete_success_optional(request_guard, last_exchange, now_ms);
    }
    record_transition(
        state,
        QueryStatus::Success,
        HttpLabAction::FullFlow.label(),
        "flow completed",
    );
}

fn fail_resource(
    state: &mut HttpLabState,
    action: HttpLabAction,
    request_guard: &RequestGuard,
    error: String,
) {
    let exchange = HttpExchange {
        label: action.label().to_string(),
        request: HttpRequestSnapshot {
            method: "-".to_string(),
            url: HTTPBIN_BASE.to_string(),
            request_body_kind: HttpRequestBodyKind::None,
            request_body_preview: String::new(),
        },
        response: None,
        error: Some(error.clone()),
    };

    if let Some(resource) = state.resources.get_mut(&action) {
        resource.complete_failure(request_guard, QueryError::transport(error));
    }

    record_transition(
        state,
        QueryStatus::Failure,
        action.label(),
        "request failed",
    );
    push_history(state, exchange);
}

pub(super) fn cancel_action_in_state(
    state: &mut HttpLabState,
    action: HttpLabAction,
    reason: &str,
) {
    if let Some(request_id) = state.resource(action).active_request_id() {
        cancel_request_flag(request_id);
        if let Some(resource) = state.resources.get_mut(&action) {
            resource.cancel(QueryError::cancelled(reason));
        }
        record_transition(state, QueryStatus::Cancelled, action.label(), reason);
    }
}

fn cancel_all_in_state(state: &mut HttpLabState, reason: &str) {
    let actions = HttpLabAction::all()
        .iter()
        .copied()
        .filter(|action| state.resource(*action).active_request_id().is_some())
        .collect::<Vec<_>>();
    for action in actions {
        cancel_action_in_state(state, action, reason);
    }
}

fn record_ignored_result(state: &mut HttpLabState, action: HttpLabAction, request_id: RequestId) {
    record_transition(
        state,
        QueryStatus::Cancelled,
        action.label(),
        &format!("ignored stale request {}", request_id.label()),
    );
}

fn record_transition(state: &mut HttpLabState, status: QueryStatus, label: &str, note: &str) {
    state
        .transition_log
        .insert(0, format!("{}: {label} ({note})", status.label()));
    state.transition_log.truncate(24);
}

fn push_history(state: &mut HttpLabState, exchange: HttpExchange) {
    state.history.insert(0, exchange);
    state.history.truncate(24);
}

fn now_ms() -> u128 {
    static STARTED_AT: OnceLock<Instant> = OnceLock::new();
    STARTED_AT.get_or_init(Instant::now).elapsed().as_millis()
}
