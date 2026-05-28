use gpui::{App, Global, SharedString};

#[derive(Clone, Debug, Default)]
pub struct SecureStorageSnapshot {
    pub available: bool,
    pub last_error: Option<String>,
}

impl Global for SecureStorageSnapshot {}

pub fn initialize(cx: &mut App) {
    let available = keyring::Entry::new("gpui-starter", "availability-check").is_ok();
    let snapshot = SecureStorageSnapshot {
        available,
        last_error: if available {
            None
        } else {
            Some("keyring entry initialization unavailable".to_string())
        },
    };
    cx.set_global(snapshot.clone());
    crate::capabilities::set(
        "secure_storage",
        crate::capabilities::CapabilityStatus {
            supported: true,
            enabled: snapshot.available,
            degraded: !snapshot.available,
            reason: snapshot.last_error.clone().map(Into::into),
            last_error: snapshot.last_error.clone().map(Into::into),
        },
        cx,
    );
}

pub fn snapshot(cx: &App) -> SecureStorageSnapshot {
    cx.try_global::<SecureStorageSnapshot>()
        .cloned()
        .unwrap_or_default()
}

pub fn set_secret(service: &str, key: &str, value: &str, cx: &mut App) -> Result<(), String> {
    let entry = keyring::Entry::new(service, key).map_err(|err| {
        tracing::error!(target: "gpui_starter::secure_storage", "entry creation failed: {err}");
        err.to_string()
    })?;
    entry.set_password(value).map_err(|err| {
        tracing::error!(target: "gpui_starter::secure_storage", service, key, "set_password failed: {err}");
        update_last_error(Some(err.to_string()), cx);
        err.to_string()
    })?;
    tracing::info!(target: "gpui_starter::secure_storage", service, key, "secret written");
    update_last_error(None, cx);
    Ok(())
}

pub fn get_secret(service: &str, key: &str, cx: &mut App) -> Result<Option<SharedString>, String> {
    let entry = keyring::Entry::new(service, key).map_err(|err| {
        tracing::error!(target: "gpui_starter::secure_storage", "entry creation failed: {err}");
        err.to_string()
    })?;
    match entry.get_password() {
        Ok(value) => {
            tracing::info!(target: "gpui_starter::secure_storage", service, key, "secret read");
            update_last_error(None, cx);
            Ok(Some(value.into()))
        }
        Err(keyring::Error::NoEntry) => {
            tracing::warn!(target: "gpui_starter::secure_storage", service, key, "no entry found");
            Ok(None)
        }
        Err(err) => {
            tracing::error!(target: "gpui_starter::secure_storage", service, key, "get_password failed: {err}");
            Err(err.to_string())
        }
    }
}

pub fn delete_secret(service: &str, key: &str, cx: &mut App) -> Result<(), String> {
    let entry = keyring::Entry::new(service, key).map_err(|err| {
        tracing::error!(target: "gpui_starter::secure_storage", "entry creation failed: {err}");
        err.to_string()
    })?;
    entry.delete_credential().map_err(|err| {
        tracing::error!(target: "gpui_starter::secure_storage", service, key, "delete failed: {err}");
        update_last_error(Some(err.to_string()), cx);
        err.to_string()
    })?;
    tracing::info!(target: "gpui_starter::secure_storage", service, key, "secret deleted");
    update_last_error(None, cx);
    Ok(())
}

fn update_last_error(last_error: Option<String>, cx: &mut App) {
    let mut current = snapshot(cx);
    current.last_error = last_error;
    cx.set_global(current);
}
