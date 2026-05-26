use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use super::NotificationBackend;
use crate::notifications::{
    CATEGORY_ACTIONS, CATEGORY_REPLY, NotificationBackendKind, NotificationCapabilities,
    NotificationPermissionState, NotificationRequest,
};

const APP_ID: &str = "com.gpui-starter.app";
const LOG: &str = "gpui_starter::notifications::user_notify";

pub struct UserNotifyBackend {
    manager: Arc<dyn user_notify::NotificationManager>,
}

impl UserNotifyBackend {
    pub fn new() -> anyhow::Result<Self> {
        tracing::info!(target: LOG, app_id = APP_ID, "initializing user-notify backend");

        if let Err(reason) = platform_primary_runtime_status() {
            tracing::warn!(target: LOG, reason, "user-notify backend unavailable");
            anyhow::bail!("{reason}");
        }

        let manager = user_notify::get_notification_manager(APP_ID.to_string(), None);
        if let Err(err) = manager.register(
            Box::new(|response| {
                tracing::info!(
                    target: LOG,
                    action = ?response.action,
                    user_text = ?response.user_text,
                    user_info = ?response.user_info,
                    "native notification response"
                );
            }),
            categories(),
        ) {
            tracing::warn!(target: LOG, error = %err, "failed to register user-notify manager");
            return Err(err.into());
        }

        tracing::info!(target: LOG, "user-notify backend initialized");
        Ok(Self { manager })
    }
}

fn categories() -> Vec<user_notify::NotificationCategory> {
    vec![
        user_notify::NotificationCategory {
            identifier: CATEGORY_ACTIONS.to_string(),
            actions: vec![
                user_notify::NotificationCategoryAction::Action {
                    identifier: "settings.open".to_string(),
                    title: "Open".to_string(),
                },
                user_notify::NotificationCategoryAction::Action {
                    identifier: "settings.snooze".to_string(),
                    title: "Snooze".to_string(),
                },
            ],
        },
        user_notify::NotificationCategory {
            identifier: CATEGORY_REPLY.to_string(),
            actions: vec![user_notify::NotificationCategoryAction::TextInputAction {
                identifier: "settings.reply".to_string(),
                title: "Reply".to_string(),
                input_button_title: "Send".to_string(),
                input_placeholder: "Type a reply".to_string(),
            }],
        },
    ]
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
        tracing::debug!(target: LOG, "refreshing user-notify permission state");
        let state = platform_permission_state().await;
        tracing::info!(target: LOG, ?state, "refreshed user-notify permission state");
        state
    }

    async fn request_permission(&self) -> NotificationPermissionState {
        if !cfg!(target_os = "macos") {
            tracing::info!(target: LOG, "permission request unsupported on this platform");
            return NotificationPermissionState::Unsupported;
        }

        tracing::info!(target: LOG, "requesting notification permission");
        match self
            .manager
            .first_time_ask_for_notification_permission()
            .await
        {
            Ok(accepted) => {
                tracing::info!(target: LOG, accepted, "permission request completed");
                platform_permission_state().await
            }
            Err(err) => {
                tracing::warn!(target: LOG, error = %err, "permission request failed");
                NotificationPermissionState::Unavailable(format!("{err:#}"))
            }
        }
    }

    async fn send(&self, request: &NotificationRequest) -> anyhow::Result<()> {
        tracing::info!(
            target: LOG,
            title = %request.title,
            importance = %request.importance,
            has_thread_id = request.thread_id.is_some(),
            has_category = request.category.is_some(),
            "sending notification through user-notify"
        );

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

        match self.manager.send_notification(builder).await {
            Ok(_) => {
                tracing::info!(target: LOG, "user-notify send succeeded");
                Ok(())
            }
            Err(err) => {
                tracing::warn!(target: LOG, error = %err, "user-notify send failed");
                Err(err.into())
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn platform_primary_runtime_status() -> Result<(), String> {
    use objc2_foundation::NSBundle;

    match NSBundle::mainBundle().bundleIdentifier() {
        Some(bundle_id) => {
            tracing::info!(
                target: LOG,
                bundle_id = %bundle_id,
                "macOS bundle identifier detected"
            );
            Ok(())
        }
        None => Err(
            "macOS bundle identifier missing; launch the bundled app from scripts/macos-dev-app.sh instead of raw cargo run"
                .to_string(),
        ),
    }
}

#[cfg(not(target_os = "macos"))]
fn platform_primary_runtime_status() -> Result<(), String> {
    tracing::debug!(target: LOG, "primary runtime accepted on this platform");
    Ok(())
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
            let status = settings.as_ref().authorizationStatus();
            tracing::debug!(target: LOG, ?status, "received macOS notification authorization status");
            let state = match status {
                UNAuthorizationStatus::Authorized
                | UNAuthorizationStatus::Provisional
                | UNAuthorizationStatus::Ephemeral => NotificationPermissionState::Authorized,
                UNAuthorizationStatus::Denied => NotificationPermissionState::Denied,
                UNAuthorizationStatus::NotDetermined => NotificationPermissionState::NotDetermined,
                status => NotificationPermissionState::Unavailable(format!(
                    "unknown macOS authorization status: {status:?}"
                )),
            };

            if let Some(tx) = tx.take()
                && tx.send(state).is_err()
            {
                tracing::warn!(target: LOG, "permission state receiver dropped");
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
