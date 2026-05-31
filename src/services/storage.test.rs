use tempfile::tempdir;

use super::{SqliteStorage, StorageBackend, init_db};

#[test]
fn initializes_schema_and_migration_table() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("app.db");
    let version = init_db(&db_path).expect("init db");
    assert_eq!(version, 2);

    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = 2",
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
    assert_eq!(backend.schema_version().expect("schema version"), 2);
}
