use std::{sync::OnceLock, time::Instant};

use gpui::{App, BorrowAppContext as _};

use crate::{
    ids::TaskId,
    services::{
        http_lab::{
            client::run_blocking_action,
            state::{HttpLabState, ResetRequests},
            task_tracking::{apply_task_update, cancel_request_flag, register_request_flag},
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

fn now_ms() -> u128 {
    static STARTED_AT: OnceLock<Instant> = OnceLock::new();
    STARTED_AT.get_or_init(Instant::now).elapsed().as_millis()
}
