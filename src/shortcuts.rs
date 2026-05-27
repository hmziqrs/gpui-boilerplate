use gpui::{App, Global};

#[derive(Clone, Debug, Default)]
pub struct ShortcutState {
    pub enabled: bool,
    pub registered: bool,
    pub accelerator: String,
    pub last_error: Option<String>,
}

impl Global for ShortcutState {}

#[cfg(target_os = "macos")]
pub fn initialize(cx: &mut App) {
    use global_hotkey::{
        GlobalHotKeyManager,
        hotkey::{Code, HotKey, Modifiers},
    };

    let enabled = crate::app_state::config(cx).global_shortcut_enabled;
    let mut state = ShortcutState {
        enabled,
        registered: false,
        accelerator: "Alt+Space".to_string(),
        last_error: None,
    };

    if enabled {
        let manager = GlobalHotKeyManager::new();
        match manager {
            Ok(manager) => {
                let hotkey = HotKey::new(Some(Modifiers::ALT), Code::Space);
                match manager.register(hotkey) {
                    Ok(()) => {
                        Box::leak(Box::new(manager));
                        state.registered = true;
                    }
                    Err(err) => {
                        state.last_error = Some(err.to_string());
                    }
                }
            }
            Err(err) => {
                state.last_error = Some(err.to_string());
            }
        }
    }

    set_capability(&state, cx);
    cx.set_global(state);
}

#[cfg(not(target_os = "macos"))]
pub fn initialize(cx: &mut App) {
    let state = ShortcutState {
        enabled: false,
        registered: false,
        accelerator: "Alt+Space".to_string(),
        last_error: Some("global shortcut service currently configured for macOS".to_string()),
    };
    set_capability(&state, cx);
    cx.set_global(state);
}

pub fn snapshot(cx: &App) -> ShortcutState {
    cx.try_global::<ShortcutState>()
        .cloned()
        .unwrap_or_default()
}

fn set_capability(state: &ShortcutState, cx: &mut App) {
    crate::capabilities::set(
        "global_shortcuts",
        crate::capabilities::CapabilityStatus {
            supported: cfg!(target_os = "macos"),
            enabled: state.registered,
            degraded: state.enabled && !state.registered,
            reason: state
                .last_error
                .as_ref()
                .map(|error| format!("shortcut unavailable: {error}").into())
                .or_else(|| {
                    if !cfg!(target_os = "macos") {
                        Some("global shortcut service currently configured for macOS".into())
                    } else if !state.enabled {
                        Some("disabled by user setting".into())
                    } else {
                        None
                    }
                }),
            last_error: state.last_error.clone().map(Into::into),
        },
        cx,
    );
}
