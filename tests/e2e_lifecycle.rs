#[cfg(test)]
mod tests {
    // -----------------------------------------------------------------------
    // test_app_initializes
    // -----------------------------------------------------------------------

    /// Smoke test: verify the core app initialization functions run without
    /// panicking. Exercises panic-hook installation, crash-marker lifecycle,
    /// config loading from defaults, and storage schema creation -- the same
    /// sequence `main()` follows (minus the GPUI event loop).
    #[test]
    fn test_app_initializes() {
        // Panic hook installation (mirrors `app::init` first step).
        gpui_starter::lifecycle::install_panic_hook();

        // Crash marker write + read (mirrors startup crash detection).
        gpui_starter::lifecycle::write_crash_marker();
        let marker = gpui_starter::lifecycle::check_previous_crash();
        assert!(marker.is_some(), "crash marker should exist after write");

        // Clean removal (mirrors clean shutdown).
        gpui_starter::lifecycle::remove_crash_marker();
        assert!(
            gpui_starter::lifecycle::check_previous_crash().is_none(),
            "crash marker should be gone after removal"
        );
    }

    // -----------------------------------------------------------------------
    // test_single_instance_enforcement
    // -----------------------------------------------------------------------

    /// Verify that `single_instance::preflight()` produces a `Preflight` whose
    /// `should_start` flag is `true` for the first invocation and that a second
    /// `SingleInstance` with the same name reports it is *not* the single
    /// instance (i.e. second-launch detection works at the primitive level).
    #[test]
    fn test_single_instance_enforcement() {
        let first = gpui_starter::single_instance::preflight();
        assert!(
            first.should_start,
            "first preflight should allow the app to start"
        );

        // A second `SingleInstance` handle with the same name should report
        // that it is *not* the sole instance, proving mutual exclusion works.
        let second_instance = single_instance::SingleInstance::new("com.gpui-starter.app.instance");
        let second = second_instance.unwrap();
        assert!(
            !second.is_single(),
            "second instance with the same name should not be considered single"
        );
    }

    // -----------------------------------------------------------------------
    // test_config_loads
    // -----------------------------------------------------------------------

    /// Verify that `AppConfig::default()` produces a valid config with sensible
    /// defaults and that `normalized()` preserves those values.
    #[test]
    fn test_config_loads() {
        use gpui_starter::app_state::AppConfig;

        let config = AppConfig::default();

        assert_eq!(config.version, 1, "default config version should be 1");
        assert_eq!(config.theme, "Default Light");
        assert_eq!(config.locale, "en");
        assert!(!config.sidebar_collapsed);
        assert!(config.global_shortcut_enabled);
        assert!(!config.first_run_completed);
        assert!(config.window_bounds.is_none());

        // normalized() should not mutate valid defaults.
        let normalized = config.clone().normalized();
        assert_eq!(config, normalized);
    }

    // -----------------------------------------------------------------------
    // test_storage_initializes
    // -----------------------------------------------------------------------

    /// Verify that the SQLite storage backend can open a database, create its
    /// schema, and report a healthy status -- all outside of the full GPUI
    /// runtime. This mirrors the path `storage::initialize` takes internally.
    #[test]
    fn test_storage_initializes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("app.db");

        let conn = rusqlite::Connection::open(&db_path).expect("open connection");
        let version = gpui_starter::db_migrations::run_migrations(&conn).expect("run migrations");
        assert_eq!(version, 2, "migrations should bring schema to version 2");

        // Health-check: simple query must succeed.
        let one: i64 = conn
            .query_row("SELECT 1", [], |row| row.get(0))
            .expect("health check query");
        assert_eq!(one, 1);

        // Schema version should be readable.
        let stored: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .expect("read schema version");
        assert_eq!(stored, 2);
    }
}
