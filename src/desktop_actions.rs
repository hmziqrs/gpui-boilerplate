use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use arboard::Clipboard;
use gpui::{App, Global};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Clone, Debug)]
pub struct DesktopActionsSnapshot {
    pub clipboard_available: bool,
    pub picker_available: bool,
    pub opener_available: bool,
    pub active_watchers: usize,
    pub last_error: Option<String>,
}

impl Default for DesktopActionsSnapshot {
    fn default() -> Self {
        Self {
            clipboard_available: false,
            picker_available: true,
            opener_available: true,
            active_watchers: 0,
            last_error: None,
        }
    }
}

#[derive(Clone)]
pub struct DesktopActionsState {
    snapshot: DesktopActionsSnapshot,
    inner: Arc<Mutex<DesktopActionsInner>>,
}

struct DesktopActionsInner {
    next_watcher_id: u64,
    watchers: BTreeMap<u64, RecommendedWatcher>,
}

impl Global for DesktopActionsState {}

pub fn initialize(cx: &mut App) {
    let mut snapshot = DesktopActionsSnapshot {
        clipboard_available: Clipboard::new().is_ok(),
        ..DesktopActionsSnapshot::default()
    };
    if !snapshot.clipboard_available {
        snapshot.last_error = Some("clipboard backend unavailable".to_string());
    }
    crate::capabilities::set(
        "desktop_actions",
        crate::capabilities::CapabilityStatus {
            supported: true,
            enabled: true,
            degraded: snapshot.last_error.is_some(),
            reason: snapshot.last_error.clone().map(Into::into),
            last_error: snapshot.last_error.clone().map(Into::into),
        },
        cx,
    );
    cx.set_global(DesktopActionsState {
        snapshot,
        inner: Arc::new(Mutex::new(DesktopActionsInner {
            next_watcher_id: 1,
            watchers: BTreeMap::new(),
        })),
    });
}

pub fn snapshot(cx: &App) -> DesktopActionsSnapshot {
    cx.try_global::<DesktopActionsState>()
        .map(|s| s.snapshot.clone())
        .unwrap_or_default()
}

pub fn copy_text(text: &str, cx: &mut App) -> Result<(), String> {
    let result = Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text.to_string()))
        .map_err(|err| err.to_string());
    update_result("clipboard_copy", result.as_ref().err().cloned(), cx);
    result
}

pub fn copy_diagnostics(cx: &mut App) -> Result<(), String> {
    let diagnostics = build_diagnostics_text(cx);
    copy_text(&diagnostics, cx)
}

pub fn open_logs_folder(cx: &mut App) -> Result<(), String> {
    let path = crate::app_state::paths(cx).log_dir;
    open_path(path, cx)
}

pub fn open_config_folder(cx: &mut App) -> Result<(), String> {
    let path = crate::app_state::paths(cx).config_dir;
    open_path(path, cx)
}

pub fn open_url(url: &str, cx: &mut App) -> Result<(), String> {
    let result = open::that_detached(url).map_err(|err| err.to_string());
    update_result("open_url", result.as_ref().err().cloned(), cx);
    result
}

pub fn pick_file(cx: &mut App) -> Option<PathBuf> {
    let file = rfd::FileDialog::new().pick_file();
    tracing::info!(target: "gpui_starter::desktop_actions", file = ?file, "file picker result");
    update_result("pick_file", None, cx);
    file
}

pub fn pick_folder(cx: &mut App) -> Option<PathBuf> {
    let folder = rfd::FileDialog::new().pick_folder();
    tracing::info!(target: "gpui_starter::desktop_actions", folder = ?folder, "folder picker result");
    update_result("pick_folder", None, cx);
    folder
}

pub fn save_file(cx: &mut App) -> Option<PathBuf> {
    let file = rfd::FileDialog::new().save_file();
    tracing::info!(target: "gpui_starter::desktop_actions", file = ?file, "save file picker result");
    update_result("save_file", None, cx);
    file
}

pub fn watch_path(path: PathBuf, cx: &mut App) -> Result<u64, String> {
    let state = cx
        .try_global::<DesktopActionsState>()
        .cloned()
        .ok_or_else(|| "desktop actions unavailable".to_string())?;
    let mut inner = state
        .inner
        .lock()
        .map_err(|_| "watcher lock poisoned".to_string())?;
    let watcher_id = inner.next_watcher_id;
    inner.next_watcher_id += 1;

    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<notify::Event>| match result {
            Ok(event) => tracing::debug!(
                target: "gpui_starter::desktop_actions",
                watcher_id,
                kind = ?event.kind,
                paths = ?event.paths,
                "watch event"
            ),
            Err(err) => tracing::warn!(
                target: "gpui_starter::desktop_actions",
                watcher_id,
                error = %err,
                "watch event error"
            ),
        })
        .map_err(|err| err.to_string())?;
    watcher
        .watch(&path, RecursiveMode::NonRecursive)
        .map_err(|err| err.to_string())?;
    inner.watchers.insert(watcher_id, watcher);
    drop(inner);
    let mut next = state;
    next.snapshot.active_watchers = watcher_count(&next);
    next.snapshot.last_error = None;
    cx.set_global(next);
    tracing::info!(target: "gpui_starter::desktop_actions", watcher_id, path = %path.display(), "watcher registered");
    Ok(watcher_id)
}

pub fn shutdown(cx: &mut App) {
    if let Some(state) = cx.try_global::<DesktopActionsState>().cloned() {
        if let Ok(mut inner) = state.inner.lock() {
            inner.watchers.clear();
        }
        let mut next = state;
        next.snapshot.active_watchers = 0;
        cx.set_global(next);
    }
}

pub fn watch_log_dir(cx: &mut App) -> Result<u64, String> {
    let path = crate::app_state::paths(cx).log_dir;
    watch_path(path, cx)
}

pub fn watch_config_dir(cx: &mut App) -> Result<u64, String> {
    let path = crate::app_state::paths(cx).config_dir;
    watch_path(path, cx)
}

pub fn unwatch_path(id: u64, cx: &mut App) -> bool {
    let state = match cx.try_global::<DesktopActionsState>().cloned() {
        Some(state) => state,
        None => return false,
    };
    let mut removed = false;
    if let Ok(mut inner) = state.inner.lock() {
        removed = inner.watchers.remove(&id).is_some();
    }
    let mut next = state;
    next.snapshot.active_watchers = watcher_count(&next);
    cx.set_global(next);
    removed
}

pub fn unwatch_all(cx: &mut App) -> usize {
    let state = match cx.try_global::<DesktopActionsState>().cloned() {
        Some(state) => state,
        None => return 0,
    };
    let watcher_ids = state
        .inner
        .lock()
        .ok()
        .map(|inner| inner.watchers.keys().copied().collect::<Vec<_>>())
        .unwrap_or_default();
    let count = watcher_ids.len();
    for watcher_id in watcher_ids {
        let _ = unwatch_path(watcher_id, cx);
    }
    count
}

fn build_diagnostics_text(cx: &App) -> String {
    let app_state = crate::app_state::config(cx);
    let lifecycle = cx
        .try_global::<crate::lifecycle::LifecycleState>()
        .cloned()
        .unwrap_or_else(crate::lifecycle::LifecycleState::starting);
    let connectivity = crate::connectivity::snapshot(cx);
    format!(
        "app={} version={} route={} lifecycle={:?} connectivity={:?}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        app_state.active_route.to_url(),
        lifecycle.stage,
        connectivity.state
    )
}

pub fn open_path(path: PathBuf, cx: &mut App) -> Result<(), String> {
    let result = open::that_detached(path.as_path()).map_err(|err| err.to_string());
    update_result("open_path", result.as_ref().err().cloned(), cx);
    result
}

fn update_result(action: &str, error: Option<String>, cx: &mut App) {
    let mut state = match cx.try_global::<DesktopActionsState>().cloned() {
        Some(state) => state,
        None => return,
    };
    state.snapshot.last_error = error.clone();
    state.snapshot.active_watchers = watcher_count(&state);
    cx.set_global(state);
    if let Some(error) = error {
        tracing::warn!(
            target: "gpui_starter::desktop_actions",
            action,
            error = %error,
            "desktop action failed"
        );
    } else {
        tracing::debug!(
            target: "gpui_starter::desktop_actions",
            action,
            "desktop action succeeded"
        );
    }
}

fn watcher_count(state: &DesktopActionsState) -> usize {
    state
        .inner
        .lock()
        .ok()
        .map(|inner| inner.watchers.len())
        .unwrap_or(0)
}
