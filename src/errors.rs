#![allow(dead_code)]

use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppErrorSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to initialize app paths")]
    PathInitialization,
    #[error("failed to read app state from {path}: {details}")]
    StateRead { path: PathBuf, details: String },
    #[error("failed to parse app state from {path}: {details}")]
    StateParse { path: PathBuf, details: String },
    #[error("failed to write app state to {path}: {details}")]
    StateWrite { path: PathBuf, details: String },
    #[error("invalid deep link `{input}`: {reason}")]
    InvalidDeepLink { input: String, reason: String },
    #[error("io error: {0}")]
    Io(String),
}

impl AppError {
    pub fn severity(&self) -> AppErrorSeverity {
        match self {
            Self::InvalidDeepLink { .. } | Self::StateParse { .. } => AppErrorSeverity::Warning,
            Self::PathInitialization
            | Self::StateRead { .. }
            | Self::StateWrite { .. }
            | Self::Io(_) => AppErrorSeverity::Error,
        }
    }

    pub fn io(err: io::Error) -> Self {
        Self::Io(err.to_string())
    }
}
