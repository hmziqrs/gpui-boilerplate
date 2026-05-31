use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, Disableable as _, Selectable as _,
    button::{Button, ButtonVariants as _},
    v_flex,
};

use crate::http_lab::{self, HttpExchange, HttpLabAction, HttpLabState};
use crate::query::{QueryResource, QueryStatus};

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
        let selected_resource = state.selected_resource();

        v_flex()
            .min_h_full()
            .p_6()
            .gap_5()
            .child(hero(&state, cx))
            .child(action_bar(&state))
            .child(tab_bar(&state))
            .child(resource_panel(
                &state,
                state.selected_action,
                selected_resource,
                cx,
            ))
            .child(activity_panel(&state, cx))
    }
}

fn hero(state: &HttpLabState, cx: &App) -> Div {
    div()
        .p_5()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().muted)
        .child(
            v_flex()
                .gap_3()
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::BOLD)
                        .child("HTTP Lab"),
                )
                .child(
                    div()
                        .max_w(px(760.))
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("A small React Query-style GPUI store: every request type has its own resource state, request policy, cache policy, request id, cancellation guard, and response cache."),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .child(chip(
                            &format!("Active requests: {}", state.active_count()),
                            cx.theme().background,
                            cx,
                        ))
                        .child(chip(
                            &format!("History: {}", state.history.len()),
                            cx.theme().background,
                            cx,
                        ))
                        .child(chip(
                            "Cancellation is logical: blocking reqwest may finish, but stale request results are ignored.",
                            cx.theme().background,
                            cx,
                        )),
                ),
        )
}

fn action_bar(state: &HttpLabState) -> Div {
    div()
        .flex()
        .flex_wrap()
        .gap_2()
        .children(HttpLabAction::all().iter().copied().map(|action| {
            let resource = state.resource(action);
            action_button(action, resource)
        }))
        .child(
            Button::new("http-lab-cancel-all")
                .outline()
                .label("Cancel all")
                .disabled(state.active_count() == 0)
                .on_click(|_, _, cx| {
                    http_lab::cancel_all(cx);
                }),
        )
        .child(
            Button::new("http-lab-reset")
                .outline()
                .label("Reset")
                .on_click(|_, _, cx| {
                    http_lab::reset(cx);
                }),
        )
}

fn action_button(action: HttpLabAction, resource: &QueryResource<HttpExchange>) -> Button {
    let is_loading = resource.is_loading();
    Button::new(format!("http-lab-run-{}", action.id()))
        .outline()
        .label(if is_loading {
            format!("Loading {}", action.label())
        } else {
            action.label().to_string()
        })
        .loading(is_loading)
        .disabled(is_loading)
        .on_click(move |_, _, cx| {
            http_lab::run_action(action, cx);
        })
}

fn tab_bar(state: &HttpLabState) -> Div {
    div()
        .flex()
        .flex_wrap()
        .gap_1()
        .children(HttpLabAction::all().iter().copied().map(|action| {
            let resource = state.resource(action);
            Button::new(format!("http-lab-tab-{}", action.id()))
                .ghost()
                .selected(state.selected_action == action)
                .label(format!(
                    "{} {}",
                    status_dot(resource.status()),
                    action.label()
                ))
                .on_click(move |_, _, cx| {
                    http_lab::select_action(action, cx);
                })
        }))
}

fn resource_panel(
    state: &HttpLabState,
    action: HttpLabAction,
    resource: &QueryResource<HttpExchange>,
    cx: &App,
) -> Div {
    panel(action.label(), cx)
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(status_chip(resource.status(), cx))
                .child(chip(action.method_label(), cx.theme().background, cx))
                .child(chip(
                    &resource.cache_policy().label(),
                    cx.theme().background,
                    cx,
                ))
                .child(chip(
                    resource.request_policy().label(),
                    cx.theme().background,
                    cx,
                ))
                .when_some(resource.active_request_id(), |this, request_id| {
                    this.child(chip(
                        &format!("request #{}", request_id.value()),
                        cx.theme().background,
                        cx,
                    ))
                }),
        )
        .child(resource_metrics(resource, cx))
        .when(resource.is_loading(), |this| {
            this.child(
                Button::new(format!("http-lab-cancel-{}", action.id()))
                    .danger()
                    .outline()
                    .label("Cancel request")
                    .on_click(move |_, _, cx| {
                        http_lab::cancel_action(action, cx);
                    }),
            )
        })
        .when_some(resource.error(), |this, error| {
            this.child(callout("Error", error, cx))
        })
        .when_some(resource.data(), |this, exchange| {
            this.child(exchange_panel(exchange, cx))
        })
        .when(resource.data().is_none(), |this| {
            this.child(empty_state(resource.status(), cx))
        })
        .when(action == HttpLabAction::Cookies, |this| {
            this.when_some(state.cookies.as_ref(), |this, cookies| {
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
        })
}

fn resource_metrics(resource: &QueryResource<HttpExchange>, cx: &App) -> Div {
    div()
        .grid()
        .gap_2()
        .child(kv("Cache hits", &resource.cache_hits().to_string(), cx))
        .child(kv("Cancelled", &resource.cancelled_count().to_string(), cx))
        .child(kv(
            "Ignored stale results",
            &resource.ignored_results().to_string(),
            cx,
        ))
        .child(kv(
            "Last update",
            &resource
                .last_updated_at_ms()
                .map(|value| value.to_string())
                .unwrap_or_else(|| "Never".to_string()),
            cx,
        ))
}

fn activity_panel(state: &HttpLabState, cx: &App) -> Div {
    div()
        .grid()
        .gap_4()
        .child(
            panel("Lifecycle trace", cx).children(state.transition_log.iter().map(|entry| {
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(entry.clone())
            })),
        )
        .child(
            panel("History", cx).children(state.history.iter().take(10).map(|exchange| {
                let fields = http_lab::response_fields(exchange);
                let summary = fields
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join(" | ");
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(summary)
            })),
        )
}

fn exchange_panel(exchange: &HttpExchange, cx: &App) -> Div {
    let mut panel = panel("Response", cx)
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
        this.child(callout("Response error", error, cx))
    })
}

fn headers_block(headers: &[(String, String)], cx: &App) -> Div {
    panel("Headers", cx).children(
        headers
            .iter()
            .take(16)
            .map(|(name, value)| kv(name, value, cx)),
    )
}

fn empty_state(status: QueryStatus, cx: &App) -> Div {
    div()
        .p_5()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .text_color(cx.theme().muted_foreground)
        .child(match status {
            QueryStatus::LoadingEmpty => "Request is loading without cached data.",
            QueryStatus::Cancelled => "Request was cancelled before a response was applied.",
            _ => "No response captured for this tab yet.",
        })
}

fn panel(title: &str, cx: &App) -> Div {
    v_flex()
        .gap_3()
        .p_4()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .child(section_title(title))
}

fn section_title(title: &str) -> Div {
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
                .min_w(px(150.))
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
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::BOLD)
                .child(label.to_string()),
        )
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

fn callout(title: &str, message: &str, cx: &App) -> Div {
    div()
        .p_3()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().danger)
        .bg(cx.theme().danger.opacity(0.08))
        .child(
            v_flex()
                .gap_1()
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_color(cx.theme().danger)
                        .child(title.to_string()),
                )
                .child(div().text_sm().child(message.to_string())),
        )
}

fn chip(label: &str, background: Hsla, cx: &App) -> Div {
    div()
        .px_3()
        .py_1()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .bg(background)
        .text_sm()
        .child(label.to_string())
}

fn status_chip(status: QueryStatus, cx: &App) -> Div {
    let background = match status {
        QueryStatus::Success => cx.theme().success.opacity(0.12),
        QueryStatus::Failure => cx.theme().danger.opacity(0.12),
        QueryStatus::Cancelled => cx.theme().warning.opacity(0.12),
        QueryStatus::LoadingEmpty | QueryStatus::LoadingWithData => cx.theme().info.opacity(0.12),
        QueryStatus::Idle => cx.theme().muted,
    };
    chip(status.label(), background, cx)
}

fn status_dot(status: QueryStatus) -> &'static str {
    match status {
        QueryStatus::Idle => "[ ]",
        QueryStatus::LoadingEmpty | QueryStatus::LoadingWithData => "[~]",
        QueryStatus::Success => "[+]",
        QueryStatus::Failure => "[x]",
        QueryStatus::Cancelled => "!",
    }
}
