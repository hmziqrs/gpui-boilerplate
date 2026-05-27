use std::collections::BTreeMap;

use gpui::{App, Global, SharedString};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityStatus {
    pub supported: bool,
    pub enabled: bool,
    pub degraded: bool,
    pub reason: Option<SharedString>,
    pub last_error: Option<SharedString>,
}

impl CapabilityStatus {
    pub fn supported_enabled() -> Self {
        Self {
            supported: true,
            enabled: true,
            degraded: false,
            reason: None,
            last_error: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CapabilityRegistry {
    entries: BTreeMap<String, CapabilityStatus>,
}

impl Global for CapabilityRegistry {}

pub fn initialize(cx: &mut App) {
    cx.set_global(CapabilityRegistry::default());
}

pub fn set(name: impl Into<String>, status: CapabilityStatus, cx: &mut App) {
    let key = name.into();
    let mut registry = cx
        .try_global::<CapabilityRegistry>()
        .cloned()
        .unwrap_or_default();
    tracing::debug!(
        target: "gpui_starter::capabilities",
        capability = %key,
        supported = status.supported,
        enabled = status.enabled,
        degraded = status.degraded,
        reason = ?status.reason,
        last_error = ?status.last_error,
        "capability updated"
    );
    registry.entries.insert(key, status);
    cx.set_global(registry);
}

pub fn snapshot(cx: &App) -> BTreeMap<String, CapabilityStatus> {
    cx.try_global::<CapabilityRegistry>()
        .map(|registry| registry.entries.clone())
        .unwrap_or_default()
}
