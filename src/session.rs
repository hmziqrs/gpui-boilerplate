use gpui::{App, Global};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionState {
    SignedOut,
    SigningIn,
    SignedIn { account_label: String },
    Error(String),
}

#[derive(Clone, Debug)]
pub struct SessionSnapshot {
    pub state: SessionState,
}

impl Default for SessionSnapshot {
    fn default() -> Self {
        Self {
            state: SessionState::SignedOut,
        }
    }
}

impl Global for SessionSnapshot {}

pub fn initialize(cx: &mut App) {
    cx.set_global(SessionSnapshot::default());
}

pub fn snapshot(cx: &App) -> SessionSnapshot {
    cx.try_global::<SessionSnapshot>()
        .cloned()
        .unwrap_or_default()
}

pub fn set_state(state: SessionState, cx: &mut App) {
    tracing::info!(target: "gpui_starter::session", state = ?state, "session state updated");
    cx.set_global(SessionSnapshot { state });
}
