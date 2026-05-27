#![allow(dead_code)]

use std::sync::{Mutex, OnceLock};

use gpui::{App, Global};
use serde::{Deserialize, Serialize};

use crate::time::AppTimestamp;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleStage {
    Starting,
    Running,
    ShuttingDown,
    Crashed,
}

#[derive(Clone, Debug)]
pub struct LifecycleState {
    pub stage: LifecycleStage,
    pub updated_at: AppTimestamp,
    pub startup_step: Option<String>,
    pub shutdown_step: Option<String>,
    pub last_startup_error: Option<String>,
    pub last_shutdown_error: Option<String>,
    pub last_error: Option<String>,
}

impl Global for LifecycleState {}

impl LifecycleState {
    pub fn starting() -> Self {
        Self {
            stage: LifecycleStage::Starting,
            updated_at: AppTimestamp::now(),
            startup_step: None,
            shutdown_step: None,
            last_startup_error: None,
            last_shutdown_error: None,
            last_error: None,
        }
    }
}

pub fn set_stage(stage: LifecycleStage, cx: &mut App) {
    let mut next = cx
        .try_global::<LifecycleState>()
        .cloned()
        .unwrap_or_else(LifecycleState::starting);
    next.stage = stage;
    next.updated_at = AppTimestamp::now();
    cx.set_global(next);
}

pub fn set_startup_step(step: impl Into<String>, cx: &mut App) {
    let mut next = cx
        .try_global::<LifecycleState>()
        .cloned()
        .unwrap_or_else(LifecycleState::starting);
    next.startup_step = Some(step.into());
    next.updated_at = AppTimestamp::now();
    cx.set_global(next);
}

pub fn set_shutdown_step(step: impl Into<String>, cx: &mut App) {
    let mut next = cx
        .try_global::<LifecycleState>()
        .cloned()
        .unwrap_or_else(LifecycleState::starting);
    next.shutdown_step = Some(step.into());
    next.updated_at = AppTimestamp::now();
    cx.set_global(next);
}

pub fn set_startup_error(error: impl Into<String>, cx: &mut App) {
    let error = error.into();
    let mut next = cx
        .try_global::<LifecycleState>()
        .cloned()
        .unwrap_or_else(LifecycleState::starting);
    next.last_startup_error = Some(error.clone());
    next.last_error = Some(error);
    next.updated_at = AppTimestamp::now();
    cx.set_global(next);
}

pub fn set_shutdown_error(error: impl Into<String>, cx: &mut App) {
    let error = error.into();
    let mut next = cx
        .try_global::<LifecycleState>()
        .cloned()
        .unwrap_or_else(LifecycleState::starting);
    next.last_shutdown_error = Some(error.clone());
    next.last_error = Some(error);
    next.updated_at = AppTimestamp::now();
    cx.set_global(next);
}

static LAST_PANIC_SUMMARY: OnceLock<Mutex<Option<String>>> = OnceLock::new();

pub fn last_panic_summary() -> Option<String> {
    LAST_PANIC_SUMMARY
        .get()
        .and_then(|slot| slot.lock().ok().and_then(|value| value.clone()))
}

pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let summary = info.to_string();
        let slot = LAST_PANIC_SUMMARY.get_or_init(|| Mutex::new(None));
        if let Ok(mut value) = slot.lock() {
            *value = Some(summary.clone());
        }
        tracing::error!(
            target: "gpui_starter::lifecycle",
            panic = %summary,
            "application panic captured"
        );
        previous(info);
    }));
}
