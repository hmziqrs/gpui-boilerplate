mod backend;
mod service;

pub use service::{
    NativeNotificationState, NotificationPermissionState, NotificationRequest,
    NotificationRuntimeSnapshot, initialize, open_system_settings, request_permission_from_window,
    send_from_window, set_native_notifications_enabled, snapshot,
};

pub(crate) use service::{NotificationBackendKind, NotificationCapabilities};
