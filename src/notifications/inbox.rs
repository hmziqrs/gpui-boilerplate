use gpui::{App, Global};
use serde::{Deserialize, Serialize};

use crate::{
    app_state, ids::NotificationId, notifications::NotificationBackendKind, time::AppTimestamp,
};

const MAX_INBOX_ITEMS: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationInboxKind {
    Attempt,
    PermissionUpdate,
    SettingsUpdate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationInboxItem {
    pub id: NotificationId,
    pub created_at: AppTimestamp,
    pub title: String,
    pub body: String,
    pub read: bool,
    pub backend: String,
    pub delivered_natively: bool,
    pub degraded: bool,
    pub error_summary: Option<String>,
    pub kind: NotificationInboxKind,
}

impl NotificationInboxItem {
    pub fn summary_line(&self) -> String {
        format!(
            "{} • {}",
            self.backend,
            if self.read { "read" } else { "unread" }
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct NotificationInboxState {
    pub items: Vec<NotificationInboxItem>,
}

impl Global for NotificationInboxState {}

pub fn initialize(cx: &mut App) {
    let config = app_state::config(cx);
    cx.set_global(NotificationInboxState {
        items: config.notification_inbox,
    });
}

pub fn snapshot(cx: &App) -> Vec<NotificationInboxItem> {
    cx.try_global::<NotificationInboxState>()
        .map(|state| state.items.clone())
        .unwrap_or_default()
}

pub fn record(item: NotificationInboxItem, cx: &mut App) {
    {
        let state = cx.default_global::<NotificationInboxState>();
        state.items.insert(0, item);
        if state.items.len() > MAX_INBOX_ITEMS {
            state.items.truncate(MAX_INBOX_ITEMS);
        }
    }
    let items = cx.global::<NotificationInboxState>().items.clone();
    app_state::update_config(cx, |config| {
        config.notification_inbox = items;
    });
}

pub fn mark_all_read(cx: &mut App) {
    {
        let state = cx.default_global::<NotificationInboxState>();
        for item in &mut state.items {
            item.read = true;
        }
    }
    let items = cx.global::<NotificationInboxState>().items.clone();
    app_state::update_config(cx, |config| {
        config.notification_inbox = items;
    });
}

pub fn clear_all(cx: &mut App) {
    {
        let state = cx.default_global::<NotificationInboxState>();
        state.items.clear();
    }
    app_state::update_config(cx, |config| {
        config.notification_inbox = Vec::new();
    });
}

pub struct NotificationAttemptRecord {
    pub title: String,
    pub body: String,
    pub backend: NotificationBackendKind,
    pub delivered_natively: bool,
    pub degraded: bool,
    pub error_summary: Option<String>,
    pub kind: NotificationInboxKind,
}

pub fn record_attempt(record_input: NotificationAttemptRecord, cx: &mut App) {
    record(
        NotificationInboxItem {
            id: NotificationId::new(),
            created_at: AppTimestamp::now(),
            title: record_input.title,
            body: record_input.body,
            read: false,
            backend: record_input.backend.to_string(),
            delivered_natively: record_input.delivered_natively,
            degraded: record_input.degraded,
            error_summary: record_input.error_summary,
            kind: record_input.kind,
        },
        cx,
    );
}
