# QA Matrix

This matrix validates desktop boilerplate behavior across platforms.

## Core Startup And Lifecycle

| Case | macOS | Windows | Linux | Expected |
|---|---|---|---|---|
| App cold start | ☐ | ☐ | ☐ | Window opens, lifecycle reaches `Running` |
| Graceful quit | ☐ | ☐ | ☐ | Lifecycle enters `ShuttingDown`, tasks cancelled, app exits cleanly |
| Panic capture smoke test | ☐ | ☐ | ☐ | Panic summary appears in diagnostics/logs |

## Routing / Deep Links / Single Instance

| Case | macOS | Windows | Linux | Expected |
|---|---|---|---|---|
| Internal navigation | ☐ | ☐ | ☐ | Sidebar/launcher/menu navigate to same routes |
| Deep link parse valid | ☐ | ☐ | ☐ | `gpui-starter://settings` opens Settings |
| Deep link parse invalid | ☐ | ☐ | ☐ | Error logged, app does not crash |
| Second-instance forwarding | ☐ | ☐ | ☐ | Second launch forwards payload and exits |

## Notifications (Local + In-App)

| Case | macOS | Windows | Linux | Expected |
|---|---|---|---|---|
| Test local notification | ☐ | ☐ | ☐ | Native delivery attempt logged |
| Notification inbox persistence | ☐ | ☐ | ☐ | Inbox survives relaunch |
| Action/reply fallback | ☐ | ☐ | ☐ | Unsupported features degrade safely with logs |

## Runtime Boundaries

| Case | macOS | Windows | Linux | Expected |
|---|---|---|---|---|
| Connectivity probe | ☐ | ☐ | ☐ | State updates online/offline with error detail |
| Secure storage set/get/delete | ☐ | ☐ | ☐ | Operations succeed or degrade with clear reason |
| Storage boot migration | ☐ | ☐ | ☐ | SQLite schema initialized and version visible |
| Telemetry mode switch | ☐ | ☐ | ☐ | Disabled/local/remote mode reflected in diagnostics |
| Secure storage unavailable path | ☐ | ☐ | ☐ | Unavailable backend degrades with visible reason |

## Observability

| Case | macOS | Windows | Linux | Expected |
|---|---|---|---|---|
| File logging enabled | ☐ | ☐ | ☐ | Log path/file prefix visible in diagnostics |
| Capability registry | ☐ | ☐ | ☐ | Degraded states include reason/error |
| Command availability | ☐ | ☐ | ☐ | Disabled reason visible in diagnostics |
| Open logs folder | ☐ | ☐ | ☐ | System file manager opens app log directory |

## Accessibility

| Case | macOS | Windows | Linux | Expected |
|---|---|---|---|---|
| Keyboard-only traversal | ☐ | ☐ | ☐ | Full nav and actions without mouse |
| Focus visibility | ☐ | ☐ | ☐ | Active focus always visible |
| Menu accessibility smoke | ☐ | ☐ | ☐ | Standard menu commands reachable |

## Automation Gates

- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
