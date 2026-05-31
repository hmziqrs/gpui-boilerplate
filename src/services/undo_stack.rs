use gpui::{App, Global};
use gpui_component::ThemeMode;

use crate::time::AppTimestamp;

#[derive(Clone, Debug)]
pub struct UndoEntry {
    pub label: String,
    pub undo_label: String,
    pub redo_label: String,
    pub created_at: AppTimestamp,
    pub kind: UndoKind,
}

#[derive(Clone, Debug)]
pub enum UndoKind {
    ThemeMode { before: ThemeMode, after: ThemeMode },
}

#[derive(Clone, Debug, Default)]
pub struct UndoState {
    pub past: Vec<UndoEntry>,
    pub future: Vec<UndoEntry>,
    pub applying: bool,
    pub last_rejected: Option<String>,
}

impl Global for UndoState {}

pub fn initialize(cx: &mut App) {
    cx.set_global(UndoState::default());
    crate::capabilities::set(
        "undo_stack",
        crate::capabilities::CapabilityStatus::supported_enabled(),
        cx,
    );
}

pub fn snapshot(cx: &App) -> UndoState {
    cx.try_global::<UndoState>().cloned().unwrap_or_default()
}

pub fn can_undo(cx: &App) -> Option<String> {
    snapshot(cx)
        .past
        .last()
        .map(|entry| entry.undo_label.clone())
}

pub fn can_redo(cx: &App) -> Option<String> {
    snapshot(cx)
        .future
        .last()
        .map(|entry| entry.redo_label.clone())
}

pub fn record_theme_mode_change(before: ThemeMode, after: ThemeMode, cx: &mut App) {
    if before == after {
        return;
    }
    let state = cx.default_global::<UndoState>();
    let mut model = UndoModel::from_state(std::mem::take(state));
    model.record(UndoEntry {
        label: "Switch Theme".to_string(),
        undo_label: "Undo Theme Switch".to_string(),
        redo_label: "Redo Theme Switch".to_string(),
        created_at: AppTimestamp::now(),
        kind: UndoKind::ThemeMode { before, after },
    });
    *state = model.into_state();
    tracing::debug!(target: "gpui_starter::undo", "recorded theme mode change");
}

pub fn undo(cx: &mut App) -> bool {
    let state = cx.default_global::<UndoState>();
    let mut model = UndoModel::from_state(std::mem::take(state));
    let Some(entry) = model.pop_undo() else {
        *state = model.into_state();
        return false;
    };
    model.applying = true;
    *state = model.clone().into_state();
    let _ = state;
    apply_inverse(&entry.kind, cx);
    let state = cx.default_global::<UndoState>();
    let mut model = UndoModel::from_state(std::mem::take(state));
    model.applying = false;
    model.push_redo(entry);
    *state = model.into_state();
    true
}

pub fn redo(cx: &mut App) -> bool {
    let state = cx.default_global::<UndoState>();
    let mut model = UndoModel::from_state(std::mem::take(state));
    let Some(entry) = model.pop_redo() else {
        *state = model.into_state();
        return false;
    };
    model.applying = true;
    *state = model.clone().into_state();
    let _ = state;
    apply_forward(&entry.kind, cx);
    let state = cx.default_global::<UndoState>();
    let mut model = UndoModel::from_state(std::mem::take(state));
    model.applying = false;
    model.push_undo(entry);
    *state = model.into_state();
    true
}

fn apply_inverse(kind: &UndoKind, cx: &mut App) {
    match kind {
        UndoKind::ThemeMode { before, .. } => {
            crate::app::set_theme_mode_with_record(*before, false, cx)
        }
    }
}

fn apply_forward(kind: &UndoKind, cx: &mut App) {
    match kind {
        UndoKind::ThemeMode { after, .. } => {
            crate::app::set_theme_mode_with_record(*after, false, cx)
        }
    }
}

#[derive(Clone, Debug, Default)]
struct UndoModel {
    past: Vec<UndoEntry>,
    future: Vec<UndoEntry>,
    applying: bool,
    last_rejected: Option<String>,
}

impl UndoModel {
    fn from_state(state: UndoState) -> Self {
        Self {
            past: state.past,
            future: state.future,
            applying: state.applying,
            last_rejected: state.last_rejected,
        }
    }

    fn into_state(self) -> UndoState {
        UndoState {
            past: self.past,
            future: self.future,
            applying: self.applying,
            last_rejected: self.last_rejected,
        }
    }

    fn record(&mut self, entry: UndoEntry) {
        if self.applying {
            return;
        }
        self.past.push(entry);
        self.future.clear();
        self.last_rejected = None;
    }

    fn pop_undo(&mut self) -> Option<UndoEntry> {
        let entry = self.past.pop();
        if entry.is_none() {
            self.last_rejected = Some("nothing to undo".to_string());
        }
        entry
    }

    fn pop_redo(&mut self) -> Option<UndoEntry> {
        let entry = self.future.pop();
        if entry.is_none() {
            self.last_rejected = Some("nothing to redo".to_string());
        }
        entry
    }

    fn push_redo(&mut self, entry: UndoEntry) {
        self.future.push(entry);
        self.last_rejected = None;
    }

    fn push_undo(&mut self, entry: UndoEntry) {
        self.past.push(entry);
        self.last_rejected = None;
    }
}

#[cfg(test)]
#[path = "undo_stack.test.rs"]
mod undo_stack_test;
