use gpui::{App, Global};
use serde::{Deserialize, Serialize};

use crate::{
    errors::AppError,
    ids::{EventId, TaskId},
    routes::AppRoute,
    time::AppTimestamp,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AppEventKind {
    Navigate(AppRoute),
    DeepLinkReceived(String),
    BackgroundTaskChanged(TaskId),
    AppError {
        message: String,
        severity: crate::errors::AppErrorSeverity,
    },
    DiagnosticsChanged,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppEvent {
    pub id: EventId,
    pub emitted_at: AppTimestamp,
    pub kind: AppEventKind,
}

impl AppEvent {
    pub fn new(kind: AppEventKind) -> Self {
        Self {
            id: EventId::new(),
            emitted_at: AppTimestamp::now(),
            kind,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AppEventQueue(pub Vec<AppEvent>);

impl Global for AppEventQueue {}

pub fn emit(kind: AppEventKind, cx: &mut App) {
    let event = AppEvent::new(kind);
    tracing::debug!(
        target: "gpui_starter::events",
        event_id = %event.id,
        kind = ?event.kind,
        "emitting app event"
    );
    cx.default_global::<AppEventQueue>().0.push(event);
}

pub fn emit_error(error: AppError, cx: &mut App) {
    let severity = error.severity();
    let message = error.to_string();
    tracing::warn!(
        target: "gpui_starter::events",
        severity = ?severity,
        error = %message,
        "emitting app error"
    );
    emit(AppEventKind::AppError { message, severity }, cx);
}

pub fn drain(cx: &mut App) -> Vec<AppEvent> {
    std::mem::take(&mut cx.default_global::<AppEventQueue>().0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_event_new_sets_id_and_timestamp() {
        let event = AppEvent::new(AppEventKind::DiagnosticsChanged);
        assert!(!event.id.to_string().is_empty());
        assert!(!event.emitted_at.to_rfc3339().is_empty());
    }

    #[test]
    fn queue_preserves_event_order() {
        let first = AppEvent::new(AppEventKind::DiagnosticsChanged);
        let second = AppEvent::new(AppEventKind::AppError {
            message: "oops".to_string(),
            severity: crate::errors::AppErrorSeverity::Error,
        });
        let third = AppEvent::new(AppEventKind::DeepLinkReceived(
            "gpui-starter://settings".to_string(),
        ));
        let queue = AppEventQueue(vec![first.clone(), second.clone(), third.clone()]);
        assert!(matches!(queue.0[0].kind, AppEventKind::DiagnosticsChanged));
        assert!(matches!(queue.0[1].kind, AppEventKind::AppError { .. }));
        assert!(matches!(queue.0[2].kind, AppEventKind::DeepLinkReceived(_)));
    }
}
