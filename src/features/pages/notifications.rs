use gpui::{prelude::*, *};
use gpui_component::button::Button;
use gpui_component::{h_flex, v_flex};

use crate::notifications::inbox::{self, NotificationInboxItem};

pub struct NotificationsPage {
    _subscriptions: Vec<Subscription>,
}

impl NotificationsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscriptions =
            vec![
                cx.observe_global_in::<inbox::NotificationInboxState>(window, |_, _, cx| {
                    cx.notify();
                }),
            ];
        Self {
            _subscriptions: subscriptions,
        }
    }
}

impl Render for NotificationsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = inbox::snapshot(cx);
        let unread = items.iter().filter(|item| !item.read).count();

        v_flex()
            .min_h_full()
            .p_6()
            .gap_4()
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .child(format!("Notifications ({unread} unread)")),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("notifications-mark-read")
                                    .outline()
                                    .label("Mark all read")
                                    .on_click(|_, _, cx| {
                                        inbox::mark_all_read(cx);
                                    }),
                            )
                            .child(
                                Button::new("notifications-clear-all")
                                    .outline()
                                    .label("Clear all")
                                    .on_click(|_, _, cx| {
                                        inbox::clear_all(cx);
                                    }),
                            ),
                    ),
            )
            .children(items.into_iter().map(render_item))
    }
}

fn render_item(item: NotificationInboxItem) -> Div {
    let timestamp = item.created_at.to_rfc3339();
    let summary = item.summary_line();
    let title = item.title;
    let body = item.body;
    let error_summary = item.error_summary;

    v_flex()
        .gap_1()
        .p_3()
        .border_1()
        .rounded_lg()
        .child(
            h_flex()
                .justify_between()
                .items_center()
                .child(div().font_weight(FontWeight::BOLD).child(title))
                .child(div().text_xs().child(timestamp)),
        )
        .child(div().text_sm().child(body))
        .child(div().text_xs().child(summary))
        .when_some(error_summary, |this, error| {
            this.child(div().text_xs().child(format!("error: {error}")))
        })
}
