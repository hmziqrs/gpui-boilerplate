use anyhow::Result;
use rusqlite::Connection;

struct Migration {
    version: u32,
    name: &'static str,
    up_sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "kv_store",
        up_sql: r#"
            CREATE TABLE IF NOT EXISTS kv_store (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
        "#,
    },
    Migration {
        version: 2,
        name: "error_log",
        up_sql: r#"
            CREATE TABLE IF NOT EXISTS error_log (
                id TEXT PRIMARY KEY,
                occurred_at TEXT NOT NULL,
                severity TEXT NOT NULL,
                category TEXT NOT NULL,
                message TEXT NOT NULL,
                actions TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_error_log_occurred_at
                ON error_log (occurred_at DESC);
        "#,
    },
];

pub fn run_migrations(conn: &Connection) -> Result<u32> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )?;

    let current_version: u32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    tracing::info!(
        target: "gpui_starter::db_migrations",
        current_version,
        pending = MIGRATIONS.len().saturating_sub(current_version as usize),
        "checking migrations"
    );

    let mut applied = current_version;

    for migration in MIGRATIONS {
        if migration.version <= current_version {
            continue;
        }

        tracing::info!(
            target: "gpui_starter::db_migrations",
            version = migration.version,
            name = migration.name,
            "applying migration"
        );

        let tx = conn.unchecked_transaction()?;

        tx.execute_batch(migration.up_sql)?;

        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, datetime('now'))",
            [migration.version],
        )?;

        tx.commit()?;

        tracing::info!(
            target: "gpui_starter::db_migrations",
            version = migration.version,
            name = migration.name,
            "migration applied"
        );

        applied = migration.version;
    }

    tracing::info!(
        target: "gpui_starter::db_migrations",
        schema_version = applied,
        "migrations complete"
    );

    Ok(applied)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn creates_schema_table_from_empty_db() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let conn = Connection::open(&db_path).expect("open db");

        let version = run_migrations(&conn).expect("run migrations");
        assert_eq!(version, 2);

        let recorded: u32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .expect("read version");
        assert_eq!(recorded, 2);
    }

    #[test]
    fn is_idempotent() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");

        let conn = Connection::open(&db_path).expect("open db");
        let first = run_migrations(&conn).expect("first run");
        let second = run_migrations(&conn).expect("second run");
        assert_eq!(first, second);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("count rows");
        assert_eq!(count, 2);
    }

    #[test]
    fn kv_store_table_is_usable() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let conn = Connection::open(&db_path).expect("open db");

        run_migrations(&conn).expect("run migrations");

        conn.execute(
            "INSERT INTO kv_store (key, value, updated_at) VALUES ('k', 'v', datetime('now'))",
            [],
        )
        .expect("insert");

        let value: String = conn
            .query_row("SELECT value FROM kv_store WHERE key = 'k'", [], |row| {
                row.get(0)
            })
            .expect("select");
        assert_eq!(value, "v");
    }

    #[test]
    fn error_log_table_is_usable() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let conn = Connection::open(&db_path).expect("open db");

        run_migrations(&conn).expect("run migrations");

        conn.execute(
            "INSERT INTO error_log (id, occurred_at, severity, category, message, actions)
             VALUES ('test-id', '2025-01-01T00:00:00Z', 'Error', 'Network', 'timeout', \"[]\")",
            [],
        )
        .expect("insert");

        let message: String = conn
            .query_row(
                "SELECT message FROM error_log WHERE id = 'test-id'",
                [],
                |row| row.get(0),
            )
            .expect("select");
        assert_eq!(message, "timeout");
    }
}
