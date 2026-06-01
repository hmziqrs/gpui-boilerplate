use std::{sync::OnceLock, time::Instant};

use gpui::{App, BorrowAppContext as _};

use crate::{
    ids::TaskId,
    services::{
        http_lab::{
            client::run_http_action,
            state::{HttpLabState, ResetRequests},
            task_tracking::{apply_task_update, cancel_request_flag, register_request_flag},
            transitions::{
                apply_result_to_state, begin_action, cancel_action_in_state, cancel_all_in_state,
            },
            types::{ActionExchange, HttpLabAction},
        },
        query::RequestId,
        tokio_runtime::TokioRuntimeGlobal,
    },
    tasks::TaskProgress,
};

const LOG: &str = "gpui_starter::http_lab";

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
    tracing::info!(target: LOG, "HTTP Lab reset requested");
    let reset_requests = if cx.try_global::<HttpLabState>().is_some() {
        cx.update_global::<HttpLabState, _>(|state, _cx| state.reset_for_user())
    } else {
        cx.set_global(HttpLabState::default());
        ResetRequests::default()
    };

    for request_id in reset_requests.request_ids {
        tracing::debug!(
            target: LOG,
            request_id = %request_id.label(),
            "HTTP Lab reset cancelling request token"
        );
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

/// Prepare an HTTP action: update state, register task, return handles for spawning.
/// Returns `None` if the action was deduplicated (cache hit or already loading).
pub fn prepare_action(action: HttpLabAction, cx: &mut App) -> Option<ActionHandle> {
    let now_ms = now_ms();
    tracing::info!(
        target: LOG,
        action = action.id(),
        now_ms,
        "HTTP Lab preparing action"
    );
    let request_id =
        cx.update_global::<HttpLabState, _>(|state, _cx| begin_action(state, action, now_ms))?;

    let task_id = TaskId::new();
    tracing::info!(
        target: LOG,
        action = action.id(),
        request_id = %request_id.label(),
        task_id = ?task_id,
        "HTTP Lab action accepted"
    );
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
    tracing::debug!(
        target: LOG,
        action = action.id(),
        request_id = %request_id.label(),
        "HTTP Lab fetching Tokio runtime global"
    );
    let rt = cx.global::<TokioRuntimeGlobal>().0.runtime.clone();
    tracing::debug!(
        target: LOG,
        action = action.id(),
        request_id = %request_id.label(),
        "HTTP Lab fetching reqwest client global"
    );
    let client = cx.global::<TokioRuntimeGlobal>().0.http_client.clone();
    let request_cancellation = cancellation.clone();
    tracing::info!(
        target: LOG,
        action = action.id(),
        request_id = %request_id.label(),
        "HTTP Lab spawning Tokio request task immediately"
    );
    let http_handle = rt.spawn(async move {
        tracing::info!(
            target: LOG,
            action = action.id(),
            request_id = %request_id.label(),
            "HTTP Lab Tokio request task started"
        );
        let result = run_http_action(&client, action, request_cancellation).await;
        match &result {
            Ok(exchanges) => tracing::info!(
                target: LOG,
                action = action.id(),
                request_id = %request_id.label(),
                exchange_count = exchanges.len(),
                "HTTP Lab Tokio request task completed"
            ),
            Err(error) => tracing::warn!(
                target: LOG,
                action = action.id(),
                request_id = %request_id.label(),
                error = %error,
                "HTTP Lab Tokio request task failed"
            ),
        }
        result
    });
    tracing::info!(
        target: LOG,
        action = action.id(),
        request_id = %request_id.label(),
        "HTTP Lab action handle prepared"
    );

    Some(ActionHandle {
        action,
        request_id,
        cancellation,
        http_handle,
    })
}

/// Run a prepared action. The caller must spawn this on a GPUI entity context
/// so that `cx.update` can push results back into the view.
pub async fn execute_action(handle: ActionHandle, cx: &mut gpui::AsyncApp) {
    let started = Instant::now();
    let ActionHandle {
        action,
        request_id,
        cancellation,
        http_handle,
    } = handle;
    tracing::info!(
        target: LOG,
        action = action.id(),
        request_id = %request_id.label(),
        cancelled = cancellation.is_cancelled(),
        "HTTP Lab awaiting pre-spawned Tokio request task"
    );

    let result = http_handle
        .await
        .unwrap_or_else(|e| Err(format!("HTTP task panicked: {e}")));

    tracing::info!(
        target: LOG,
        action = action.id(),
        request_id = %request_id.label(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "HTTP Lab joined Tokio request task"
    );

    cx.update(move |cx| {
        tracing::info!(
            target: LOG,
            action = action.id(),
            request_id = %request_id.label(),
            "HTTP Lab applying result on GPUI thread"
        );
        apply_result(action, request_id, result, cx);
        tracing::info!(
            target: LOG,
            action = action.id(),
            request_id = %request_id.label(),
            "HTTP Lab applied result on GPUI thread"
        );
    });
}

pub struct ActionHandle {
    pub action: HttpLabAction,
    request_id: crate::services::query::RequestId,
    cancellation: tokio_util::sync::CancellationToken,
    http_handle: tokio::task::JoinHandle<Result<Vec<ActionExchange>, String>>,
}

pub fn cancel_action(action: HttpLabAction, cx: &mut App) {
    tracing::info!(target: LOG, action = action.id(), "HTTP Lab cancel requested");
    cx.update_global::<HttpLabState, _>(|state, _cx| {
        cancel_action_in_state(state, action, "Cancelled by user");
    });
}

pub fn cancel_all(cx: &mut App) {
    tracing::info!(target: LOG, "HTTP Lab cancel all requested");
    cx.update_global::<HttpLabState, _>(|state, _cx| {
        cancel_all_in_state(state, "Cancelled by user");
    });
}

fn apply_result(
    action: HttpLabAction,
    request_id: RequestId,
    result: Result<Vec<ActionExchange>, String>,
    cx: &mut App,
) {
    let now_ms = now_ms();
    tracing::debug!(
        target: LOG,
        action = action.id(),
        request_id = %request_id.label(),
        "HTTP Lab reducing result into state"
    );
    let task_update = cx.update_global::<HttpLabState, _>(|state, _cx| {
        apply_result_to_state(state, action, request_id, result, now_ms)
    });
    apply_task_update(task_update, cx);
}

fn now_ms() -> u128 {
    static STARTED_AT: OnceLock<Instant> = OnceLock::new();
    STARTED_AT.get_or_init(Instant::now).elapsed().as_millis()
}
