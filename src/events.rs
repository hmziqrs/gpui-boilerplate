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
    AppError(String),
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
    let mut queue = cx
        .try_global::<AppEventQueue>()
        .cloned()
        .unwrap_or_default();
    queue.0.push(event);
    cx.set_global(queue);
}

pub fn emit_error(error: AppError, cx: &mut App) {
    tracing::warn!(
        target: "gpui_starter::events",
        severity = ?error.severity(),
        error = %error,
        "emitting app error"
    );
    emit(AppEventKind::AppError(error.to_string()), cx);
}

pub fn drain(cx: &mut App) -> Vec<AppEvent> {
    let events = cx
        .try_global::<AppEventQueue>()
        .map(|queue| queue.0.clone())
        .unwrap_or_default();
    if !events.is_empty() {
        cx.set_global(AppEventQueue::default());
    }
    events
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
        let second = AppEvent::new(AppEventKind::AppError("oops".to_string()));
        let third = AppEvent::new(AppEventKind::DeepLinkReceived(
            "gpui-starter://settings".to_string(),
        ));
        let queue = AppEventQueue(vec![first.clone(), second.clone(), third.clone()]);
        assert!(matches!(queue.0[0].kind, AppEventKind::DiagnosticsChanged));
        assert!(matches!(queue.0[1].kind, AppEventKind::AppError(_)));
        assert!(matches!(queue.0[2].kind, AppEventKind::DeepLinkReceived(_)));
    }
}
