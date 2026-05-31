use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use gpui::{App, BorrowAppContext as _, Global};
use reqwest::blocking::{Client, multipart};
use reqwest::header::{CONTENT_TYPE, HeaderMap};
use serde::{Deserialize, Serialize};

const HTTPBIN_BASE: &str = "https://httpbin.org";
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpBodyKind {
    Text,
    Json,
    Xml,
}

impl HttpBodyKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Xml => "xml",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpRequestBodyKind {
    None,
    Json,
    FormUrlEncoded,
    MultipartFormData,
}

impl HttpRequestBodyKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Json => "json",
            Self::FormUrlEncoded => "form-urlencoded",
            Self::MultipartFormData => "multipart/form-data",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpDemoStatus {
    Idle,
    LoadingEmpty,
    Success,
    Failure,
    LoadingWithState,
}

impl HttpDemoStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Idle => "IDLE",
            Self::LoadingEmpty => "Loading EMPTY State",
            Self::Success => "Success",
            Self::Failure => "Failure",
            Self::LoadingWithState => "Loading with state",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpRequestSnapshot {
    pub method: String,
    pub url: String,
    pub request_body_kind: HttpRequestBodyKind,
    pub request_body_preview: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpResponseSnapshot {
    pub status: u16,
    pub status_text: String,
    pub final_url: String,
    pub elapsed_ms: u128,
    pub headers: Vec<(String, String)>,
    pub body_kind: HttpBodyKind,
    pub body_preview: String,
    pub parsed_json: Option<String>,
    pub parsed_xml_preview: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpExchange {
    pub label: String,
    pub request: HttpRequestSnapshot,
    pub response: Option<HttpResponseSnapshot>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpCookieSnapshot {
    pub set_cookie_header: Option<String>,
    pub echoed_cookies_json: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpLabState {
    pub status: HttpDemoStatus,
    pub active_label: Option<String>,
    pub last_success: Option<HttpExchange>,
    pub last_error: Option<HttpExchange>,
    pub history: Vec<HttpExchange>,
    pub transition_log: Vec<String>,
    pub cookies: Option<HttpCookieSnapshot>,
}

impl Default for HttpLabState {
    fn default() -> Self {
        Self {
            status: HttpDemoStatus::Idle,
            active_label: None,
            last_success: None,
            last_error: None,
            history: Vec::new(),
            transition_log: vec![HttpDemoStatus::Idle.label().to_string()],
            cookies: None,
        }
    }
}

impl Global for HttpLabState {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpLabAction {
    GetText,
    GetJson,
    GetXml,
    PostJson,
    PostForm,
    PostMultipart,
    Cookies,
    Failure,
    FullFlow,
}

impl HttpLabAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::GetText => "GET text",
            Self::GetJson => "GET JSON",
            Self::GetXml => "GET XML",
            Self::PostJson => "POST JSON",
            Self::PostForm => "POST form",
            Self::PostMultipart => "POST multipart",
            Self::Cookies => "Cookies",
            Self::Failure => "Failure",
            Self::FullFlow => "Run full flow",
        }
    }
}

pub fn initialize(cx: &mut App) {
    cx.set_global(HttpLabState::default());
    crate::capabilities::set(
        "http_lab",
        crate::capabilities::CapabilityStatus::supported_enabled(),
        cx,
    );
}

pub fn snapshot(cx: &App) -> HttpLabState {
    cx.try_global::<HttpLabState>().cloned().unwrap_or_default()
}

pub fn reset(cx: &mut App) {
    cx.set_global(HttpLabState::default());
}

pub fn run_action(action: HttpLabAction, cx: &mut App) {
    let snapshot = snapshot(cx);
    let loading_status = loading_status_for(action, &snapshot);
    cx.update_global::<HttpLabState, _>(|state, _cx| {
        begin_action(state, action, loading_status);
    });

    cx.spawn(async move |cx| {
        let result = cx
            .background_executor()
            .spawn(async move { run_blocking_action(action) })
            .await;

        cx.update(move |cx| {
            apply_result(action, result, cx);
        });
    })
    .detach();
}

fn loading_status_for(action: HttpLabAction, state: &HttpLabState) -> HttpDemoStatus {
    if action == HttpLabAction::FullFlow {
        HttpDemoStatus::LoadingEmpty
    } else if state.last_success.is_some() || state.last_error.is_some() {
        HttpDemoStatus::LoadingWithState
    } else {
        HttpDemoStatus::LoadingEmpty
    }
}

fn begin_action(state: &mut HttpLabState, action: HttpLabAction, loading_status: HttpDemoStatus) {
    if action == HttpLabAction::FullFlow {
        *state = HttpLabState::default();
    }
    state.status = loading_status;
    state.active_label = Some(action.label().to_string());
    record_transition(state, loading_status, action.label());
}

fn apply_result(action: HttpLabAction, result: Result<Vec<HttpExchange>, String>, cx: &mut App) {
    cx.update_global::<HttpLabState, _>(|state, _cx| {
        apply_result_to_state(state, action, result);
    });
}

fn apply_result_to_state(
    state: &mut HttpLabState,
    action: HttpLabAction,
    result: Result<Vec<HttpExchange>, String>,
) {
    state.active_label = None;
    match result {
        Ok(exchanges) => {
            for (index, exchange) in exchanges.into_iter().enumerate() {
                if action == HttpLabAction::FullFlow && index > 0 {
                    record_transition(state, HttpDemoStatus::LoadingWithState, &exchange.label);
                }
                let is_success = exchange.error.is_none();
                if is_success {
                    state.status = HttpDemoStatus::Success;
                    state.last_success = Some(exchange.clone());
                    state.last_error = None;
                    record_transition(state, HttpDemoStatus::Success, &exchange.label);
                    if let Some(cookie_snapshot) = cookie_snapshot_from_exchange(&exchange) {
                        state.cookies = Some(cookie_snapshot);
                    }
                } else {
                    state.status = HttpDemoStatus::Failure;
                    state.last_error = Some(exchange.clone());
                    record_transition(state, HttpDemoStatus::Failure, &exchange.label);
                }
                push_history(state, exchange);
            }
        }
        Err(error) => {
            let exchange = HttpExchange {
                label: action.label().to_string(),
                request: HttpRequestSnapshot {
                    method: "-".to_string(),
                    url: HTTPBIN_BASE.to_string(),
                    request_body_kind: HttpRequestBodyKind::None,
                    request_body_preview: String::new(),
                },
                response: None,
                error: Some(error),
            };
            state.status = HttpDemoStatus::Failure;
            state.last_error = Some(exchange.clone());
            record_transition(state, HttpDemoStatus::Failure, action.label());
            push_history(state, exchange);
        }
    }
}

fn record_transition(state: &mut HttpLabState, status: HttpDemoStatus, label: &str) {
    state
        .transition_log
        .insert(0, format!("{}: {label}", status.label()));
    state.transition_log.truncate(20);
}

fn push_history(state: &mut HttpLabState, exchange: HttpExchange) {
    state.history.insert(0, exchange);
    state.history.truncate(12);
}

fn run_blocking_action(action: HttpLabAction) -> Result<Vec<HttpExchange>, String> {
    match action {
        HttpLabAction::FullFlow => run_full_flow(),
        HttpLabAction::Cookies => run_cookies().map(|exchange| vec![exchange]),
        _ => run_single(action).map(|exchange| vec![exchange]),
    }
}

fn client() -> Result<Client, String> {
    Client::builder()
        .timeout(TIMEOUT)
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| err.to_string())
}

fn run_single(action: HttpLabAction) -> Result<HttpExchange, String> {
    let client = client()?;
    match action {
        HttpLabAction::GetText => execute_get(
            &client,
            "GET text",
            &format!("{HTTPBIN_BASE}/encoding/utf8"),
            HttpBodyKind::Text,
        ),
        HttpLabAction::GetJson => execute_get(
            &client,
            "GET JSON",
            &format!("{HTTPBIN_BASE}/json"),
            HttpBodyKind::Json,
        ),
        HttpLabAction::GetXml => execute_get(
            &client,
            "GET XML",
            &format!("{HTTPBIN_BASE}/xml"),
            HttpBodyKind::Xml,
        ),
        HttpLabAction::PostJson => execute_post_json(&client),
        HttpLabAction::PostForm => execute_post_form(&client),
        HttpLabAction::PostMultipart => execute_post_multipart(&client),
        HttpLabAction::Failure => execute_get(
            &client,
            "Failure",
            &format!("{HTTPBIN_BASE}/status/500"),
            HttpBodyKind::Text,
        ),
        HttpLabAction::Cookies | HttpLabAction::FullFlow => unreachable!(),
    }
}

fn run_full_flow() -> Result<Vec<HttpExchange>, String> {
    let client = client()?;
    let success_empty = execute_get(
        &client,
        "Flow success empty",
        &format!("{HTTPBIN_BASE}/json"),
        HttpBodyKind::Json,
    )?;
    let failure_with_state = execute_get(
        &client,
        "Flow failure with cached state",
        &format!("{HTTPBIN_BASE}/status/503"),
        HttpBodyKind::Text,
    )?;
    let success_with_state = execute_post_json(&client)?;
    let cookies = run_cookies_with_client(&client)?;
    Ok(vec![
        success_empty,
        failure_with_state,
        success_with_state,
        cookies,
    ])
}

fn execute_get(
    client: &Client,
    label: &str,
    url: &str,
    expected_kind: HttpBodyKind,
) -> Result<HttpExchange, String> {
    let started = Instant::now();
    let request = HttpRequestSnapshot {
        method: "GET".to_string(),
        url: url.to_string(),
        request_body_kind: HttpRequestBodyKind::None,
        request_body_preview: String::new(),
    };
    let response = client
        .get(url)
        .header("accept", content_type_for(expected_kind))
        .send()
        .map_err(|err| err.to_string())?;
    response_to_exchange(label, request, response, started, expected_kind)
}

fn execute_post_json(client: &Client) -> Result<HttpExchange, String> {
    let url = format!("{HTTPBIN_BASE}/post");
    let payload = serde_json::json!({
        "name": "gpui-starter",
        "mode": "json",
        "nested": { "source": "httpbin" }
    });
    let request = HttpRequestSnapshot {
        method: "POST".to_string(),
        url: url.clone(),
        request_body_kind: HttpRequestBodyKind::Json,
        request_body_preview: serde_json::to_string_pretty(&payload).unwrap_or_default(),
    };
    let started = Instant::now();
    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .map_err(|err| err.to_string())?;
    response_to_exchange("POST JSON", request, response, started, HttpBodyKind::Json)
}

fn execute_post_form(client: &Client) -> Result<HttpExchange, String> {
    let url = format!("{HTTPBIN_BASE}/post");
    let form = [("framework", "gpui"), ("body", "form-urlencoded")];
    let request = HttpRequestSnapshot {
        method: "POST".to_string(),
        url: url.clone(),
        request_body_kind: HttpRequestBodyKind::FormUrlEncoded,
        request_body_preview: "framework=gpui&body=form-urlencoded".to_string(),
    };
    let started = Instant::now();
    let response = client
        .post(&url)
        .form(&form)
        .send()
        .map_err(|err| err.to_string())?;
    response_to_exchange("POST form", request, response, started, HttpBodyKind::Json)
}

fn execute_post_multipart(client: &Client) -> Result<HttpExchange, String> {
    let url = format!("{HTTPBIN_BASE}/post");
    let file_part = multipart::Part::text("hello from gpui-starter\n")
        .file_name("gpui-starter.txt")
        .mime_str("text/plain")
        .map_err(|err| err.to_string())?;
    let form = multipart::Form::new()
        .text("framework", "gpui")
        .text("body", "multipart")
        .part("file", file_part);
    let request = HttpRequestSnapshot {
        method: "POST".to_string(),
        url: url.clone(),
        request_body_kind: HttpRequestBodyKind::MultipartFormData,
        request_body_preview: "framework=gpui, body=multipart, file=gpui-starter.txt".to_string(),
    };
    let started = Instant::now();
    let response = client
        .post(&url)
        .multipart(form)
        .send()
        .map_err(|err| err.to_string())?;
    response_to_exchange(
        "POST multipart",
        request,
        response,
        started,
        HttpBodyKind::Json,
    )
}

fn run_cookies() -> Result<HttpExchange, String> {
    let client = client()?;
    run_cookies_with_client(&client)
}

fn run_cookies_with_client(client: &Client) -> Result<HttpExchange, String> {
    let set_url = format!("{HTTPBIN_BASE}/cookies/set/session/gpui-starter");
    let set_response = client.get(&set_url).send().map_err(|err| err.to_string())?;
    let set_cookie_header = set_response
        .headers()
        .iter()
        .find(|(name, _)| name.as_str().eq_ignore_ascii_case("set-cookie"))
        .map(|(_, value)| value.to_str().unwrap_or("<non-utf8>").to_string());

    let mut exchange = execute_get(
        client,
        "Cookies",
        &format!("{HTTPBIN_BASE}/cookies"),
        HttpBodyKind::Json,
    )?;

    if let (Some(response), Some(set_cookie_header)) =
        (exchange.response.as_mut(), set_cookie_header)
    {
        response
            .headers
            .push(("set-cookie".to_string(), set_cookie_header));
    }

    Ok(exchange)
}

fn response_to_exchange(
    label: &str,
    request: HttpRequestSnapshot,
    response: reqwest::blocking::Response,
    started: Instant,
    expected_kind: HttpBodyKind,
) -> Result<HttpExchange, String> {
    let status = response.status();
    let final_url = response.url().to_string();
    let headers = headers_to_vec(response.headers());
    let body_kind = detect_body_kind(response.headers(), expected_kind);
    let body = response.text().map_err(|err| err.to_string())?;
    let response_snapshot = HttpResponseSnapshot {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("unknown").to_string(),
        final_url,
        elapsed_ms: started.elapsed().as_millis(),
        headers,
        body_kind,
        body_preview: truncate(&body, 8_000),
        parsed_json: parse_json(&body),
        parsed_xml_preview: parse_xml_preview(&body, body_kind),
    };
    let error = (!status.is_success()).then(|| format!("HTTP {}", status.as_u16()));
    Ok(HttpExchange {
        label: label.to_string(),
        request,
        response: Some(response_snapshot),
        error,
    })
}

fn headers_to_vec(headers: &HeaderMap) -> Vec<(String, String)> {
    let mut values = headers
        .iter()
        .map(|(name, value)| {
            (
                name.to_string(),
                value.to_str().unwrap_or("<non-utf8>").to_string(),
            )
        })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left.0.cmp(&right.0));
    values
}

fn detect_body_kind(headers: &HeaderMap, expected_kind: HttpBodyKind) -> HttpBodyKind {
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if content_type.contains("json") {
        HttpBodyKind::Json
    } else if content_type.contains("xml") {
        HttpBodyKind::Xml
    } else {
        expected_kind
    }
}

fn content_type_for(kind: HttpBodyKind) -> &'static str {
    match kind {
        HttpBodyKind::Text => "text/plain, */*",
        HttpBodyKind::Json => "application/json",
        HttpBodyKind::Xml => "application/xml, text/xml",
    }
}

fn parse_json(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .map(|value| truncate(&value, 8_000))
}

fn parse_xml_preview(body: &str, kind: HttpBodyKind) -> Option<String> {
    (kind == HttpBodyKind::Xml).then(|| truncate(body, 4_000))
}

fn truncate(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        value.to_string()
    } else {
        let end = value
            .char_indices()
            .map(|(index, _)| index)
            .take_while(|index| *index <= max_len)
            .last()
            .unwrap_or(0);
        format!("{}…", &value[..end])
    }
}

fn cookie_snapshot_from_exchange(exchange: &HttpExchange) -> Option<HttpCookieSnapshot> {
    if exchange.label != "Cookies" {
        return None;
    }
    let response = exchange.response.as_ref()?;
    let set_cookie_header = response
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, value)| value.clone());
    Some(HttpCookieSnapshot {
        set_cookie_header,
        echoed_cookies_json: response.parsed_json.clone(),
    })
}

pub fn response_fields(exchange: &HttpExchange) -> BTreeMap<&'static str, String> {
    let mut fields = BTreeMap::new();
    fields.insert("label", exchange.label.clone());
    fields.insert("method", exchange.request.method.clone());
    fields.insert("url", exchange.request.url.clone());
    fields.insert(
        "request_body",
        exchange.request.request_body_kind.label().to_string(),
    );
    if let Some(response) = &exchange.response {
        fields.insert("status", response.status.to_string());
        fields.insert("body_kind", response.body_kind.label().to_string());
        fields.insert("elapsed_ms", response.elapsed_ms.to_string());
    }
    if let Some(error) = &exchange.error {
        fields.insert("error", error.clone());
    }
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exchange(label: &str, status: u16, error: Option<&str>) -> HttpExchange {
        HttpExchange {
            label: label.to_string(),
            request: HttpRequestSnapshot {
                method: "GET".to_string(),
                url: format!("{HTTPBIN_BASE}/test"),
                request_body_kind: HttpRequestBodyKind::None,
                request_body_preview: String::new(),
            },
            response: Some(HttpResponseSnapshot {
                status,
                status_text: "test".to_string(),
                final_url: format!("{HTTPBIN_BASE}/test"),
                elapsed_ms: 1,
                headers: Vec::new(),
                body_kind: HttpBodyKind::Text,
                body_preview: String::new(),
                parsed_json: None,
                parsed_xml_preview: None,
            }),
            error: error.map(str::to_string),
        }
    }

    #[test]
    fn loading_with_state_counts_prior_failure() {
        let state = HttpLabState {
            last_error: Some(exchange("Failure", 500, Some("HTTP 500"))),
            ..HttpLabState::default()
        };
        assert_eq!(
            loading_status_for(HttpLabAction::GetJson, &state),
            HttpDemoStatus::LoadingWithState
        );
    }

    #[test]
    fn full_flow_starts_from_empty_even_when_cached() {
        let mut state = HttpLabState {
            last_success: Some(exchange("Success", 200, None)),
            last_error: Some(exchange("Failure", 500, Some("HTTP 500"))),
            ..HttpLabState::default()
        };
        let loading_status = loading_status_for(HttpLabAction::FullFlow, &state);

        begin_action(&mut state, HttpLabAction::FullFlow, loading_status);

        assert_eq!(state.status, HttpDemoStatus::LoadingEmpty);
        assert!(state.last_success.is_none());
        assert!(state.last_error.is_none());
        assert_eq!(state.active_label.as_deref(), Some("Run full flow"));
    }

    #[test]
    fn successful_exchange_clears_stale_failure() {
        let mut state = HttpLabState {
            last_error: Some(exchange("Failure", 500, Some("HTTP 500"))),
            active_label: Some("GET JSON".to_string()),
            ..HttpLabState::default()
        };
        let success = exchange("Success", 200, None);

        apply_result_to_state(&mut state, HttpLabAction::GetJson, Ok(vec![success]));

        assert_eq!(state.status, HttpDemoStatus::Success);
        assert!(state.last_success.is_some());
        assert!(state.last_error.is_none());
        assert!(state.active_label.is_none());
        assert_eq!(state.history.len(), 1);
    }

    #[test]
    fn failed_exchange_preserves_previous_success_as_cached_state() {
        let previous_success = exchange("Success", 200, None);
        let mut state = HttpLabState {
            last_success: Some(previous_success.clone()),
            active_label: Some("Failure".to_string()),
            ..HttpLabState::default()
        };
        let failure = exchange("Failure", 500, Some("HTTP 500"));

        apply_result_to_state(&mut state, HttpLabAction::Failure, Ok(vec![failure]));

        assert_eq!(state.status, HttpDemoStatus::Failure);
        assert_eq!(state.last_success, Some(previous_success));
        assert!(state.last_error.is_some());
        assert!(state.active_label.is_none());
        assert_eq!(state.history.len(), 1);
    }

    #[test]
    fn full_flow_records_intermediate_loading_with_state_transitions() {
        let mut state = HttpLabState::default();
        let exchanges = vec![
            exchange("Flow success empty", 200, None),
            exchange("Flow failure with cached state", 503, Some("HTTP 503")),
            exchange("POST JSON", 200, None),
        ];

        apply_result_to_state(&mut state, HttpLabAction::FullFlow, Ok(exchanges));

        assert_eq!(state.status, HttpDemoStatus::Success);
        assert_eq!(state.history.len(), 3);
        assert!(
            state
                .transition_log
                .iter()
                .any(|entry| entry == "Loading with state: Flow failure with cached state")
        );
        assert!(
            state
                .transition_log
                .iter()
                .any(|entry| entry == "Loading with state: POST JSON")
        );
    }

    #[test]
    fn cookies_exchange_updates_cookie_snapshot() {
        let mut cookies = exchange("Cookies", 200, None);
        let response = cookies.response.as_mut().expect("response");
        response
            .headers
            .push(("set-cookie".to_string(), "session=gpui-starter".to_string()));
        response.parsed_json = Some("{\"cookies\":{\"session\":\"gpui-starter\"}}".to_string());
        let mut state = HttpLabState::default();

        apply_result_to_state(&mut state, HttpLabAction::Cookies, Ok(vec![cookies]));

        let cookies = state.cookies.expect("cookie snapshot");
        assert_eq!(
            cookies.set_cookie_header.as_deref(),
            Some("session=gpui-starter")
        );
        assert_eq!(
            cookies.echoed_cookies_json.as_deref(),
            Some("{\"cookies\":{\"session\":\"gpui-starter\"}}")
        );
    }

    #[test]
    fn parse_helpers_split_json_xml_and_text() {
        assert!(parse_json("{\"ok\":true}").is_some());
        assert!(parse_json("<root />").is_none());
        assert!(parse_xml_preview("<root />", HttpBodyKind::Xml).is_some());
        assert!(parse_xml_preview("<html />", HttpBodyKind::Text).is_none());
    }

    #[test]
    fn truncate_preserves_utf8_boundaries() {
        let value = "hello ڄاڻ world";
        let truncated = truncate(value, 9);
        assert!(truncated.ends_with('…'));
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
    }
}
