use gpui::{App, Global};
use serde::{Deserialize, Serialize};

use crate::{ids::EventId, time::AppTimestamp};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorAction {
    Retry,
    OpenSettings,
    CopyDetails,
    Dismiss,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub id: EventId,
    pub occurred_at: AppTimestamp,
    pub severity: crate::errors::AppErrorSeverity,
    pub message: String,
    pub actions: Vec<ErrorAction>,
}

#[derive(Clone, Debug, Default)]
pub struct ErrorSurfaceState {
    pub records: Vec<ErrorRecord>,
}

impl Global for ErrorSurfaceState {}

pub fn initialize(cx: &mut App) {
    cx.set_global(ErrorSurfaceState::default());
}

pub fn report(
    message: impl Into<String>,
    severity: crate::errors::AppErrorSeverity,
    actions: Vec<ErrorAction>,
    cx: &mut App,
) -> EventId {
    let mut state = cx
        .try_global::<ErrorSurfaceState>()
        .cloned()
        .unwrap_or_default();
    let record = ErrorRecord {
        id: EventId::new(),
        occurred_at: AppTimestamp::now(),
        severity,
        message: message.into(),
        actions,
    };
    let id = record.id;
    state.records.insert(0, record);
    if state.records.len() > 200 {
        state.records.truncate(200);
    }
    cx.set_global(state);
    id
}

pub fn snapshot(cx: &App) -> Vec<ErrorRecord> {
    cx.try_global::<ErrorSurfaceState>()
        .map(|state| state.records.clone())
        .unwrap_or_default()
}

pub fn latest(cx: &App) -> Option<ErrorRecord> {
    snapshot(cx).into_iter().next()
}

pub fn dismiss(id: EventId, cx: &mut App) {
    let mut state = cx
        .try_global::<ErrorSurfaceState>()
        .cloned()
        .unwrap_or_default();
    state.records.retain(|record| record.id != id);
    cx.set_global(state);
}
