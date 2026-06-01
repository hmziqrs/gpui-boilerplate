use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};

use gpui::App;
use tokio_util::sync::CancellationToken;

use crate::{ids::TaskId, services::query::RequestId};

const LOG: &str = "gpui_starter::http_lab::tasks";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum HttpTaskStatus {
    Succeeded,
    Failed(String),
    Cancelled(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HttpTaskUpdate {
    task_id: Option<TaskId>,
    status: HttpTaskStatus,
}

impl HttpTaskUpdate {
    pub(super) fn succeeded(task_id: Option<TaskId>) -> Self {
        Self {
            task_id,
            status: HttpTaskStatus::Succeeded,
        }
    }

    pub(super) fn failed(task_id: Option<TaskId>, error: String) -> Self {
        Self {
            task_id,
            status: HttpTaskStatus::Failed(error),
        }
    }

    pub(super) fn cancelled(task_id: Option<TaskId>, reason: String) -> Self {
        Self {
            task_id,
            status: HttpTaskStatus::Cancelled(reason),
        }
    }
}

pub(super) fn apply_task_update(update: Option<HttpTaskUpdate>, cx: &mut App) {
    let Some(HttpTaskUpdate { task_id, status }) = update else {
        return;
    };
    let Some(task_id) = task_id else {
        return;
    };
    tracing::debug!(
        target: LOG,
        task_id = ?task_id,
        status = ?status,
        "HTTP Lab applying background task update"
    );
    match status {
        HttpTaskStatus::Succeeded => crate::tasks::succeed(task_id, cx),
        HttpTaskStatus::Failed(error) => crate::tasks::fail(task_id, error, cx),
        HttpTaskStatus::Cancelled(reason) => crate::tasks::cancel(task_id, reason, cx),
    }
}

pub(super) fn cancellation_flags() -> &'static Mutex<BTreeMap<RequestId, CancellationToken>> {
    static FLAGS: OnceLock<Mutex<BTreeMap<RequestId, CancellationToken>>> = OnceLock::new();
    FLAGS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

pub(super) fn register_request_flag(request_id: RequestId) -> CancellationToken {
    let flag = CancellationToken::new();
    if let Ok(mut flags) = cancellation_flags().lock() {
        flags.insert(request_id, flag.clone());
        tracing::debug!(
            target: LOG,
            request_id = %request_id.label(),
            active_tokens = flags.len(),
            "HTTP Lab cancellation token registered"
        );
    }
    flag
}

pub(super) fn cancel_request_flag(request_id: RequestId) {
    if let Ok(flags) = cancellation_flags().lock()
        && let Some(flag) = flags.get(&request_id)
    {
        flag.cancel();
        tracing::info!(
            target: LOG,
            request_id = %request_id.label(),
            "HTTP Lab cancellation token cancelled"
        );
    }
}

pub(super) fn remove_request_flag(request_id: RequestId) {
    if let Ok(mut flags) = cancellation_flags().lock() {
        flags.remove(&request_id);
        tracing::debug!(
            target: LOG,
            request_id = %request_id.label(),
            active_tokens = flags.len(),
            "HTTP Lab cancellation token removed"
        );
    }
}
