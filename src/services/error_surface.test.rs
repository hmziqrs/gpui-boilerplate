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
    use crate::storage::SqliteStorage;

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
    use crate::storage::SqliteStorage;

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
