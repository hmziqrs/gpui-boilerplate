use crate::app_state::{APP_STATE_VERSION, AppConfig};

pub fn migrate(mut config: AppConfig) -> AppConfig {
    // v0 -> v1 migration placeholders. Keep explicit so later versions can
    // append deterministic transforms.
    if config.version < 1 {
        config.global_shortcut_enabled = true;
        config.version = 1;
    }

    if config.version != APP_STATE_VERSION {
        config.version = APP_STATE_VERSION;
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_config_to_current_version() {
        let legacy = AppConfig {
            version: 0,
            global_shortcut_enabled: false,
            ..AppConfig::default()
        };
        let migrated = migrate(legacy);
        assert_eq!(migrated.version, APP_STATE_VERSION);
        assert!(migrated.global_shortcut_enabled);
    }
}
