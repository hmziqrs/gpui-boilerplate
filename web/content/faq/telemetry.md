---
question: "Does gpui-starter collect any telemetry?"
description: "No. Telemetry is disabled by default with opt-in local-only logging available."
category: "Advanced"
order: 18
---

No. Telemetry ships in the **disabled** state and stays that way until you change it. Nothing leaves the app without an explicit opt-in.

## Telemetry modes

The telemetry module in `src/telemetry.rs` supports three modes:

- Disabled (default): all events are discarded. No data is recorded or sent anywhere.
- Local only: events are written to the local log stream via `tracing`. They never leave the machine.
- Remote: events are forwarded to an endpoint you configure. This is the only mode that sends data externally, and it requires both consent and a valid endpoint URL.

## Consent gate

Every mode except `Disabled` requires `consented = true` in the call to `set_mode`. If consent is `false`, the sink is replaced with a no-op regardless of the configured mode. There is no path where data is collected without the user actively opting in.

## Module boundary

All telemetry logic lives behind the `TelemetrySink` trait. The rest of the codebase calls `record_event` and `record_error` through that interface and never touches the underlying sink directly. To remove telemetry entirely, delete the `src/telemetry.rs` module and the initialization call in `app.rs`.
