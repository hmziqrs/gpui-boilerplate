#![allow(dead_code)]

use std::collections::HashSet;
use std::{io::Write, path::Path};

use atomic_write_file::AtomicWriteFile;
use gpui::{App, Global};
use gpui_component::scroll::ScrollbarShow;
use serde::{Deserialize, Serialize};

use crate::{
    app::{LOCALE_EN, LOCALE_ZH_CN},
    errors::AppError,
    notifications::inbox::NotificationInboxItem,
    paths::{AppPaths, ensure_parent_dir},
    routes::AppRoute,
};

pub const APP_STATE_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct AppState {
    pub paths: AppPaths,
    pub config: AppConfig,
    pub last_load_error: Option<String>,
    pub last_save_error: Option<String>,
}

impl Global for AppState {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub theme: String,
    pub scrollbar_show: Option<ScrollbarShow>,
    pub locale: String,
    pub active_route: AppRoute,
    pub sidebar_collapsed: bool,
    pub native_notifications_enabled: bool,
    #[serde(default = "default_true")]
    pub global_shortcut_enabled: bool,
    pub first_run_completed: bool,
    pub notification_inbox: Vec<NotificationInboxItem>,
    pub window_bounds: Option<PersistedWindowBounds>,
    #[serde(default)]
    pub granted_permissions: HashSet<String>,
    #[serde(default)]
    pub denied_permissions: HashSet<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedWindowBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: APP_STATE_VERSION,
            theme: "Default Light".to_string(),
            scrollbar_show: None,
            locale: LOCALE_EN.to_string(),
            active_route: AppRoute::default(),
            sidebar_collapsed: false,
            native_notifications_enabled: true,
            global_shortcut_enabled: true,
            first_run_completed: false,
            notification_inbox: Vec::new(),
            window_bounds: None,
            granted_permissions: HashSet::new(),
            denied_permissions: HashSet::new(),
        }
    }
}

impl AppConfig {
    pub fn normalized(mut self) -> Self {
        if self.version == 0 {
            self.version = APP_STATE_VERSION;
        }
        if self.locale != LOCALE_EN && self.locale != LOCALE_ZH_CN {
            self.locale = LOCALE_EN.to_string();
        }
        self
    }
}

fn default_true() -> bool {
    true
}

pub fn initialize(cx: &mut App) {
    let paths = match AppPaths::new() {
        Ok(paths) => paths,
        Err(err) => {
            tracing::error!(target: "gpui_starter::app_state", error = %err, "failed to initialize app paths");
            return;
        }
    };

    let (config, last_load_error) = load_config(&paths.state_file);
    tracing::info!(
        target: "gpui_starter::app_state",
        state_file = %paths.state_file.display(),
        config_dir = %paths.config_dir.display(),
        data_dir = %paths.data_dir.display(),
        log_dir = %paths.log_dir.display(),
        last_load_error = ?last_load_error,
        "loaded app state"
    );
    cx.set_global(AppState {
        paths,
        config,
        last_load_error,
        last_save_error: None,
    });
}

pub fn config(cx: &App) -> AppConfig {
    cx.global::<AppState>().config.clone()
}

pub fn paths(cx: &App) -> AppPaths {
    cx.global::<AppState>().paths.clone()
}

pub fn update_config(cx: &mut App, update: impl FnOnce(&mut AppConfig)) {
    let Some(current) = cx.try_global::<AppState>().cloned() else {
        tracing::warn!(target: "gpui_starter::app_state", "attempted to update app state before initialization");
        return;
    };

    let mut next = current.clone();
    update(&mut next.config);
    next.config = next.config.normalized();

    match save_config(&next.paths.state_file, &next.config) {
        Ok(()) => {
            next.last_save_error = None;
            tracing::debug!(
                target: "gpui_starter::app_state",
                state_file = %next.paths.state_file.display(),
                "persisted app state"
            );
        }
        Err(err) => {
            let error = err.to_string();
            tracing::error!(
                target: "gpui_starter::app_state",
                error = %error,
                "failed to persist app state"
            );
            next.last_save_error = Some(error);
        }
    }

    cx.set_global(next);
}

fn load_config(path: &Path) -> (AppConfig, Option<String>) {
    match std::fs::read_to_string(path) {
        Ok(json) => match serde_json::from_str::<AppConfig>(&json) {
            Ok(config) => (crate::config_migrations::migrate(config).normalized(), None),
            Err(err) => {
                quarantine_bad_config(path);
                (
                    AppConfig::default(),
                    Some(
                        AppError::StateParse {
                            path: path.to_path_buf(),
                            details: err.to_string(),
                        }
                        .to_string(),
                    ),
                )
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (AppConfig::default(), None),
        Err(err) => (
            AppConfig::default(),
            Some(
                AppError::StateRead {
                    path: path.to_path_buf(),
                    details: err.to_string(),
                }
                .to_string(),
            ),
        ),
    }
}

fn save_config(path: &Path, config: &AppConfig) -> Result<(), AppError> {
    ensure_parent_dir(path)?;
    let mut file = AtomicWriteFile::options()
        .open(path)
        .map_err(|err| AppError::StateWrite {
            path: path.to_path_buf(),
            details: err.to_string(),
        })?;
    let json = serde_json::to_vec_pretty(config).map_err(|err| AppError::StateWrite {
        path: path.to_path_buf(),
        details: err.to_string(),
    })?;
    file.write_all(&json).map_err(|err| AppError::StateWrite {
        path: path.to_path_buf(),
        details: err.to_string(),
    })?;
    file.write_all(b"\n").map_err(|err| AppError::StateWrite {
        path: path.to_path_buf(),
        details: err.to_string(),
    })?;
    file.commit().map_err(|err| AppError::StateWrite {
        path: path.to_path_buf(),
        details: err.to_string(),
    })?;
    Ok(())
}

fn quarantine_bad_config(path: &Path) {
    if !path.exists() {
        return;
    }
    let quarantine_path = path.with_extension("json.bad");
    if let Err(err) = std::fs::rename(path, &quarantine_path) {
        tracing::warn!(
            target: "gpui_starter::app_state",
            source = %path.display(),
            target_path = %quarantine_path.display(),
            error = %err,
            "failed to quarantine corrupt app state"
        );
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::sidebar::Page;

    #[test]
    fn save_and_load_config_uses_json_state_file() {
        let dir = tempdir().unwrap();
        let state_file = dir.path().join("state.json");
        let config = AppConfig {
            active_route: AppRoute::page(Page::Settings),
            sidebar_collapsed: true,
            ..AppConfig::default()
        };

        save_config(&state_file, &config).unwrap();
        let (loaded, err) = load_config(&state_file);

        assert_eq!(err, None);
        assert_eq!(loaded.active_route, AppRoute::page(Page::Settings));
        assert!(loaded.sidebar_collapsed);
    }

    #[test]
    fn corrupt_config_is_quarantined() {
        let dir = tempdir().unwrap();
        let state_file = dir.path().join("state.json");
        std::fs::write(&state_file, "{not-json").unwrap();

        let (loaded, err) = load_config(&state_file);

        assert_eq!(loaded, AppConfig::default());
        assert!(err.is_some());
        assert!(!state_file.exists());
        assert!(state_file.with_extension("json.bad").exists());
    }
}
