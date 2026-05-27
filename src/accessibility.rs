use gpui::{App, Global};

#[derive(Clone, Debug)]
pub struct AccessibilitySnapshot {
    pub accesskit_linked: bool,
    pub bridge_enabled: bool,
    pub status: String,
}

impl Default for AccessibilitySnapshot {
    fn default() -> Self {
        Self {
            accesskit_linked: true,
            bridge_enabled: false,
            status: "keyboard/focus baseline present; full bridge pending".to_string(),
        }
    }
}

impl Global for AccessibilitySnapshot {}

pub fn initialize(cx: &mut App) {
    let _role = accesskit::Role::Window;
    let snapshot = AccessibilitySnapshot::default();
    crate::capabilities::set(
        "accessibility",
        crate::capabilities::CapabilityStatus {
            supported: true,
            enabled: snapshot.bridge_enabled,
            degraded: !snapshot.bridge_enabled,
            reason: Some(snapshot.status.clone().into()),
            last_error: None,
        },
        cx,
    );
    cx.set_global(snapshot);
}

pub fn snapshot(cx: &App) -> AccessibilitySnapshot {
    cx.try_global::<AccessibilitySnapshot>()
        .cloned()
        .unwrap_or_default()
}
