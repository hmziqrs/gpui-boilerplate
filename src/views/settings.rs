use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, Selectable as _, WindowExt as _, v_flex,
    button::Button,
    label::Label,
    switch::Switch,
};

use crate::app::SwitchThemeMode;

pub struct SettingsPage;

impl SettingsPage {
    pub fn new(_: &mut Window, _: &mut App) -> Self {
        Self
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = cx.theme().mode.is_dark();

        v_flex()
            .size_full()
            .p_6()
            .gap_6()
            .child(
                div().text_xl().font_weight(FontWeight::BOLD).child("Settings"),
            )
            // Dark mode toggle
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new("Dark Mode"))
                    .child(
                        Switch::new("dark-mode")
                            .checked(is_dark)
                            .on_click(cx.listener(|_, _: &bool, _, cx| {
                                let next = if cx.theme().mode.is_dark() {
                                    gpui_component::ThemeMode::Light
                                } else {
                                    gpui_component::ThemeMode::Dark
                                };
                                cx.dispatch_action(&SwitchThemeMode(next));
                            })),
                    ),
            )
            // Light / Dark buttons (mirrors the Appearance menu)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(Label::new("Appearance"))
                    .child(
                        div().flex().gap_2().children(vec![
                            Button::new("light-mode")
                                .label("Light")
                                .selected(!is_dark)
                                .on_click(cx.listener(|_, _, _, cx| {
                                    cx.dispatch_action(&SwitchThemeMode(gpui_component::ThemeMode::Light));
                                })),
                            Button::new("dark-mode-btn")
                                .label("Dark")
                                .selected(is_dark)
                                .on_click(cx.listener(|_, _, _, cx| {
                                    cx.dispatch_action(&SwitchThemeMode(gpui_component::ThemeMode::Dark));
                                })),
                        ]),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Label::new("Push a Notification"))
                    .child(
                        Button::new("notify")
                            .label("Notify")
                            .on_click(|_, window, cx| {
                                window.push_notification("Hello from Settings!", cx);
                            }),
                    ),
            )
    }
}
