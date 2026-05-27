use gpui::{App, Global};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    let mut registry = cx.try_global::<TaskRegistry>().cloned().unwrap_or_default();
    registry.tasks.insert(
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
    cx.set_global(registry);
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

pub fn shutdown(cx: &mut App) {
    let mut registry = cx.try_global::<TaskRegistry>().cloned().unwrap_or_default();
    let mut changed = false;
    for task in &mut registry.tasks {
        if matches!(task.status, TaskStatus::Queued | TaskStatus::Running) {
            task.status = TaskStatus::Cancelled;
            task.finished_at = Some(AppTimestamp::now());
            task.error = Some("cancelled during app shutdown".to_string());
            changed = true;
        }
    }
    if changed {
        tracing::info!(
            target: "gpui_starter::tasks",
            "cancelled running tasks during shutdown"
        );
        cx.set_global(registry);
    }
}

fn mutate_task(id: TaskId, cx: &mut App, mutate: impl FnOnce(&mut BackgroundTask)) {
    let mut registry = cx.try_global::<TaskRegistry>().cloned().unwrap_or_default();
    if let Some(task) = registry.tasks.iter_mut().find(|task| task.id == id) {
        mutate(task);
        cx.set_global(registry);
        events::emit(AppEventKind::BackgroundTaskChanged(id), cx);
    }
}
