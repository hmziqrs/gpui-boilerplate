---
question: "Is there a diagnostics or debug page?"
description: "gpui-starter includes a diagnostics page showing app state, logs, and system information."
category: "Advanced"
order: 17
---

Yes. gpui-starter ships with a built-in diagnostics page that gives you a live snapshot of the app's internal state. You can open it from the sidebar by clicking the **Diagnostics** entry (the info icon).

## What the diagnostics page shows

The page displays a table of key/value rows covering most subsystems in the app:

App info shows the package name, version, and active route. Lifecycle state tracks whether the app is starting, running, shutting down, or has crashed. Connectivity reports the network probe URL, connection status, and last error. Storage displays the database path, schema version, health status, and recent migration results.

Telemetry shows whether it is compiled in, consented, and enabled, plus event counts. Notifications reports the active backend, permission status, and degraded reasons. Secure storage shows availability and errors. Session state displays the current session representation.

Commands lists the registered command count and per-command availability. Accessibility shows AccessKit linkage and bridge status. Desktop actions reports clipboard, picker, and opener availability. Undo/redo stack shows stack sizes and the last action. Error surface displays error count and the latest error message. File paths lists config, data, cache, and log directories. Capabilities shows per-capability supported/enabled/degraded status.

## Actions on the page

The diagnostics page includes buttons to **Refresh** the view, **Copy Diagnostics** to the clipboard, **Open Logs Folder** in Finder, **Reset First-Run** state, and **Dismiss Latest Error**. In debug builds, there is also a button to trigger a test panic for crash handling development.
