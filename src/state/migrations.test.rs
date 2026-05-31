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
