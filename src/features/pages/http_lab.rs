use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, button::Button, v_flex};

use crate::http_lab::{self, HttpExchange, HttpLabAction, HttpLabState};

pub struct HttpLabPage {
    _subscriptions: Vec<Subscription>,
}

impl HttpLabPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.observe_global_in::<HttpLabState>(window, |_, _, cx| {
            cx.notify();
        })];

        Self {
            _subscriptions: subscriptions,
        }
    }
}

impl Render for HttpLabPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = http_lab::snapshot(cx);
        let current_exchange = state.last_error.as_ref().or(state.last_success.as_ref());

        v_flex()
            .min_h_full()
            .p_6()
            .gap_5()
            .child(header(&state, cx))
            .child(action_grid())
            .child(flow_panel(&state, cx))
            .when_some(current_exchange, |this, exchange| {
                this.child(exchange_panel(exchange, cx))
            })
            .when_some(state.cookies.as_ref(), |this, cookies| {
                this.child(
                    panel("Cookie jar", cx)
                        .child(kv(
                            "Set-Cookie",
                            cookies.set_cookie_header.as_deref().unwrap_or("None"),
                            cx,
                        ))
                        .child(kv(
                            "Echoed cookies",
                            cookies.echoed_cookies_json.as_deref().unwrap_or("None"),
                            cx,
                        )),
                )
            })
            .child(history_panel(&state, cx))
    }
}

fn header(state: &HttpLabState, cx: &App) -> Div {
    panel("HTTP Lab", cx)
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("Uses https://httpbin.org with reqwest blocking calls on GPUI background executor threads."),
        )
        .child(kv("State", state.status.label(), cx))
        .child(kv(
            "Active request",
            state.active_label.as_deref().unwrap_or("None"),
            cx,
        ))
}

fn action_grid() -> Div {
    let actions = [
        HttpLabAction::FullFlow,
        HttpLabAction::GetText,
        HttpLabAction::GetJson,
        HttpLabAction::GetXml,
        HttpLabAction::PostJson,
        HttpLabAction::PostForm,
        HttpLabAction::PostMultipart,
        HttpLabAction::Cookies,
        HttpLabAction::Failure,
    ];

    div()
        .flex()
        .flex_wrap()
        .gap_2()
        .children(actions.into_iter().map(action_button))
        .child(
            Button::new("http-lab-reset")
                .outline()
                .label("Reset")
                .on_click(|_, _, cx| {
                    http_lab::reset(cx);
                }),
        )
}

fn action_button(action: HttpLabAction) -> Button {
    Button::new(format!("http-lab-{:?}", action))
        .outline()
        .label(action.label())
        .on_click(move |_, _, cx| {
            http_lab::run_action(action, cx);
        })
}

fn flow_panel(state: &HttpLabState, cx: &App) -> Div {
    panel("Lifecycle trace", cx)
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("Target sequence: IDLE -> Loading EMPTY State -> Success/Failure -> Loading with state -> Success/Failure."),
        )
        .children(
            state
                .transition_log
                .iter()
                .map(|entry| div().text_sm().child(entry.clone())),
        )
}

fn exchange_panel(exchange: &HttpExchange, cx: &App) -> Div {
    let mut panel = panel("Latest exchange", cx)
        .child(kv("Label", &exchange.label, cx))
        .child(kv("Method", &exchange.request.method, cx))
        .child(kv("URL", &exchange.request.url, cx))
        .child(kv(
            "Request body kind",
            exchange.request.request_body_kind.label(),
            cx,
        ))
        .child(preview_block(
            "Request body preview",
            &exchange.request.request_body_preview,
            cx,
        ));

    if let Some(response) = &exchange.response {
        panel = panel
            .child(kv(
                "Status",
                &format!("{} {}", response.status, response.status_text),
                cx,
            ))
            .child(kv("Final URL", &response.final_url, cx))
            .child(kv("Elapsed", &format!("{}ms", response.elapsed_ms), cx))
            .child(kv("Body kind", response.body_kind.label(), cx))
            .child(headers_block(&response.headers, cx))
            .child(preview_block("Body text", &response.body_preview, cx))
            .when_some(response.parsed_json.as_ref(), |this, json| {
                this.child(preview_block("Parsed JSON", json, cx))
            })
            .when_some(response.parsed_xml_preview.as_ref(), |this, xml| {
                this.child(preview_block("Parsed XML", xml, cx))
            });
    }

    panel.when_some(exchange.error.as_ref(), |this, error| {
        this.child(kv("Error", error, cx))
    })
}

fn headers_block(headers: &[(String, String)], cx: &App) -> Div {
    div().mt_2().child(section_title("Headers", cx)).children(
        headers
            .iter()
            .take(16)
            .map(|(name, value)| kv(name, value, cx)),
    )
}

fn history_panel(state: &HttpLabState, cx: &App) -> Div {
    panel("History", cx).children(state.history.iter().map(|exchange| {
        let fields = http_lab::response_fields(exchange);
        let summary = fields
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(" | ");
        div().text_sm().child(summary)
    }))
}

fn panel(title: &str, cx: &App) -> Div {
    v_flex()
        .gap_3()
        .p_4()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .child(section_title(title, cx))
}

fn section_title(title: &str, _cx: &App) -> Div {
    div()
        .text_lg()
        .font_weight(FontWeight::BOLD)
        .child(title.to_string())
}

fn kv(label: &str, value: &str, cx: &App) -> Div {
    div()
        .flex()
        .gap_2()
        .text_sm()
        .child(
            div()
                .min_w(px(140.))
                .font_weight(FontWeight::BOLD)
                .child(format!("{label}:")),
        )
        .child(
            div()
                .flex_1()
                .text_color(cx.theme().muted_foreground)
                .child(value.to_string()),
        )
}

fn preview_block(label: &str, value: &str, cx: &App) -> Div {
    v_flex()
        .gap_1()
        .child(section_title(label, cx).text_sm())
        .child(
            div()
                .p_3()
                .rounded(cx.theme().radius_lg)
                .bg(cx.theme().muted)
                .text_sm()
                .child(if value.is_empty() {
                    "None".to_string()
                } else {
                    value.to_string()
                }),
        )
}
