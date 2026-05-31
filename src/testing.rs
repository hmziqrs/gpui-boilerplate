#![allow(dead_code)]

use std::collections::VecDeque;

#[derive(Debug, thiserror::Error)]
pub enum FakeNotificationError {
    #[error("send failed")]
    SendFailed,
}

#[derive(Debug, thiserror::Error)]
pub enum FakeConnectivityError {
    #[error("offline")]
    Offline,
}

#[derive(Default)]
pub struct FakeTelemetrySink {
    pub events: VecDeque<String>,
    pub flushed: bool,
}

impl FakeTelemetrySink {
    pub fn record_event(&mut self, name: &str) {
        self.events.push_back(format!("event:{name}"));
    }

    pub fn record_error(&mut self, error: &str) {
        self.events.push_back(format!("error:{error}"));
    }

    pub fn flush(&mut self) {
        self.flushed = true;
    }
}

#[derive(Default)]
pub struct FakeConnectivityProbe {
    pub next_ok: bool,
}

#[derive(Default)]
pub struct FakeNotificationBackend {
    pub sent: VecDeque<String>,
    pub fail_send: bool,
}

impl FakeNotificationBackend {
    pub fn send(&mut self, title: &str) -> Result<(), FakeNotificationError> {
        if self.fail_send {
            return Err(FakeNotificationError::SendFailed);
        }
        self.sent.push_back(title.to_string());
        Ok(())
    }
}

impl FakeConnectivityProbe {
    pub fn probe(&self) -> Result<(), FakeConnectivityError> {
        if self.next_ok { Ok(()) } else { Err(FakeConnectivityError::Offline) }
    }
}

#[derive(Default)]
pub struct FakeSecureStorage {
    value: Option<String>,
}

impl FakeSecureStorage {
    pub fn set(&mut self, value: &str) {
        self.value = Some(value.to_string());
    }

    pub fn get(&self) -> Option<String> {
        self.value.clone()
    }

    pub fn delete(&mut self) {
        self.value = None;
    }
}

#[cfg(test)]
mod tests {
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
}
