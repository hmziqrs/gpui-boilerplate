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
        if self.next_ok {
            Ok(())
        } else {
            Err(FakeConnectivityError::Offline)
        }
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
#[path = "testing.test.rs"]
mod testing_test;
