use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, v_flex};

pub struct AboutPage;

impl AboutPage {
    pub fn new() -> Self {
        Self
    }
}

impl Render for AboutPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_4()
            .child(
                div()
                    .text_2xl()
                    .child(es_fluent::localize("about_title", None)),
            )
            .child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(es_fluent::localize("about_version", None)),
            )
    }
}
