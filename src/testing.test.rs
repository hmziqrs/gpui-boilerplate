use super::*;

#[test]
fn fake_telemetry_tracks_events_and_flush() {
    let mut sink = FakeTelemetrySink::default();
    sink.record_event("start");
    sink.record_error("boom");
    sink.flush();

    assert_eq!(sink.events.len(), 2);
    assert!(sink.flushed);
}

#[test]
fn fake_secure_storage_roundtrip() {
    let mut storage = FakeSecureStorage::default();
    storage.set("token");
    assert_eq!(storage.get().as_deref(), Some("token"));
    storage.delete();
    assert_eq!(storage.get(), None);
}

#[test]
fn fake_connectivity_probe_can_fail() {
    let probe = FakeConnectivityProbe { next_ok: false };
    assert!(probe.probe().is_err());
}

#[test]
fn fake_notification_backend_success_and_failure() {
    let mut backend = FakeNotificationBackend::default();
    backend.send("hello").expect("send");
    assert_eq!(backend.sent.len(), 1);

    backend.fail_send = true;
    assert!(backend.send("world").is_err());
    assert_eq!(backend.sent.len(), 1);
}
