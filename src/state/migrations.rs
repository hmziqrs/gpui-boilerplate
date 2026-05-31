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
#[path = "migrations.test.rs"]
mod migrations_test;
