use std::{path::PathBuf, sync::Arc};

use gpui::{App, Global};
use rusqlite::Connection;

#[derive(Clone, Debug, Default)]
pub struct StorageSnapshot {
    pub available: bool,
    pub db_path: String,
    pub schema_version: i64,
    pub healthy: bool,
    pub last_maintenance_at: Option<String>,
    pub last_migration_result: Option<String>,
    pub last_error: Option<String>,
}

impl Global for StorageSnapshot {}

pub trait StorageBackend: Send + Sync {
    fn schema_version(&self) -> rusqlite::Result<i64>;
    fn health_check(&self) -> rusqlite::Result<()>;
    fn maintenance(&self) -> rusqlite::Result<()>;
}

#[derive(Clone, Debug)]
struct SqliteStorage {
    path: PathBuf,
}

impl SqliteStorage {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn open(&self) -> rusqlite::Result<Connection> {
        Connection::open(&self.path)
    }
}

impl StorageBackend for SqliteStorage {
    fn schema_version(&self) -> rusqlite::Result<i64> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
    }

    fn health_check(&self) -> rusqlite::Result<()> {
        let conn = self.open()?;
        let _: i64 = conn.query_row("SELECT 1", [], |row| row.get(0))?;
        Ok(())
    }

    fn maintenance(&self) -> rusqlite::Result<()> {
        let conn = self.open()?;
        conn.execute_batch("PRAGMA optimize;")
    }
}

#[derive(Clone)]
pub struct StorageRuntime {
    backend: Arc<dyn StorageBackend>,
}

impl Global for StorageRuntime {}

pub fn initialize(cx: &mut App) {
    let path = db_path(cx);
    let backend = Arc::new(SqliteStorage::new(path.clone()));
    let mut snapshot = StorageSnapshot {
        db_path: path.display().to_string(),
        ..StorageSnapshot::default()
    };

    match init_db(&path) {
        Ok(schema_version) => {
            snapshot.available = true;
            snapshot.schema_version = schema_version;
            snapshot.last_migration_result =
                Some(format!("schema version {} ready", schema_version));
            tracing::info!(
                target: "gpui_starter::storage",
                db_path = %snapshot.db_path,
                schema_version,
                "storage initialized"
            );
        }
        Err(err) => {
            let error = err.to_string();
            snapshot.last_error = Some(error.clone());
            snapshot.last_migration_result = Some("migration failed".to_string());
            tracing::error!(
                target: "gpui_starter::storage",
                db_path = %snapshot.db_path,
                error = %error,
                "storage initialization failed"
            );
        }
    }

    if snapshot.available {
        match backend.health_check() {
            Ok(()) => snapshot.healthy = true,
            Err(err) => {
                snapshot.healthy = false;
                snapshot.last_error = Some(err.to_string());
            }
        }
    }

    crate::capabilities::set(
        "storage",
        crate::capabilities::CapabilityStatus {
            supported: true,
            enabled: snapshot.available,
            degraded: snapshot.last_error.is_some() || !snapshot.healthy,
            reason: snapshot
                .last_error
                .as_ref()
                .map(|err| format!("storage issue: {err}").into()),
            last_error: snapshot.last_error.clone().map(Into::into),
        },
        cx,
    );

    cx.set_global(snapshot);
    cx.set_global(StorageRuntime { backend });
}

pub fn snapshot(cx: &App) -> StorageSnapshot {
    cx.try_global::<StorageSnapshot>()
        .cloned()
        .unwrap_or_default()
}

pub fn run_health_check(cx: &mut App) {
    let Some(runtime) = cx.try_global::<StorageRuntime>().cloned() else {
        return;
    };
    let mut snap = snapshot(cx);
    match runtime.backend.health_check() {
        Ok(()) => {
            snap.healthy = true;
            snap.last_error = None;
            if let Ok(version) = runtime.backend.schema_version() {
                snap.schema_version = version;
            }
        }
        Err(err) => {
            snap.healthy = false;
            snap.last_error = Some(err.to_string());
        }
    }
    cx.set_global(snap);
}

pub fn run_maintenance(cx: &mut App) {
    let Some(runtime) = cx.try_global::<StorageRuntime>().cloned() else {
        return;
    };
    let mut snap = snapshot(cx);
    match runtime.backend.maintenance() {
        Ok(()) => {
            snap.last_maintenance_at = Some(chrono::Utc::now().to_rfc3339());
            snap.last_error = None;
        }
        Err(err) => {
            snap.last_error = Some(err.to_string());
        }
    }
    cx.set_global(snap);
}

pub fn shutdown(cx: &mut App) {
    let snapshot = snapshot(cx);
    tracing::debug!(
        target: "gpui_starter::storage",
        available = snapshot.available,
        healthy = snapshot.healthy,
        db_path = %snapshot.db_path,
        "storage shutdown requested"
    );
}

fn db_path(cx: &App) -> PathBuf {
    crate::app_state::paths(cx).data_dir.join("app.db")
}

fn init_db(path: &PathBuf) -> rusqlite::Result<i64> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS kv_store (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
    "#,
    )?;

    let current_version = 1_i64;
    conn.execute(
        "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (?1, datetime('now'))",
        [current_version],
    )?;
    Ok(current_version)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{SqliteStorage, StorageBackend, init_db};

    #[test]
    fn initializes_schema_and_migration_table() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("app.db");
        let version = init_db(&db_path).expect("init db");
        assert_eq!(version, 1);

        let conn = rusqlite::Connection::open(&db_path).expect("open db");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
            .expect("read migrations");
        assert_eq!(count, 1);
    }

    #[test]
    fn backend_health_and_maintenance_work() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("app.db");
        init_db(&db_path).expect("init db");
        let backend = SqliteStorage::new(db_path);
        backend.health_check().expect("health check");
        backend.maintenance().expect("maintenance");
        assert_eq!(backend.schema_version().expect("schema version"), 1);
    }
}
