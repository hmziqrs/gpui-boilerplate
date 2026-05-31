use std::{collections::HashMap, sync::OnceLock};

use es_fluent::{FluentLocalizer as _, FluentMessage, FluentValue};
use es_fluent_manager_embedded::EmbeddedI18n;

es_fluent_manager_embedded::define_i18n_module!();

static I18N: OnceLock<EmbeddedI18n> = OnceLock::new();

pub fn init_i18n(lang: es_fluent::unic_langid::LanguageIdentifier) -> Result<(), String> {
    EmbeddedI18n::try_new_with_language(lang).map_err(|e| e.to_string()).map(|i18n| {
        let _ = I18N.set(i18n);
    })
}

pub fn i18n() -> &'static EmbeddedI18n {
    I18N.get_or_init(|| {
        tracing::warn!("i18n not initialized, using fallback");
        EmbeddedI18n::try_new().expect("embedded i18n fallback must succeed")
    })
}

pub fn localize(id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>) -> String {
    i18n().localize(id, args).unwrap_or_else(|| id.to_string())
}

pub fn localize_message<T: FluentMessage + ?Sized>(message: &T) -> String {
    i18n().localize_message(message)
}

/// Detect the system locale using the `sys-locale` crate. Returns a
/// normalised language identifier string (e.g. `"en-US"`, `"zh-CN"`).
/// Falls back to `"en"` when detection fails.
pub fn detect_system_locale() -> String {
    sys_locale::get_locale().unwrap_or_else(|| "en".to_string())
}
