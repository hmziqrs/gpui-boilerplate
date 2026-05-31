#![allow(dead_code)]

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::errors::AppError;

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub state_file: PathBuf,
}

impl AppPaths {
    pub fn new() -> Result<Self, AppError> {
        let project_dirs = ProjectDirs::from("com", "gpui-starter", "GPUI Starter")
            .ok_or(AppError::PathInitialization)?;

        let config_dir = project_dirs.config_dir().to_path_buf();
        let data_dir = project_dirs.data_dir().to_path_buf();
        let cache_dir = project_dirs.cache_dir().to_path_buf();
        let log_dir = data_dir.join("logs");
        let runtime_dir = cache_dir.join("runtime");
        let state_file = config_dir.join("state.json");

        for dir in [&config_dir, &data_dir, &cache_dir, &log_dir, &runtime_dir] {
            std::fs::create_dir_all(dir)?;
        }

        Ok(Self {
            config_dir,
            data_dir,
            cache_dir,
            log_dir,
            runtime_dir,
            state_file,
        })
    }
}

pub fn ensure_parent_dir(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
