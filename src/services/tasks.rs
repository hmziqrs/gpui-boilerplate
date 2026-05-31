use gpui::{App, Global, Task};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::{
    events::{self, AppEventKind},
    ids::TaskId,
    time::AppTimestamp,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskProgress {
    Indeterminate,
    Percent(u8),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackgroundTask {
    pub id: TaskId,
    pub label: String,
    pub status: TaskStatus,
    pub progress: TaskProgress,
    pub started_at: AppTimestamp,
    pub finished_at: Option<AppTimestamp>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TaskRegistry {
    pub tasks: Vec<BackgroundTask>,
}

impl Global for TaskRegistry {}

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn is_shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::SeqCst)
}

pub fn request_shutdown() {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

#[allow(dead_code)]
pub fn reset_for_testing() {
    SHUTDOWN_REQUESTED.store(false, Ordering::SeqCst);
}

pub fn initialize(cx: &mut App) {
    cx.set_global(TaskRegistry::default());
}

pub fn snapshot(cx: &App) -> Vec<BackgroundTask> {
    cx.try_global::<TaskRegistry>()
        .map(|state| state.tasks.clone())
        .unwrap_or_default()
}

pub fn active_count(cx: &App) -> usize {
    snapshot(cx)
        .iter()
        .filter(|task| matches!(task.status, TaskStatus::Queued | TaskStatus::Running))
        .count()
}

pub fn start_demo_task(cx: &mut App) {
    let id = TaskId::new();
    start(
        id,
        "Demo background task".to_string(),
        TaskProgress::Percent(0),
        cx,
    );

    cx.spawn(async move |cx| {
        let background = cx.background_executor();
        for step in [20_u8, 40, 60, 80, 100] {
            if is_shutdown_requested() {
                cx.update(|cx| {
                    fail(id, "cancelled during shutdown".to_string(), cx);
                });
                return;
            }
            background.timer(Duration::from_millis(350)).await;
            cx.update(move |cx| {
                update_progress(id, TaskProgress::Percent(step), cx);
            });
        }
        cx.update(move |cx| {
            succeed(id, cx);
        });
    })
    .detach();
}

pub fn start(id: TaskId, label: String, progress: TaskProgress, cx: &mut App) {
    cx.default_global::<TaskRegistry>().tasks.insert(
        0,
        BackgroundTask {
            id,
            label,
            status: TaskStatus::Running,
            progress,
            started_at: AppTimestamp::now(),
            finished_at: None,
            error: None,
        },
    );
    events::emit(AppEventKind::BackgroundTaskChanged(id), cx);
}

pub fn update_progress(id: TaskId, progress: TaskProgress, cx: &mut App) {
    mutate_task(id, cx, |task| {
        task.progress = progress;
        task.status = TaskStatus::Running;
    });
}

pub fn succeed(id: TaskId, cx: &mut App) {
    mutate_task(id, cx, |task| {
        task.status = TaskStatus::Succeeded;
        task.progress = TaskProgress::Percent(100);
        task.finished_at = Some(AppTimestamp::now());
        task.error = None;
    });
}

pub fn fail(id: TaskId, error: String, cx: &mut App) {
    mutate_task(id, cx, |task| {
        task.status = TaskStatus::Failed;
        task.finished_at = Some(AppTimestamp::now());
        task.error = Some(error);
    });
}

pub fn cancel(id: TaskId, reason: String, cx: &mut App) {
    mutate_task(id, cx, |task| {
        task.status = TaskStatus::Cancelled;
        task.finished_at = Some(AppTimestamp::now());
        task.error = Some(reason);
    });
}

pub fn force_cancel_remaining(cx: &mut App) {
    let registry = cx.default_global::<TaskRegistry>();
    let mut changed = false;
    for task in &mut registry.tasks {
        if matches!(task.status, TaskStatus::Queued | TaskStatus::Running) {
            task.status = TaskStatus::Cancelled;
            task.finished_at = Some(AppTimestamp::now());
            task.error = Some("cancelled during app shutdown (drain timeout)".to_string());
            changed = true;
        }
    }
    if changed {
        tracing::info!(
            target: "gpui_starter::tasks",
            "force-cancelled remaining tasks after drain timeout"
        );
    }
}

pub fn drain_with_timeout(timeout: Duration, cx: &mut App) -> Task<()> {
    request_shutdown();

    cx.spawn(async move |cx| {
        let start = Instant::now();
        loop {
            let active = cx.update(|cx| active_count(cx));
            if active == 0 {
                tracing::info!(
                    target: "gpui_starter::tasks",
                    "all background tasks drained cooperatively"
                );
                return;
            }
            if start.elapsed() >= timeout {
                tracing::warn!(
                    target: "gpui_starter::tasks",
                    elapsed_ms = start.elapsed().as_millis() as u64,
                    remaining = active,
                    "drain deadline exceeded, force-cancelling remaining tasks"
                );
                cx.update(force_cancel_remaining);
                return;
            }
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
        }
    })
}

#[allow(dead_code)]
pub fn shutdown(cx: &mut App) {
    request_shutdown();
    force_cancel_remaining(cx);
}

fn mutate_task(id: TaskId, cx: &mut App, mutate: impl FnOnce(&mut BackgroundTask)) {
    let registry = cx.default_global::<TaskRegistry>();
    if let Some(task) = registry.tasks.iter_mut().find(|task| task.id == id) {
        mutate(task);
        events::emit(AppEventKind::BackgroundTaskChanged(id), cx);
    }
}
