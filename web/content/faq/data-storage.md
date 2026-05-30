---
question: "Where does gpui-starter store app data?"
description: "App data is stored in SQLite with WAL mode, following platform conventions for data directories."
category: "Advanced"
order: 16
---

gpui-starter stores all structured data in a local SQLite database running in WAL (write-ahead logging) mode for safe concurrent reads without blocking the UI thread.

## Platform data directories

The `paths` module uses the `directories` crate to resolve the correct OS-specific locations. On macOS the data directory lives under `~/Library/Application Support/GPUI Starter`, and on Linux it follows the XDG specification. Config, cache, logs, and runtime files each get their own directory, all created automatically on first launch.

## Database schema and migrations

The database file is `app.db` inside the data directory. On initialization, gpui-starter creates a `schema_migrations` table that tracks which migration versions have been applied. Each migration is a numbered step, so the schema can evolve across releases without manual intervention. A `kv_store` table is included for simple key/value persistence.

## App config and blob storage

User preferences (theme, window bounds, locale, sidebar state) are persisted as a JSON file (`state.json`) in the config directory using atomic writes to prevent corruption. If the file becomes unreadable, it gets quarantined with a `.bad` extension and the app falls back to defaults.

For large binary payloads like images or exported files, store them as files in the data directory and keep only the path reference in SQLite. This keeps the database compact and fast to back up.
