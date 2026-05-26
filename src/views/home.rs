use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, v_flex,
    button::{Button, ButtonVariants as _},
};

pub struct HomePage;

impl HomePage {
    pub fn new() -> Self {
        Self
    }
}

impl Render for HomePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_6()
            .child(
                div()
                    .text_3xl()
                    .font_weight(FontWeight::BOLD)
                    .child(crate::i18n::localize("home_title", None)),
            )
            .child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(crate::i18n::localize("home_subtitle", None)),
            )
            .child(
                Button::new("get-started")
                    .primary()
                    .label(crate::i18n::localize("home_get_started", None))
                    .on_click(|_, _, _| {
                        tracing::info!("Get Started clicked");
                    }),
            )
    }
}
