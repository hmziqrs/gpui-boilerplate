use std::{sync::OnceLock, time::Instant};

use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, Disableable as _,
    button::{Button, ButtonVariants as _},
    v_flex,
};
use tokio_util::sync::CancellationToken;

use crate::services::{
    query::{
        CachePolicy, QueryBeginResult, QueryError, QueryFetchMode, QueryResource, RequestPolicy,
        RequestSequencer,
    },
    tokio_runtime::TokioRuntimeGlobal,
};

const LOG: &str = "gpui_starter::http_lab_testing";
const TEST_URL: &str = "https://httpbingo.org/get";
const PREVIEW_LIMIT: usize = 8_000;

#[derive(Clone, Debug)]
enum RawStatus {
    Idle,
    Sending,
    Completed,
    Failed,
    Cancelled,
}

impl RawStatus {
    fn label(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Sending => "Sending",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RawResponse {
    status: u16,
    final_url: String,
    header_count: usize,
    bytes: usize,
    preview: String,
}

pub struct HttpLabTestingPage {
    next_operation_id: u64,
    active_operation_id: Option<u64>,
    cancellation: Option<CancellationToken>,
    status: RawStatus,
    last_message: String,
    last_response: Option<RawResponse>,
    query_resource: QueryResource<RawResponse>,
    query_ttl_resource: QueryResource<RawResponse>,
    query_ignore_resource: QueryResource<RawResponse>,
    query_latest_resource: QueryResource<RawResponse>,
    query_sequencer: RequestSequencer,
    query_message: String,
}

impl HttpLabTestingPage {
    pub fn new() -> Self {
        Self {
            next_operation_id: 1,
            active_operation_id: None,
            cancellation: None,
            status: RawStatus::Idle,
            last_message: "No request sent yet.".to_string(),
            last_response: None,
            query_resource: QueryResource::new(
                "http_lab_testing/raw_query",
                CachePolicy::NoCache,
                RequestPolicy::LatestWins,
            ),
            query_ttl_resource: QueryResource::new(
                "http_lab_testing/ttl_query",
                CachePolicy::Ttl { ttl_ms: 30_000 },
                RequestPolicy::LatestWins,
            ),
            query_ignore_resource: QueryResource::new(
                "http_lab_testing/ignore_query",
                CachePolicy::NoCache,
                RequestPolicy::IgnoreWhileLoading,
            ),
            query_latest_resource: QueryResource::new(
                "http_lab_testing/latest_query",
                CachePolicy::NoCache,
                RequestPolicy::LatestWins,
            ),
            query_sequencer: RequestSequencer::new(),
            query_message: "No query request sent yet.".to_string(),
        }
    }

    fn send_raw_get(&mut self, cx: &mut Context<Self>) {
        let operation_id = self.next_operation_id;
        self.next_operation_id += 1;

        if let Some(token) = self.cancellation.take() {
            token.cancel();
        }

        let cancellation = CancellationToken::new();
        self.active_operation_id = Some(operation_id);
        self.cancellation = Some(cancellation.clone());
        self.status = RawStatus::Sending;
        self.last_message = format!("operation {operation_id}: dispatching raw GET");
        self.last_response = None;
        cx.notify();

        let runtime = cx.global::<TokioRuntimeGlobal>().0.runtime.clone();
        let client = cx.global::<TokioRuntimeGlobal>().0.http_client.clone();
        let url = TEST_URL.to_string();

        tracing::info!(
            target: LOG,
            operation_id,
            url,
            "HTTP Lab Testing scheduling entity foreground task"
        );

        cx.spawn(async move |this, cx| {
            tracing::info!(
                target: LOG,
                operation_id,
                "HTTP Lab Testing foreground task started"
            );

            let started = Instant::now();
            let request_cancellation = cancellation.clone();
            let handle = runtime.spawn(async move {
                raw_reqwest_get(client, url, request_cancellation, operation_id).await
            });

            tracing::info!(
                target: LOG,
                operation_id,
                "HTTP Lab Testing Tokio request task spawned"
            );

            let result = match handle.await {
                Ok(result) => result,
                Err(err) => Err(format!("tokio task join failed: {err}")),
            };

            let elapsed_ms = started.elapsed().as_millis();
            tracing::info!(
                target: LOG,
                operation_id,
                elapsed_ms,
                ok = result.is_ok(),
                "HTTP Lab Testing foreground task joined Tokio result"
            );

            this.update(cx, |this, cx| {
                if this.active_operation_id != Some(operation_id) {
                    tracing::info!(
                        target: LOG,
                        operation_id,
                        active_operation_id = ?this.active_operation_id,
                        "HTTP Lab Testing ignoring stale operation result"
                    );
                    return;
                }

                this.active_operation_id = None;
                this.cancellation = None;

                match result {
                    Ok(response) => {
                        this.status = RawStatus::Completed;
                        this.last_message =
                            format!("operation {operation_id}: completed in {elapsed_ms}ms");
                        this.last_response = Some(response);
                    }
                    Err(err) if err == "cancelled" => {
                        this.status = RawStatus::Cancelled;
                        this.last_message =
                            format!("operation {operation_id}: cancelled after {elapsed_ms}ms");
                        this.last_response = None;
                    }
                    Err(err) => {
                        this.status = RawStatus::Failed;
                        this.last_message =
                            format!("operation {operation_id}: failed after {elapsed_ms}ms: {err}");
                        this.last_response = None;
                    }
                }

                tracing::info!(
                    target: LOG,
                    operation_id,
                    status = this.status.label(),
                    "HTTP Lab Testing applying operation result"
                );
                cx.notify();
            })
            .ok();
        })
        .detach();

        tracing::info!(
            target: LOG,
            operation_id,
            "HTTP Lab Testing entity foreground task scheduled"
        );
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        let Some(operation_id) = self.active_operation_id.take() else {
            return;
        };

        if let Some(token) = self.cancellation.take() {
            token.cancel();
        }

        self.status = RawStatus::Cancelled;
        self.last_message = format!("operation {operation_id}: cancel requested");
        self.last_response = None;
        tracing::info!(
            target: LOG,
            operation_id,
            "HTTP Lab Testing cancellation requested"
        );
        cx.notify();
    }

    fn send_query_get(&mut self, cx: &mut Context<Self>) {
        let operation_id = self.next_operation_id;
        self.next_operation_id += 1;

        if let Some(token) = self.cancellation.take() {
            token.cancel();
        }

        let query_started_ms = query_now_ms();
        tracing::info!(
            target: LOG,
            operation_id,
            query_started_ms,
            "HTTP Lab Testing query begin_request entered"
        );
        let request_id = match self.query_resource.begin_request(
            &mut self.query_sequencer,
            query_started_ms,
            QueryFetchMode::Normal,
        ) {
            QueryBeginResult::Started {
                request_id,
                status,
                replaced_request_id,
            } => {
                tracing::info!(
                    target: LOG,
                    operation_id,
                    request_id = %request_id.label(),
                    status = status.label(),
                    replaced_request_id = ?replaced_request_id.map(|id| id.label()),
                    "HTTP Lab Testing query request started"
                );
                request_id
            }
            QueryBeginResult::CacheHit => {
                self.query_message = "query cache hit".to_string();
                tracing::info!(
                    target: LOG,
                    operation_id,
                    "HTTP Lab Testing query cache hit"
                );
                cx.notify();
                return;
            }
            QueryBeginResult::IgnoredWhileLoading { active_request_id } => {
                self.query_message =
                    format!("query ignored while loading {}", active_request_id.label());
                tracing::info!(
                    target: LOG,
                    operation_id,
                    active_request_id = %active_request_id.label(),
                    "HTTP Lab Testing query ignored while loading"
                );
                cx.notify();
                return;
            }
        };

        let cancellation = CancellationToken::new();
        self.active_operation_id = Some(operation_id);
        self.cancellation = Some(cancellation.clone());
        self.status = RawStatus::Sending;
        self.last_message = format!("operation {operation_id}: dispatching query GET");
        self.query_message = format!(
            "operation {operation_id}: query request {} loading",
            request_id.label()
        );
        self.last_response = None;
        cx.notify();

        let runtime = cx.global::<TokioRuntimeGlobal>().0.runtime.clone();
        let client = cx.global::<TokioRuntimeGlobal>().0.http_client.clone();
        let url = TEST_URL.to_string();

        tracing::info!(
            target: LOG,
            operation_id,
            request_id = %request_id.label(),
            url,
            "HTTP Lab Testing scheduling query foreground task"
        );

        cx.spawn(async move |this, cx| {
            tracing::info!(
                target: LOG,
                operation_id,
                request_id = %request_id.label(),
                "HTTP Lab Testing query foreground task started"
            );

            let started = Instant::now();
            let request_cancellation = cancellation.clone();
            let handle = runtime.spawn(async move {
                raw_reqwest_get(client, url, request_cancellation, operation_id).await
            });

            tracing::info!(
                target: LOG,
                operation_id,
                request_id = %request_id.label(),
                "HTTP Lab Testing query Tokio request task spawned"
            );

            let result = match handle.await {
                Ok(result) => result,
                Err(err) => Err(format!("tokio task join failed: {err}")),
            };

            let elapsed_ms = started.elapsed().as_millis();
            tracing::info!(
                target: LOG,
                operation_id,
                request_id = %request_id.label(),
                elapsed_ms,
                ok = result.is_ok(),
                "HTTP Lab Testing query foreground task joined Tokio result"
            );

            if let Err(err) = this.update(cx, |this, cx| {
                if this.active_operation_id != Some(operation_id) {
                    tracing::info!(
                        target: LOG,
                        operation_id,
                        request_id = %request_id.label(),
                        active_operation_id = ?this.active_operation_id,
                        "HTTP Lab Testing query ignoring stale operation result"
                    );
                    return;
                }

                this.active_operation_id = None;
                this.cancellation = None;

                match result {
                    Ok(response) => {
                        let completed = this.query_resource.complete_current_success(
                            request_id,
                            response.clone(),
                            query_now_ms(),
                        );
                        this.status = RawStatus::Completed;
                        this.last_message =
                            format!("operation {operation_id}: query completed in {elapsed_ms}ms");
                        this.query_message = format!(
                            "operation {operation_id}: query complete accepted={completed}"
                        );
                        this.last_response = Some(response);
                    }
                    Err(err) if err == "cancelled" => {
                        let cancelled = this
                            .query_resource
                            .complete_current_failure(request_id, QueryError::cancelled(err));
                        this.status = RawStatus::Cancelled;
                        this.last_message =
                            format!("operation {operation_id}: query cancelled after {elapsed_ms}ms");
                        this.query_message = format!(
                            "operation {operation_id}: query cancel accepted={cancelled}"
                        );
                        this.last_response = None;
                    }
                    Err(err) => {
                        let completed = this
                            .query_resource
                            .complete_current_failure(request_id, QueryError::transport(err.clone()));
                        this.status = RawStatus::Failed;
                        this.last_message =
                            format!("operation {operation_id}: query failed after {elapsed_ms}ms: {err}");
                        this.query_message = format!(
                            "operation {operation_id}: query failure accepted={completed}"
                        );
                        this.last_response = None;
                    }
                }

                tracing::info!(
                    target: LOG,
                    operation_id,
                    request_id = %request_id.label(),
                    status = this.query_resource.status().label(),
                    active_request_id = ?this.query_resource.active_request_id().map(|id| id.label()),
                    "HTTP Lab Testing applying query result"
                );
                cx.notify();
            }) {
                tracing::warn!(
                    target: LOG,
                    operation_id,
                    request_id = %request_id.label(),
                    error = %err,
                    "HTTP Lab Testing failed to apply query result"
                );
            }
        })
        .detach();

        tracing::info!(
            target: LOG,
            operation_id,
            request_id = %request_id.label(),
            "HTTP Lab Testing query foreground task scheduled"
        );
    }

    fn exercise_query_ttl_cache(&mut self, cx: &mut Context<Self>) {
        let started_ms = query_now_ms();
        let first = self.query_ttl_resource.begin_request(
            &mut self.query_sequencer,
            started_ms,
            QueryFetchMode::Normal,
        );

        let QueryBeginResult::Started { request_id, .. } = first else {
            self.query_message = format!("TTL setup did not start: {first:?}");
            cx.notify();
            return;
        };

        let accepted = self.query_ttl_resource.complete_current_success(
            request_id,
            fake_response("ttl-cache"),
            started_ms + 1,
        );
        let second = self.query_ttl_resource.begin_request(
            &mut self.query_sequencer,
            started_ms + 2,
            QueryFetchMode::Normal,
        );
        let cache_hit = matches!(second, QueryBeginResult::CacheHit);
        self.query_message = format!(
            "TTL cache probe: first={} accepted={accepted} second_cache_hit={cache_hit}",
            request_id.label()
        );

        tracing::info!(
            target: LOG,
            request_id = %request_id.label(),
            accepted,
            cache_hit,
            status = self.query_ttl_resource.status().label(),
            "HTTP Lab Testing TTL query cache probe completed"
        );
        cx.notify();
    }

    fn exercise_query_ignore_while_loading(&mut self, cx: &mut Context<Self>) {
        let now_ms = query_now_ms();
        let first = self.query_ignore_resource.begin_request(
            &mut self.query_sequencer,
            now_ms,
            QueryFetchMode::Normal,
        );

        let QueryBeginResult::Started { request_id, .. } = first else {
            self.query_message = format!("Ignore setup did not start: {first:?}");
            cx.notify();
            return;
        };

        let second = self.query_ignore_resource.begin_request(
            &mut self.query_sequencer,
            now_ms + 1,
            QueryFetchMode::Normal,
        );
        let ignored = matches!(
            second,
            QueryBeginResult::IgnoredWhileLoading { active_request_id }
                if active_request_id == request_id
        );
        let cancelled = self
            .query_ignore_resource
            .cancel(QueryError::cancelled("ignore probe cleanup"));
        self.query_message = format!(
            "Ignore probe: first={} duplicate_ignored={ignored} cleanup_cancelled={cancelled}",
            request_id.label()
        );

        tracing::info!(
            target: LOG,
            request_id = %request_id.label(),
            ignored,
            cancelled,
            status = self.query_ignore_resource.status().label(),
            "HTTP Lab Testing ignore-while-loading query probe completed"
        );
        cx.notify();
    }

    fn exercise_query_latest_wins(&mut self, cx: &mut Context<Self>) {
        let now_ms = query_now_ms();
        let first = self.query_latest_resource.begin_request(
            &mut self.query_sequencer,
            now_ms,
            QueryFetchMode::Normal,
        );
        let QueryBeginResult::Started {
            request_id: first_id,
            ..
        } = first
        else {
            self.query_message = format!("Latest setup did not start: {first:?}");
            cx.notify();
            return;
        };

        let second = self.query_latest_resource.begin_request(
            &mut self.query_sequencer,
            now_ms + 1,
            QueryFetchMode::Normal,
        );
        let QueryBeginResult::Started {
            request_id: second_id,
            replaced_request_id,
            ..
        } = second
        else {
            self.query_message = format!("Latest replacement did not start: {second:?}");
            cx.notify();
            return;
        };

        let stale_accepted = self.query_latest_resource.complete_current_success(
            first_id,
            fake_response("latest-stale"),
            now_ms + 2,
        );
        let latest_accepted = self.query_latest_resource.complete_current_success(
            second_id,
            fake_response("latest-current"),
            now_ms + 3,
        );
        self.query_message = format!(
            "Latest probe: first={} second={} replaced={:?} stale_accepted={stale_accepted} latest_accepted={latest_accepted}",
            first_id.label(),
            second_id.label(),
            replaced_request_id.map(|id| id.label())
        );

        tracing::info!(
            target: LOG,
            first_id = %first_id.label(),
            second_id = %second_id.label(),
            replaced_request_id = ?replaced_request_id.map(|id| id.label()),
            stale_accepted,
            latest_accepted,
            status = self.query_latest_resource.status().label(),
            "HTTP Lab Testing latest-wins query probe completed"
        );
        cx.notify();
    }
}

impl Render for HttpLabTestingPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_sending = matches!(self.status, RawStatus::Sending);

        v_flex()
            .min_h_full()
            .p_6()
            .gap_5()
            .child(
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
                                    .child("HTTP Lab Testing"),
                            )
                            .child(
                                div()
                                    .max_w(px(760.))
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Raw reqwest-only screen for isolating GPUI task scheduling from the existing HTTP Lab store and gpui-query path."),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new("http-lab-testing-send")
                            .outline()
                            .label(if is_sending { "Sending raw GET" } else { "Send raw GET" })
                            .disabled(is_sending)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.send_raw_get(cx);
                            })),
                    )
                    .child(
                        Button::new("http-lab-testing-query-send")
                            .outline()
                            .label(if is_sending {
                                "Sending query GET"
                            } else {
                                "Send query GET"
                            })
                            .disabled(is_sending)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.send_query_get(cx);
                            })),
                    )
                    .child(
                        Button::new("http-lab-testing-query-ttl")
                            .outline()
                            .label("Query TTL")
                            .disabled(is_sending)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.exercise_query_ttl_cache(cx);
                            })),
                    )
                    .child(
                        Button::new("http-lab-testing-query-ignore")
                            .outline()
                            .label("Query ignore")
                            .disabled(is_sending)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.exercise_query_ignore_while_loading(cx);
                            })),
                    )
                    .child(
                        Button::new("http-lab-testing-query-latest")
                            .outline()
                            .label("Query latest")
                            .disabled(is_sending)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.exercise_query_latest_wins(cx);
                            })),
                    )
                    .child(
                        Button::new("http-lab-testing-cancel")
                            .danger()
                            .outline()
                            .label("Cancel")
                            .disabled(!is_sending)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cancel(cx);
                            })),
                    ),
            )
            .child(status_panel(self, cx))
            .child(query_panel(self, cx))
            .child(response_panel(self, cx))
    }
}

async fn raw_reqwest_get(
    client: reqwest::Client,
    url: String,
    cancellation: CancellationToken,
    operation_id: u64,
) -> Result<RawResponse, String> {
    tracing::info!(
        target: LOG,
        operation_id,
        url,
        "HTTP Lab Testing raw request build started"
    );

    let request = client
        .get(&url)
        .header("accept", "application/json")
        .header("x-gpui-http-lab-testing", operation_id.to_string());

    tracing::info!(
        target: LOG,
        operation_id,
        "HTTP Lab Testing raw request send started"
    );

    let send_started = Instant::now();
    let mut response = tokio::select! {
        biased;
        _ = cancellation.cancelled() => {
            tracing::info!(
                target: LOG,
                operation_id,
                "HTTP Lab Testing raw request cancelled before response"
            );
            return Err("cancelled".to_string());
        }
        result = request.send() => result.map_err(|err| err.to_string())?,
    };

    let send_elapsed_ms = send_started.elapsed().as_millis();
    let status = response.status().as_u16();
    let final_url = response.url().to_string();
    let header_count = response.headers().len();

    tracing::info!(
        target: LOG,
        operation_id,
        status,
        final_url,
        header_count,
        send_elapsed_ms,
        "HTTP Lab Testing raw request send completed"
    );

    let mut bytes = Vec::new();
    let body_started = Instant::now();
    loop {
        if bytes.len() >= PREVIEW_LIMIT {
            break;
        }

        let chunk = tokio::select! {
            biased;
            _ = cancellation.cancelled() => {
                tracing::info!(
                    target: LOG,
                    operation_id,
                    bytes = bytes.len(),
                    "HTTP Lab Testing raw body cancelled"
                );
                return Err("cancelled".to_string());
            }
            result = response.chunk() => result.map_err(|err| err.to_string())?,
        };

        let Some(chunk) = chunk else {
            break;
        };

        let remaining = PREVIEW_LIMIT - bytes.len();
        bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
    }

    let body_elapsed_ms = body_started.elapsed().as_millis();
    let preview = String::from_utf8_lossy(&bytes).to_string();

    tracing::info!(
        target: LOG,
        operation_id,
        bytes = bytes.len(),
        body_elapsed_ms,
        "HTTP Lab Testing raw body preview completed"
    );

    Ok(RawResponse {
        status,
        final_url,
        header_count,
        bytes: bytes.len(),
        preview,
    })
}

fn status_panel(page: &HttpLabTestingPage, cx: &App) -> Div {
    div()
        .p_4()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .child(
            v_flex()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Raw request state"),
                )
                .child(row("Status", page.status.label(), cx))
                .child(row(
                    "Active operation",
                    &page
                        .active_operation_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                    cx,
                ))
                .child(row("Message", &page.last_message, cx)),
        )
}

fn response_panel(page: &HttpLabTestingPage, cx: &App) -> Div {
    let panel = div()
        .p_4()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .child(
            v_flex()
                .gap_3()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Response"),
                )
                .when_some(page.last_response.as_ref(), |this, response| {
                    this.child(row("Status", &response.status.to_string(), cx))
                        .child(row("Final URL", &response.final_url, cx))
                        .child(row("Headers", &response.header_count.to_string(), cx))
                        .child(row("Preview bytes", &response.bytes.to_string(), cx))
                        .child(
                            div()
                                .p_3()
                                .rounded(cx.theme().radius)
                                .bg(cx.theme().muted)
                                .text_xs()
                                .font_family("monospace")
                                .child(response.preview.clone()),
                        )
                }),
        );

    if page.last_response.is_some() {
        panel
    } else {
        panel.child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("No response captured."),
        )
    }
}

fn query_panel(page: &HttpLabTestingPage, cx: &App) -> Div {
    div()
        .p_4()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .child(
            v_flex()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Minimal query state"),
                )
                .child(row(
                    "Query status",
                    page.query_resource.status().label(),
                    cx,
                ))
                .child(row(
                    "Query active",
                    &page
                        .query_resource
                        .active_request_id()
                        .map(|id| id.label())
                        .unwrap_or_else(|| "none".to_string()),
                    cx,
                ))
                .child(row(
                    "Query data",
                    if page.query_resource.data().is_some() {
                        "present"
                    } else {
                        "none"
                    },
                    cx,
                ))
                .child(row(
                    "Query error",
                    page.query_resource
                        .error()
                        .map(QueryError::message)
                        .unwrap_or("none"),
                    cx,
                ))
                .child(query_resource_row("TTL", &page.query_ttl_resource, cx))
                .child(query_resource_row(
                    "Ignore",
                    &page.query_ignore_resource,
                    cx,
                ))
                .child(query_resource_row(
                    "Latest",
                    &page.query_latest_resource,
                    cx,
                ))
                .child(row("Query message", &page.query_message, cx)),
        )
}

fn query_resource_row(label: &str, resource: &QueryResource<RawResponse>, cx: &App) -> Div {
    let active = resource
        .active_request_id()
        .map(|id| id.label())
        .unwrap_or_else(|| "none".to_string());
    let data = if resource.data().is_some() {
        "data"
    } else {
        "no data"
    };
    row(
        label,
        &format!("{} active={} {}", resource.status().label(), active, data),
        cx,
    )
}

fn row(label: &str, value: &str, cx: &App) -> Div {
    div()
        .flex()
        .gap_3()
        .items_start()
        .child(
            div()
                .w(px(140.))
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(cx.theme().muted_foreground)
                .child(label.to_string()),
        )
        .child(div().flex_1().text_sm().child(value.to_string()))
}

fn query_now_ms() -> u128 {
    static STARTED_AT: OnceLock<Instant> = OnceLock::new();
    STARTED_AT.get_or_init(Instant::now).elapsed().as_millis()
}

fn fake_response(label: &str) -> RawResponse {
    RawResponse {
        status: 200,
        final_url: format!("memory://{label}"),
        header_count: 0,
        bytes: label.len(),
        preview: label.to_string(),
    }
}
