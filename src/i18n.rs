use std::{collections::HashMap, sync::OnceLock};

use es_fluent::{FluentLocalizer as _, FluentMessage, FluentValue};
use es_fluent_manager_embedded::EmbeddedI18n;

es_fluent_manager_embedded::define_i18n_module!();

static I18N: OnceLock<EmbeddedI18n> = OnceLock::new();

pub fn init_i18n(lang: es_fluent::unic_langid::LanguageIdentifier) -> Result<(), String> {
    let i18n = EmbeddedI18n::try_new_with_language(lang).map_err(|e| e.to_string())?;
    let _ = I18N.set(i18n);
    Ok(())
}

pub fn i18n() -> &'static EmbeddedI18n {
    I18N.get().expect("i18n must be initialized before use")
}

pub fn localize(id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>) -> String {
    i18n().localize(id, args).unwrap_or_else(|| id.to_string())
}

pub fn localize_message<T: FluentMessage + ?Sized>(message: &T) -> String {
    i18n().localize_message(message)
}
