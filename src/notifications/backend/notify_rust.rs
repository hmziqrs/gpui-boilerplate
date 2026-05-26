use async_trait::async_trait;

use super::NotificationBackend;
use crate::notifications::{
    NotificationBackendKind, NotificationCapabilities, NotificationPermissionState,
    NotificationRequest,
};

pub struct NotifyRustBackend;

impl NotifyRustBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NotificationBackend for NotifyRustBackend {
    fn kind(&self) -> NotificationBackendKind {
        NotificationBackendKind::NotifyRust
    }

    fn capabilities(&self) -> NotificationCapabilities {
        NotificationCapabilities {
            can_request_permission: false,
            can_read_permission_state: false,
            can_send_immediate_native: true,
            can_send_interactive: false,
            requires_packaged_runtime: cfg!(target_os = "windows"),
        }
    }

    async fn refresh_permission_state(&self) -> NotificationPermissionState {
        NotificationPermissionState::Unsupported
    }

    async fn request_permission(&self) -> NotificationPermissionState {
        NotificationPermissionState::Unsupported
    }

    async fn send(&self, request: &NotificationRequest) -> anyhow::Result<()> {
        let mut notification = notify_rust::Notification::new();
        notification
            .appname("GPUI Starter")
            .summary(&request.title)
            .body(&request.body);

        if request.play_sound {
            notification.sound_name("default");
        }

        notification.show().map(|_| ())?;
        Ok(())
    }
}
