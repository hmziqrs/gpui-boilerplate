use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, Disableable as _, Selectable as _, Theme,
    button::{Button, ButtonVariants as _},
    label::Label,
    switch::Switch,
    v_flex,
};

use crate::app::{self, LOCALE_EN, LOCALE_ZH_CN, LocaleState};
use crate::notifications::{
    self, NativeNotificationState, NotificationPermissionState, NotificationRequest,
    NotificationRuntimeSnapshot,
};

pub struct SettingsPage {
    dark_mode: bool,
    locale: SharedString,
    notifications: NotificationRuntimeSnapshot,
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
            cx.observe_global_in::<NativeNotificationState>(window, |this, _, cx| {
                this.notifications = notifications::snapshot(cx);
                cx.notify();
            }),
        ];

        Self {
            dark_mode: cx.theme().mode.is_dark(),
            locale: app::current_locale(cx),
            notifications: notifications::snapshot(cx),
            _subscriptions,
        }
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let locale = self.locale.clone();
        let is_dark = self.dark_mode;
        let notifications_snapshot = self.notifications.clone();
        let can_request_permission = notifications_snapshot.capabilities.can_request_permission
            && matches!(
                notifications_snapshot.permission,
                NotificationPermissionState::NotDetermined
                    | NotificationPermissionState::Unknown
                    | NotificationPermissionState::Unavailable(_)
            );
        let can_open_settings = cfg!(target_os = "macos")
            && matches!(
                notifications_snapshot.permission,
                NotificationPermissionState::Denied | NotificationPermissionState::Unavailable(_)
            );

        v_flex()
            .size_full()
            .p_6()
            .gap_6()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child(crate::i18n::localize("settings_title", None)),
            )
            // Dark mode toggle
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new(crate::i18n::localize(
                        "settings_dark_mode",
                        None,
                    )))
                    .child(Switch::new("dark-mode").checked(is_dark).on_click(
                        move |checked, _, cx| {
                            let mode = if *checked {
                                gpui_component::ThemeMode::Dark
                            } else {
                                gpui_component::ThemeMode::Light
                            };
                            app::set_theme_mode(mode, cx);
                        },
                    )),
            )
            // Language selection
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new(crate::i18n::localize("settings_language", None)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("settings-language-en")
                                    .outline()
                                    .selected(locale.as_ref() == LOCALE_EN)
                                    .label(crate::i18n::localize("settings_language_english", None))
                                    .on_click(|_, _, cx| {
                                        app::set_locale(LOCALE_EN, cx);
                                    }),
                            )
                            .child(
                                Button::new("settings-language-zh-cn")
                                    .outline()
                                    .selected(locale.as_ref() == LOCALE_ZH_CN)
                                    .label(crate::i18n::localize(
                                        "settings_language_simplified_chinese",
                                        None,
                                    ))
                                    .on_click(|_, _, cx| {
                                        app::set_locale(LOCALE_ZH_CN, cx);
                                    }),
                            ),
                    ),
            )
            // Native local notifications
            .child(
                v_flex()
                    .gap_3()
                    .p_4()
                    .rounded(cx.theme().radius)
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(Label::new(crate::i18n::localize(
                                "settings_native_notifications",
                                None,
                            )))
                            .child(
                                Switch::new("native-notifications-enabled")
                                    .checked(notifications_snapshot.enabled_by_user)
                                    .on_click(|checked, _, cx| {
                                        notifications::set_native_notifications_enabled(
                                            *checked, cx,
                                        );
                                    }),
                            ),
                    )
                    .child(status_row(
                        crate::i18n::localize("settings_native_backend", None),
                        notifications_snapshot.active_backend.to_string(),
                    ))
                    .child(status_row(
                        crate::i18n::localize("settings_permission", None),
                        notifications_snapshot.permission.label(),
                    ))
                    .when_some(
                        notifications_snapshot.degraded_reason.clone(),
                        |this, reason| {
                            this.child(status_row(
                                crate::i18n::localize("settings_degraded", None),
                                reason,
                            ))
                        },
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("test-native-notification")
                                    .primary()
                                    .label(crate::i18n::localize(
                                        "settings_test_native_notification",
                                        None,
                                    ))
                                    .on_click(|_, window, cx| {
                                        notifications::send_from_window(
                                            NotificationRequest::foreground(
                                                crate::i18n::localize(
                                                    "settings_test_native_notification",
                                                    None,
                                                ),
                                                crate::i18n::localize(
                                                    "settings_hello_notification",
                                                    None,
                                                ),
                                            ),
                                            window,
                                            cx,
                                        );
                                    }),
                            )
                            .child(
                                Button::new("request-notification-permission")
                                    .outline()
                                    .disabled(!can_request_permission)
                                    .label(crate::i18n::localize(
                                        "settings_request_permission",
                                        None,
                                    ))
                                    .on_click(|_, window, cx| {
                                        notifications::request_permission_from_window(window, cx);
                                    }),
                            )
                            .child(
                                Button::new("open-notification-settings")
                                    .outline()
                                    .disabled(!can_open_settings)
                                    .label(crate::i18n::localize(
                                        "settings_open_notification_settings",
                                        None,
                                    ))
                                    .on_click(|_, _, cx| {
                                        notifications::open_system_settings(cx);
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("test-action-notification")
                                    .outline()
                                    .label(crate::i18n::localize(
                                        "settings_test_action_notification",
                                        None,
                                    ))
                                    .on_click(|_, window, cx| {
                                        notifications::send_from_window(
                                            NotificationRequest::action_buttons(
                                                crate::i18n::localize(
                                                    "settings_test_action_notification",
                                                    None,
                                                ),
                                                crate::i18n::localize(
                                                    "settings_action_notification_body",
                                                    None,
                                                ),
                                            ),
                                            window,
                                            cx,
                                        );
                                    }),
                            )
                            .child(
                                Button::new("test-reply-notification")
                                    .outline()
                                    .label(crate::i18n::localize(
                                        "settings_test_reply_notification",
                                        None,
                                    ))
                                    .on_click(|_, window, cx| {
                                        notifications::send_from_window(
                                            NotificationRequest::reply(
                                                crate::i18n::localize(
                                                    "settings_test_reply_notification",
                                                    None,
                                                ),
                                                crate::i18n::localize(
                                                    "settings_reply_notification_body",
                                                    None,
                                                ),
                                            ),
                                            window,
                                            cx,
                                        );
                                    }),
                            )
                            .child(
                                Button::new("test-background-worthy-notification")
                                    .outline()
                                    .label(crate::i18n::localize(
                                        "settings_test_background_notification",
                                        None,
                                    ))
                                    .on_click(|_, window, cx| {
                                        notifications::send_from_window(
                                            NotificationRequest::background_worthy(
                                                crate::i18n::localize(
                                                    "settings_test_background_notification",
                                                    None,
                                                ),
                                                crate::i18n::localize(
                                                    "settings_background_notification_body",
                                                    None,
                                                ),
                                            ),
                                            window,
                                            cx,
                                        );
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(crate::i18n::localize(
                                "settings_in_app_notifications_note",
                                None,
                            )),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(crate::i18n::localize(
                                "settings_push_notifications_note",
                                None,
                            )),
                    ),
            )
    }
}

fn status_row(label: impl Into<SharedString>, value: impl Into<SharedString>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(Label::new(label.into()))
        .child(div().text_sm().child(value.into()))
}
