#![allow(dead_code)]

use gpui::{App, Global};
use serde::{Deserialize, Serialize};

use crate::{ids::EventId, time::AppTimestamp};

// ---------------------------------------------------------------------------
// Error categories
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCategory {
    Network,
    Storage,
    Rendering,
    Config,
    System,
}

impl ErrorCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::Storage => "storage",
            Self::Rendering => "rendering",
            Self::Config => "config",
            Self::System => "system",
        }
    }
}

// ---------------------------------------------------------------------------
// Error actions and records
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorAction {
    Retry,
    OpenSettings,
    CopyDetails,
    Dismiss,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub id: EventId,
    pub occurred_at: AppTimestamp,
    pub severity: crate::errors::AppErrorSeverity,
    pub category: ErrorCategory,
    pub message: String,
    pub actions: Vec<ErrorAction>,
}

// ---------------------------------------------------------------------------
// In-memory error surface state (primary store, capped at 200)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct ErrorSurfaceState {
    pub records: Vec<ErrorRecord>,
}

impl Global for ErrorSurfaceState {}

pub fn initialize(cx: &mut App) {
    cx.set_global(ErrorSurfaceState::default());
}

pub fn report(
    message: impl Into<String>,
    severity: crate::errors::AppErrorSeverity,
    category: ErrorCategory,
    actions: Vec<ErrorAction>,
    cx: &mut App,
) -> EventId {
    let mut state = cx
        .try_global::<ErrorSurfaceState>()
        .cloned()
        .unwrap_or_default();
    let record = ErrorRecord {
        id: EventId::new(),
        occurred_at: AppTimestamp::now(),
        severity,
        category,
        message: message.into(),
        actions,
    };
    let id = record.id;
    state.records.insert(0, record.clone());
    if state.records.len() > 200 {
        state.records.truncate(200);
    }
    cx.set_global(state);

    // Persist to SQLite as secondary store (best-effort, non-blocking).
    if let Some(runtime) = cx.try_global::<crate::storage::StorageRuntime>() {
        if let Err(err) = persist_error(&*runtime.backend, &record) {
            tracing::warn!(
                target: "gpui_starter::error_surface",
                error = %err,
                "failed to persist error to sqlite"
            );
        }
    }

    id
}

pub fn snapshot(cx: &App) -> Vec<ErrorRecord> {
    cx.try_global::<ErrorSurfaceState>()
        .map(|state| state.records.clone())
        .unwrap_or_default()
}

pub fn latest(cx: &App) -> Option<ErrorRecord> {
    snapshot(cx).into_iter().next()
}

pub fn dismiss(id: EventId, cx: &mut App) {
    let mut state = cx
        .try_global::<ErrorSurfaceState>()
        .cloned()
        .unwrap_or_default();
    state.records.retain(|record| record.id != id);
    cx.set_global(state);
}

// ---------------------------------------------------------------------------
// SQLite persistence (secondary store)
// ---------------------------------------------------------------------------

/// Persist a single error record to the `error_log` table.
///
/// The `error_log` table migration is defined in `db_migrations` (version 2).
/// If the table does not exist yet this call will fail and the in-memory store
/// remains authoritative.
pub fn persist_error(
    db: &dyn crate::storage::StorageBackend,
    error: &ErrorRecord,
) -> rusqlite::Result<()> {
    db.persist_error_record(error)
}

/// Load the most recent `limit` error records from SQLite, ordered by
/// occurrence time descending (newest first).
pub fn load_error_history(
    db: &dyn crate::storage::StorageBackend,
    limit: usize,
) -> rusqlite::Result<Vec<ErrorRecord>> {
    db.load_error_history(limit)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_category_labels() {
        assert_eq!(ErrorCategory::Network.label(), "network");
        assert_eq!(ErrorCategory::Storage.label(), "storage");
        assert_eq!(ErrorCategory::Rendering.label(), "rendering");
        assert_eq!(ErrorCategory::Config.label(), "config");
        assert_eq!(ErrorCategory::System.label(), "system");
    }

    #[test]
    fn error_record_serializes_with_category() {
        let record = ErrorRecord {
            id: EventId::new(),
            occurred_at: AppTimestamp::now(),
            severity: crate::errors::AppErrorSeverity::Warning,
            category: ErrorCategory::Network,
            message: "connection timed out".into(),
            actions: vec![ErrorAction::Retry, ErrorAction::Dismiss],
        };
        let json = serde_json::to_string(&record).expect("serialize");
        assert!(json.contains("Network"));
        let deserialized: ErrorRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.category, ErrorCategory::Network);
    }

    #[test]
    fn persist_and_load_roundtrip() {
        use crate::storage::{SqliteStorage, StorageBackend};

        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("test_errors.db");

        // Create the error_log table inline (mirrors migration v2).
        {
            let conn = rusqlite::Connection::open(&db_path).expect("open");
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS error_log (
                    id TEXT PRIMARY KEY,
                    occurred_at TEXT NOT NULL,
                    severity TEXT NOT NULL,
                    category TEXT NOT NULL,
                    message TEXT NOT NULL,
                    actions TEXT NOT NULL
                );",
            )
            .expect("create table");
        }

        let backend = SqliteStorage::new_for_test(db_path);

        let record = ErrorRecord {
            id: EventId::new(),
            occurred_at: AppTimestamp::now(),
            severity: crate::errors::AppErrorSeverity::Error,
            category: ErrorCategory::Storage,
            message: "disk full".into(),
            actions: vec![ErrorAction::Dismiss],
        };

        persist_error(&backend, &record).expect("persist");
        let loaded = load_error_history(&backend, 10).expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].message, "disk full");
        assert_eq!(loaded[0].category, ErrorCategory::Storage);
        assert_eq!(loaded[0].severity, crate::errors::AppErrorSeverity::Error);
    }

    #[test]
    fn load_error_history_respects_limit() {
        use crate::storage::{SqliteStorage, StorageBackend};

        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("test_limit.db");

        {
            let conn = rusqlite::Connection::open(&db_path).expect("open");
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS error_log (
                    id TEXT PRIMARY KEY,
                    occurred_at TEXT NOT NULL,
                    severity TEXT NOT NULL,
                    category TEXT NOT NULL,
                    message TEXT NOT NULL,
                    actions TEXT NOT NULL
                );",
            )
            .expect("create table");
        }

        let backend = SqliteStorage::new_for_test(db_path);

        for i in 0..5 {
            let record = ErrorRecord {
                id: EventId::new(),
                occurred_at: AppTimestamp::now(),
                severity: crate::errors::AppErrorSeverity::Info,
                category: ErrorCategory::System,
                message: format!("error {i}"),
                actions: vec![],
            };
            persist_error(&backend, &record).expect("persist");
        }

        let loaded = load_error_history(&backend, 3).expect("load");
        assert_eq!(loaded.len(), 3);
    }
}
