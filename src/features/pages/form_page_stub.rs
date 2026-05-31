use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, v_flex};

pub struct FormPage;

impl FormPage {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for FormPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_6()
            .gap_3()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child("Form Demo"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Enable the `form-demo` feature to build the gpui-form page."),
            )
    }
}
