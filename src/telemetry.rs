use std::sync::Arc;
#[cfg(test)]
use std::sync::Mutex;

use gpui::{App, Global};
use opentelemetry::global;
use tracing_opentelemetry as _;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TelemetryMode {
    Disabled,
    LocalOnly,
    Remote,
}

#[derive(Clone, Debug)]
pub struct TelemetrySnapshot {
    pub compiled: bool,
    pub consented: bool,
    pub enabled: bool,
    pub mode: TelemetryMode,
    pub endpoint_redacted: Option<String>,
    pub events_recorded: u64,
    pub last_export_error: Option<String>,
    pub last_error: Option<String>,
}

impl Default for TelemetrySnapshot {
    fn default() -> Self {
        Self {
            compiled: true,
            consented: false,
            enabled: false,
            mode: TelemetryMode::Disabled,
            endpoint_redacted: None,
            events_recorded: 0,
            last_export_error: None,
            last_error: None,
        }
    }
}

impl Global for TelemetrySnapshot {}

pub trait TelemetrySink: Send + Sync {
    fn record_event(&self, name: &str) -> Result<(), String>;
    fn record_error(&self, error: &str) -> Result<(), String>;
    fn set_user_properties(&self, key: &str, value: &str) -> Result<(), String>;
    fn flush(&self) -> Result<(), String>;
}

#[derive(Default)]
struct DisabledSink;

impl TelemetrySink for DisabledSink {
    fn record_event(&self, _name: &str) -> Result<(), String> {
        Ok(())
    }

    fn record_error(&self, _error: &str) -> Result<(), String> {
        Ok(())
    }

    fn set_user_properties(&self, _key: &str, _value: &str) -> Result<(), String> {
        Ok(())
    }

    fn flush(&self) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Default)]
struct LocalSink;

impl TelemetrySink for LocalSink {
    fn record_event(&self, name: &str) -> Result<(), String> {
        tracing::debug!(target: "gpui_starter::telemetry", event = %name, "local telemetry event");
        Ok(())
    }

    fn record_error(&self, error: &str) -> Result<(), String> {
        tracing::warn!(target: "gpui_starter::telemetry", error = %error, "local telemetry error");
        Ok(())
    }

    fn set_user_properties(&self, key: &str, value: &str) -> Result<(), String> {
        tracing::debug!(target: "gpui_starter::telemetry", key = %key, value = %value, "local telemetry user property");
        Ok(())
    }

    fn flush(&self) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Clone)]
struct RemoteSink {
    endpoint: String,
}

impl TelemetrySink for RemoteSink {
    fn record_event(&self, name: &str) -> Result<(), String> {
        tracing::debug!(target: "gpui_starter::telemetry", endpoint = %self.endpoint, event = %name, "remote telemetry event queued");
        Ok(())
    }

    fn record_error(&self, error: &str) -> Result<(), String> {
        tracing::warn!(target: "gpui_starter::telemetry", endpoint = %self.endpoint, error = %error, "remote telemetry error queued");
        Ok(())
    }

    fn set_user_properties(&self, key: &str, value: &str) -> Result<(), String> {
        tracing::debug!(target: "gpui_starter::telemetry", endpoint = %self.endpoint, key = %key, value = %value, "remote telemetry user property queued");
        Ok(())
    }

    fn flush(&self) -> Result<(), String> {
        tracing::debug!(target: "gpui_starter::telemetry", endpoint = %self.endpoint, "remote telemetry flush");
        Ok(())
    }
}

#[derive(Clone)]
struct TelemetryRuntime {
    sink: Arc<dyn TelemetrySink>,
}

impl Global for TelemetryRuntime {}

pub fn initialize(cx: &mut App) {
    let snapshot = TelemetrySnapshot::default();
    let runtime = TelemetryRuntime {
        sink: Arc::new(DisabledSink),
    };
    set_capability(&snapshot, cx);
    cx.set_global(snapshot);
    cx.set_global(runtime);
}

pub fn snapshot(cx: &App) -> TelemetrySnapshot {
    cx.try_global::<TelemetrySnapshot>()
        .cloned()
        .unwrap_or_default()
}

pub fn set_mode(mode: TelemetryMode, consented: bool, endpoint: Option<&str>, cx: &mut App) {
    let endpoint_redacted = endpoint.and_then(redact_endpoint);
    let enabled = consented && mode != TelemetryMode::Disabled;

    let sink: Arc<dyn TelemetrySink> = match (mode.clone(), consented) {
        (TelemetryMode::Disabled, _) | (_, false) => Arc::new(DisabledSink),
        (TelemetryMode::LocalOnly, true) => Arc::new(LocalSink),
        (TelemetryMode::Remote, true) => Arc::new(RemoteSink {
            endpoint: endpoint.unwrap_or("https://telemetry.invalid").to_string(),
        }),
    };

    let next = TelemetrySnapshot {
        compiled: true,
        consented,
        enabled,
        mode: mode.clone(),
        endpoint_redacted,
        events_recorded: snapshot(cx).events_recorded,
        last_export_error: None,
        last_error: None,
    };

    tracing::info!(
        target: "gpui_starter::telemetry",
        consented = next.consented,
        enabled = next.enabled,
        mode = ?next.mode,
        endpoint = ?next.endpoint_redacted,
        "telemetry mode updated"
    );

    set_capability(&next, cx);
    cx.set_global(next);
    cx.set_global(TelemetryRuntime { sink });
}

pub fn record_event(name: &str, cx: &mut App) {
    with_runtime(cx, |runtime, cx| {
        let result = runtime.sink.record_event(name);
        handle_record_result(result, cx);
    });
}

pub fn record_error(error: &str, cx: &mut App) {
    with_runtime(cx, |runtime, cx| {
        let result = runtime.sink.record_error(error);
        handle_record_result(result, cx);
    });
}

pub fn set_user_property(key: &str, value: &str, cx: &mut App) {
    with_runtime(cx, |runtime, cx| {
        let result = runtime.sink.set_user_properties(key, value);
        handle_record_result(result, cx);
    });
}

pub fn flush(cx: &mut App) {
    with_runtime(cx, |runtime, cx| {
        let mut snap = snapshot(cx);
        if let Err(err) = runtime.sink.flush() {
            snap.last_export_error = Some(err.clone());
            snap.last_error = Some(err);
        }
        cx.set_global(snap);
    });
}

pub fn shutdown(cx: &mut App) {
    let state = snapshot(cx);
    tracing::debug!(
        target: "gpui_starter::telemetry",
        enabled = state.enabled,
        mode = ?state.mode,
        events_recorded = state.events_recorded,
        "telemetry shutdown requested"
    );
    flush(cx);
    global::shutdown_tracer_provider();
}

fn with_runtime(cx: &mut App, f: impl FnOnce(TelemetryRuntime, &mut App)) {
    if let Some(runtime) = cx.try_global::<TelemetryRuntime>().cloned() {
        f(runtime, cx);
    }
}

fn handle_record_result(result: Result<(), String>, cx: &mut App) {
    let mut snap = snapshot(cx);
    match result {
        Ok(()) => {
            snap.events_recorded = snap.events_recorded.saturating_add(1);
            snap.last_error = None;
        }
        Err(err) => {
            snap.last_export_error = Some(err.clone());
            snap.last_error = Some(err);
        }
    }
    cx.set_global(snap);
}

fn set_capability(snapshot: &TelemetrySnapshot, cx: &mut App) {
    crate::capabilities::set(
        "telemetry",
        crate::capabilities::CapabilityStatus {
            supported: snapshot.compiled,
            enabled: snapshot.enabled,
            degraded: snapshot.last_error.is_some(),
            reason: if !snapshot.consented {
                Some("telemetry disabled until consent".into())
            } else {
                None
            },
            last_error: snapshot.last_error.clone().map(Into::into),
        },
        cx,
    );
}

fn redact_endpoint(endpoint: &str) -> Option<String> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return None;
    }
    let host = endpoint
        .split("://")
        .nth(1)
        .unwrap_or(endpoint)
        .split('/')
        .next()
        .unwrap_or(endpoint);
    Some(format!("{host}/…"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeSink {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl TelemetrySink for FakeSink {
        fn record_event(&self, name: &str) -> Result<(), String> {
            self.events.lock().expect("lock").push(name.to_string());
            Ok(())
        }

        fn record_error(&self, error: &str) -> Result<(), String> {
            self.events
                .lock()
                .expect("lock")
                .push(format!("err:{error}"));
            Ok(())
        }

        fn set_user_properties(&self, key: &str, value: &str) -> Result<(), String> {
            self.events
                .lock()
                .expect("lock")
                .push(format!("prop:{key}={value}"));
            Ok(())
        }

        fn flush(&self) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn redacts_endpoint_host_only() {
        let value =
            redact_endpoint("https://telemetry.example.com/v1/events").expect("redacted endpoint");
        assert_eq!(value, "telemetry.example.com/…");
    }

    #[test]
    fn fake_sink_records_calls() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = FakeSink {
            events: events.clone(),
        };
        sink.record_event("evt").expect("event");
        sink.record_error("oops").expect("error");
        sink.set_user_properties("k", "v").expect("prop");

        let data = events.lock().expect("lock").clone();
        assert_eq!(data, vec!["evt", "err:oops", "prop:k=v"]);
    }
}
