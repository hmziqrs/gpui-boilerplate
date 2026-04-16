use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, Selectable as _, Theme, WindowExt as _, button::Button, label::Label,
    switch::Switch, v_flex,
};

use crate::app::{self, LocaleState, LOCALE_EN, LOCALE_ZH_CN};

pub struct SettingsPage {
    dark_mode: bool,
    locale: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl SettingsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _subscriptions = vec![
            cx.observe_global_in::<Theme>(window, |this, _, cx| {
                let dark_mode = cx.theme().mode.is_dark();
                if this.dark_mode != dark_mode {
                    this.dark_mode = dark_mode;
                    cx.notify();
                }
            }),
            cx.observe_global_in::<LocaleState>(window, |this, _, cx| {
                let locale = app::current_locale(cx);
                if this.locale != locale {
                    this.locale = locale;
                    cx.notify();
                }
            }),
        ];

        Self {
            dark_mode: cx.theme().mode.is_dark(),
            locale: app::current_locale(cx),
            _subscriptions,
        }
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let locale = self.locale.clone();
        let is_dark = self.dark_mode;

        v_flex()
            .size_full()
            .p_6()
            .gap_6()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child(es_fluent::localize("settings_title", None)),
            )
            // Dark mode toggle
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new(es_fluent::localize("settings_dark_mode", None)))
                    .child(
                        Switch::new("dark-mode")
                            .checked(is_dark)
                            .on_click(move |checked, _, cx| {
                                let mode = if *checked {
                                    gpui_component::ThemeMode::Dark
                                } else {
                                    gpui_component::ThemeMode::Light
                                };
                                app::set_theme_mode(mode, cx);
                            }),
                    ),
            )
            // Language selection
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new(es_fluent::localize("settings_language", None)))
                    .child(
                        div().flex().items_center().gap_2().child(
                            Button::new("settings-language-en")
                                .outline()
                                .selected(locale.as_ref() == LOCALE_EN)
                                .label(es_fluent::localize("settings_language_english", None))
                                .on_click(|_, _, cx| {
                                    app::set_locale(LOCALE_EN, cx);
                                }),
                        )
                        .child(
                            Button::new("settings-language-zh-cn")
                                .outline()
                                .selected(locale.as_ref() == LOCALE_ZH_CN)
                                .label(es_fluent::localize(
                                    "settings_language_simplified_chinese",
                                    None,
                                ))
                                .on_click(|_, _, cx| {
                                    app::set_locale(LOCALE_ZH_CN, cx);
                                }),
                        ),
                    ),
            )
            // Push notification
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new(es_fluent::localize(
                        "settings_push_notification",
                        None,
                    )))
                    .child(
                        Button::new("notify")
                            .label(es_fluent::localize("settings_notify", None))
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    es_fluent::localize("settings_hello_notification", None),
                                    cx,
                                );
                            }),
                    ),
            )
    }
}
