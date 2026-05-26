use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use super::NotificationBackend;
use crate::notifications::{
    NotificationBackendKind, NotificationCapabilities, NotificationPermissionState,
    NotificationRequest,
};

const APP_ID: &str = "com.gpui-starter.app";

pub struct UserNotifyBackend {
    manager: Arc<dyn user_notify::NotificationManager>,
}

impl UserNotifyBackend {
    pub fn new() -> anyhow::Result<Self> {
        if !platform_can_use_primary_backend() {
            anyhow::bail!("user-notify primary backend unavailable in this runtime");
        }

        let manager = user_notify::get_notification_manager(APP_ID.to_string(), None);
        manager.register(
            Box::new(|response| {
                tracing::debug!(?response, "native notification response");
            }),
            Vec::new(),
        )?;

        Ok(Self { manager })
    }
}

#[async_trait]
impl NotificationBackend for UserNotifyBackend {
    fn kind(&self) -> NotificationBackendKind {
        NotificationBackendKind::UserNotify
    }

    fn capabilities(&self) -> NotificationCapabilities {
        NotificationCapabilities {
            can_request_permission: cfg!(target_os = "macos"),
            can_read_permission_state: cfg!(target_os = "macos"),
            can_send_immediate_native: true,
            can_send_interactive: cfg!(target_os = "macos"),
            requires_packaged_runtime: cfg!(target_os = "macos"),
        }
    }

    async fn refresh_permission_state(&self) -> NotificationPermissionState {
        platform_permission_state().await
    }

    async fn request_permission(&self) -> NotificationPermissionState {
        if !cfg!(target_os = "macos") {
            return NotificationPermissionState::Unsupported;
        }

        match self
            .manager
            .first_time_ask_for_notification_permission()
            .await
        {
            Ok(_) => platform_permission_state().await,
            Err(err) => NotificationPermissionState::Unavailable(format!("{err:#}")),
        }
    }

    async fn send(&self, request: &NotificationRequest) -> anyhow::Result<()> {
        let mut user_info = HashMap::new();
        user_info.insert("importance".to_string(), request.importance.to_string());
        user_info.insert("play_sound".to_string(), request.play_sound.to_string());

        let mut builder = user_notify::NotificationBuilder::new()
            .title(&request.title)
            .body(&request.body)
            .set_user_info(user_info);

        if let Some(thread_id) = &request.thread_id {
            builder = builder.set_thread_id(thread_id);
        }

        if let Some(category) = &request.category {
            builder = builder.set_category_id(category);
        }

        self.manager.send_notification(builder).await?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn platform_can_use_primary_backend() -> bool {
    use objc2_foundation::NSBundle;

    NSBundle::mainBundle().bundleIdentifier().is_some()
}

#[cfg(not(target_os = "macos"))]
fn platform_can_use_primary_backend() -> bool {
    true
}

#[cfg(target_os = "macos")]
async fn platform_permission_state() -> NotificationPermissionState {
    use std::{cell::RefCell, ptr::NonNull};

    use block2::RcBlock;
    use objc2_user_notifications::{
        UNAuthorizationStatus, UNNotificationSettings, UNUserNotificationCenter,
    };

    let (tx, rx) = tokio::sync::oneshot::channel::<NotificationPermissionState>();

    unsafe {
        let tx = RefCell::new(Some(tx));
        let block = RcBlock::new(move |settings: NonNull<UNNotificationSettings>| {
            let state = match settings.as_ref().authorizationStatus() {
                UNAuthorizationStatus::Authorized
                | UNAuthorizationStatus::Provisional
                | UNAuthorizationStatus::Ephemeral => NotificationPermissionState::Authorized,
                UNAuthorizationStatus::Denied => NotificationPermissionState::Denied,
                UNAuthorizationStatus::NotDetermined => NotificationPermissionState::NotDetermined,
                status => NotificationPermissionState::Unavailable(format!(
                    "unknown macOS authorization status: {status:?}"
                )),
            };

            if let Some(tx) = tx.take() {
                let _ = tx.send(state);
            }
        });

        UNUserNotificationCenter::currentNotificationCenter()
            .getNotificationSettingsWithCompletionHandler(&block);
    }

    rx.await
        .unwrap_or_else(|err| NotificationPermissionState::Unavailable(err.to_string()))
}

#[cfg(not(target_os = "macos"))]
async fn platform_permission_state() -> NotificationPermissionState {
    NotificationPermissionState::Unsupported
}
