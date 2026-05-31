use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use gpui::{App, BorrowAppContext as _, Global};
use reqwest::blocking::{Client, multipart};
use reqwest::header::{CONTENT_TYPE, HeaderMap};
use serde::{Deserialize, Serialize};

use crate::ids::TaskId;
use crate::services::query::{
    CachePolicy, QueryBeginResult, QueryError, QueryFetchMode, QueryKey, QueryResource,
    QueryStatus, RequestGuard, RequestId, RequestPolicy, RequestSequencer,
};
use crate::tasks::TaskProgress;

const HTTPBIN_BASE: &str = "https://httpbin.org";
const TIMEOUT: Duration = Duration::from_secs(15);
const GET_CACHE_TTL_MS: u64 = 60_000;
const REVALIDATE_TTL_MS: u64 = 30_000;

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
    pub selected_action: HttpLabAction,
    resources: BTreeMap<HttpLabAction, QueryResource<HttpExchange>>,
    request_sequencer: RequestSequencer,
    inflight_tasks: BTreeMap<RequestId, TaskId>,
    pub history: Vec<HttpExchange>,
    pub transition_log: Vec<String>,
    pub cookies: Option<HttpCookieSnapshot>,
}

impl Default for HttpLabState {
    fn default() -> Self {
        let mut resources = BTreeMap::new();
        for action in HttpLabAction::all() {
            resources.insert(*action, resource_for_action(*action));
        }

        Self {
            selected_action: HttpLabAction::GetJson,
            resources,
            request_sequencer: RequestSequencer::new(),
            inflight_tasks: BTreeMap::new(),
            history: Vec::new(),
            transition_log: vec!["Idle".to_string()],
            cookies: None,
        }
    }
}

impl HttpLabState {
    pub fn resource(&self, action: HttpLabAction) -> &QueryResource<HttpExchange> {
        self.resources
            .get(&action)
            .expect("all http lab actions must have resources")
    }

    pub fn selected_resource(&self) -> &QueryResource<HttpExchange> {
        self.resource(self.selected_action)
    }

    pub fn active_count(&self) -> usize {
        self.resources
            .values()
            .filter(|resource| resource.active_request_id().is_some())
            .count()
    }

    fn reset_for_user(&mut self) -> ResetRequests {
        let cancelled_requests = self.inflight_tasks.keys().copied().collect::<Vec<_>>();
        let cancelled_tasks = self.inflight_tasks.values().copied().collect::<Vec<_>>();
        let mut request_sequencer = self.request_sequencer.clone();
        request_sequencer.advance_scope();
        *self = Self::default();
        self.request_sequencer = request_sequencer;
        ResetRequests {
            request_ids: cancelled_requests,
            task_ids: cancelled_tasks,
        }
    }
}

impl Global for HttpLabState {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ResetRequests {
    request_ids: Vec<RequestId>,
    task_ids: Vec<TaskId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
    pub fn all() -> &'static [Self] {
        &[
            Self::GetText,
            Self::GetJson,
            Self::GetXml,
            Self::PostJson,
            Self::PostForm,
            Self::PostMultipart,
            Self::Cookies,
            Self::Failure,
            Self::FullFlow,
        ]
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::GetText => "get_text",
            Self::GetJson => "get_json",
            Self::GetXml => "get_xml",
            Self::PostJson => "post_json",
            Self::PostForm => "post_form",
            Self::PostMultipart => "post_multipart",
            Self::Cookies => "cookies",
            Self::Failure => "failure",
            Self::FullFlow => "full_flow",
        }
    }

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

    pub fn method_label(self) -> &'static str {
        match self {
            Self::GetText | Self::GetJson | Self::GetXml | Self::Cookies | Self::Failure => "GET",
            Self::PostJson | Self::PostForm | Self::PostMultipart => "POST",
            Self::FullFlow => "FLOW",
        }
    }

    pub fn query_key(self) -> QueryKey {
        QueryKey::new(format!("http_lab/{}", self.id()))
    }

    fn cache_policy(self) -> CachePolicy {
        match self {
            Self::GetText | Self::GetXml => CachePolicy::Ttl {
                ttl_ms: GET_CACHE_TTL_MS,
            },
            Self::GetJson => CachePolicy::StaleWhileRevalidate {
                ttl_ms: REVALIDATE_TTL_MS,
            },
            Self::PostJson
            | Self::PostForm
            | Self::PostMultipart
            | Self::Cookies
            | Self::Failure
            | Self::FullFlow => CachePolicy::NoCache,
        }
    }

    fn request_policy(self) -> RequestPolicy {
        match self {
            Self::PostMultipart | Self::FullFlow => RequestPolicy::IgnoreWhileLoading,
            _ => RequestPolicy::LatestWins,
        }
    }
}

fn resource_for_action(action: HttpLabAction) -> QueryResource<HttpExchange> {
    QueryResource::new(
        action.query_key(),
        action.cache_policy(),
        action.request_policy(),
    )
}

type ActionExchange = (HttpLabAction, HttpExchange);

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

pub fn read_state<R>(cx: &App, read: impl FnOnce(&HttpLabState) -> R) -> R {
    if let Some(state) = cx.try_global::<HttpLabState>() {
        read(state)
    } else {
        let fallback = HttpLabState::default();
        read(&fallback)
    }
}

pub fn reset(cx: &mut App) {
    let reset_requests = if cx.try_global::<HttpLabState>().is_some() {
        cx.update_global::<HttpLabState, _>(|state, _cx| state.reset_for_user())
    } else {
        cx.set_global(HttpLabState::default());
        ResetRequests::default()
    };

    for request_id in reset_requests.request_ids {
        cancel_request_flag(request_id);
    }
    for task_id in reset_requests.task_ids {
        crate::tasks::cancel(task_id, "HTTP Lab reset".to_string(), cx);
    }
}

pub fn select_action(action: HttpLabAction, cx: &mut App) {
    cx.update_global::<HttpLabState, _>(|state, _cx| {
        state.selected_action = action;
    });
}

pub fn run_action(action: HttpLabAction, cx: &mut App) {
    let now_ms = now_ms();
    let request_id =
        cx.update_global::<HttpLabState, _>(|state, _cx| begin_action(state, action, now_ms));

    let Some(request_id) = request_id else {
        return;
    };

    let task_id = TaskId::new();
    crate::tasks::start(
        task_id,
        format!("HTTP Lab {}", action.label()),
        TaskProgress::Indeterminate,
        cx,
    );
    cx.update_global::<HttpLabState, _>(|state, _cx| {
        state.inflight_tasks.insert(request_id, task_id);
    });
    let cancellation = register_request_flag(request_id);

    cx.spawn(async move |cx| {
        let result = cx
            .background_executor()
            .spawn(async move { run_blocking_action(action, cancellation) })
            .await;

        cx.update(move |cx| {
            apply_result(action, request_id, result, cx);
        });
    })
    .detach();
}

pub fn cancel_action(action: HttpLabAction, cx: &mut App) {
    cx.update_global::<HttpLabState, _>(|state, _cx| {
        cancel_action_in_state(state, action, "Cancelled by user");
    });
}

pub fn cancel_all(cx: &mut App) {
    cx.update_global::<HttpLabState, _>(|state, _cx| {
        cancel_all_in_state(state, "Cancelled by user");
    });
}

fn begin_action(
    state: &mut HttpLabState,
    action: HttpLabAction,
    now_ms: u128,
) -> Option<RequestId> {
    state.selected_action = action;

    if action == HttpLabAction::FullFlow {
        cancel_all_in_state(state, "Cancelled by full flow");
    } else {
        cancel_action_in_state(
            state,
            HttpLabAction::FullFlow,
            "Cancelled by individual request",
        );
    }

    let has_data = state.resource(action).has_data();
    let cache_policy = state.resource(action).cache_policy();
    let begin_result = {
        let HttpLabState {
            resources,
            request_sequencer,
            ..
        } = state;
        let resource = resources.get_mut(&action)?;
        resource.begin_request(request_sequencer, now_ms, QueryFetchMode::Normal)
    };

    match begin_result {
        QueryBeginResult::Started {
            request_id,
            status,
            replaced_request_id,
        } => {
            if replaced_request_id.is_some() {
                record_transition(
                    state,
                    QueryStatus::Cancelled,
                    action.label(),
                    "cancelled by newer request",
                );
            }
            let note = match (cache_policy, has_data) {
                (CachePolicy::StaleWhileRevalidate { .. }, true) => "revalidating cached data",
                _ => "request started",
            };
            record_transition(state, status, action.label(), note);
            Some(request_id)
        }
        QueryBeginResult::CacheHit => {
            record_transition(state, QueryStatus::Success, action.label(), "cache hit");
            None
        }
        QueryBeginResult::IgnoredWhileLoading { .. } => {
            record_transition(
                state,
                state.resource(action).status(),
                action.label(),
                "ignored duplicate while loading",
            );
            None
        }
    }
}

fn apply_result(
    action: HttpLabAction,
    request_id: RequestId,
    result: Result<Vec<ActionExchange>, String>,
    cx: &mut App,
) {
    let now_ms = now_ms();
    let task_update = cx.update_global::<HttpLabState, _>(|state, _cx| {
        apply_result_to_state(state, action, request_id, result, now_ms)
    });
    apply_task_update(task_update, cx);
}

fn apply_result_to_state(
    state: &mut HttpLabState,
    action: HttpLabAction,
    request_id: RequestId,
    result: Result<Vec<ActionExchange>, String>,
    now_ms: u128,
) -> Option<HttpTaskUpdate> {
    let task_id = state.inflight_tasks.remove(&request_id);
    remove_request_flag(request_id);
    if !state.request_sequencer.is_current_scope(request_id) {
        return Some(HttpTaskUpdate::cancelled(
            task_id,
            "ignored stale request scope".to_string(),
        ));
    }

    let Some(request_guard) = state
        .resources
        .get_mut(&action)
        .and_then(|resource| resource.accept_current_request(request_id))
    else {
        record_ignored_result(state, action, request_id);
        return Some(HttpTaskUpdate::cancelled(
            task_id,
            format!("ignored stale request {}", request_id.label()),
        ));
    };

    match result {
        Ok(exchanges) => {
            let last_exchange = exchanges.last().map(|(_, exchange)| exchange.clone());
            for (index, (target_action, exchange)) in exchanges.into_iter().enumerate() {
                if action == HttpLabAction::FullFlow && index > 0 {
                    record_transition(
                        state,
                        QueryStatus::LoadingWithData,
                        target_action.label(),
                        "full flow advanced",
                    );
                }
                finish_exchange(state, target_action, exchange, &request_guard, now_ms);
            }

            if action == HttpLabAction::FullFlow {
                finish_flow_resource(state, last_exchange, &request_guard, now_ms);
            }
            Some(HttpTaskUpdate::succeeded(task_id))
        }
        Err(error) => {
            fail_resource(state, action, &request_guard, error.clone());
            Some(HttpTaskUpdate::failed(task_id, error))
        }
    }
}

fn finish_exchange(
    state: &mut HttpLabState,
    action: HttpLabAction,
    exchange: HttpExchange,
    request_guard: &RequestGuard,
    now_ms: u128,
) {
    let status = if exchange.error.is_none() {
        QueryStatus::Success
    } else {
        QueryStatus::Failure
    };
    let error = exchange.error.clone();

    if let Some(resource) = state.resources.get_mut(&action) {
        match error {
            Some(error) => {
                resource.complete_failure_with_data(
                    request_guard,
                    exchange.clone(),
                    QueryError::response(error),
                );
            }
            None => resource.complete_success(request_guard, exchange.clone(), now_ms),
        }
    }

    if let Some(cookie_snapshot) = cookie_snapshot_from_exchange(&exchange) {
        state.cookies = Some(cookie_snapshot);
    }

    record_transition(state, status, action.label(), "response applied");
    push_history(state, exchange);
}

fn finish_flow_resource(
    state: &mut HttpLabState,
    last_exchange: Option<HttpExchange>,
    request_guard: &RequestGuard,
    now_ms: u128,
) {
    if let Some(resource) = state.resources.get_mut(&HttpLabAction::FullFlow) {
        resource.complete_success_optional(request_guard, last_exchange, now_ms);
    }
    record_transition(
        state,
        QueryStatus::Success,
        HttpLabAction::FullFlow.label(),
        "flow completed",
    );
}

fn fail_resource(
    state: &mut HttpLabState,
    action: HttpLabAction,
    request_guard: &RequestGuard,
    error: String,
) {
    let exchange = HttpExchange {
        label: action.label().to_string(),
        request: HttpRequestSnapshot {
            method: "-".to_string(),
            url: HTTPBIN_BASE.to_string(),
            request_body_kind: HttpRequestBodyKind::None,
            request_body_preview: String::new(),
        },
        response: None,
        error: Some(error.clone()),
    };

    if let Some(resource) = state.resources.get_mut(&action) {
        resource.complete_failure(request_guard, QueryError::transport(error));
    }

    record_transition(
        state,
        QueryStatus::Failure,
        action.label(),
        "request failed",
    );
    push_history(state, exchange);
}

fn cancel_action_in_state(state: &mut HttpLabState, action: HttpLabAction, reason: &str) {
    if let Some(request_id) = state.resource(action).active_request_id() {
        cancel_request_flag(request_id);
        if let Some(resource) = state.resources.get_mut(&action) {
            resource.cancel(QueryError::cancelled(reason));
        }
        record_transition(state, QueryStatus::Cancelled, action.label(), reason);
    }
}

fn cancel_all_in_state(state: &mut HttpLabState, reason: &str) {
    let actions = HttpLabAction::all()
        .iter()
        .copied()
        .filter(|action| state.resource(*action).active_request_id().is_some())
        .collect::<Vec<_>>();
    for action in actions {
        cancel_action_in_state(state, action, reason);
    }
}

fn record_ignored_result(state: &mut HttpLabState, action: HttpLabAction, request_id: RequestId) {
    record_transition(
        state,
        QueryStatus::Cancelled,
        action.label(),
        &format!("ignored stale request {}", request_id.label()),
    );
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HttpTaskStatus {
    Succeeded,
    Failed(String),
    Cancelled(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HttpTaskUpdate {
    task_id: Option<TaskId>,
    status: HttpTaskStatus,
}

impl HttpTaskUpdate {
    fn succeeded(task_id: Option<TaskId>) -> Self {
        Self {
            task_id,
            status: HttpTaskStatus::Succeeded,
        }
    }

    fn failed(task_id: Option<TaskId>, error: String) -> Self {
        Self {
            task_id,
            status: HttpTaskStatus::Failed(error),
        }
    }

    fn cancelled(task_id: Option<TaskId>, reason: String) -> Self {
        Self {
            task_id,
            status: HttpTaskStatus::Cancelled(reason),
        }
    }
}

fn apply_task_update(update: Option<HttpTaskUpdate>, cx: &mut App) {
    let Some(HttpTaskUpdate { task_id, status }) = update else {
        return;
    };
    let Some(task_id) = task_id else {
        return;
    };
    match status {
        HttpTaskStatus::Succeeded => crate::tasks::succeed(task_id, cx),
        HttpTaskStatus::Failed(error) => crate::tasks::fail(task_id, error, cx),
        HttpTaskStatus::Cancelled(reason) => crate::tasks::cancel(task_id, reason, cx),
    }
}

fn cancellation_flags() -> &'static Mutex<BTreeMap<RequestId, Arc<AtomicBool>>> {
    static FLAGS: OnceLock<Mutex<BTreeMap<RequestId, Arc<AtomicBool>>>> = OnceLock::new();
    FLAGS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn register_request_flag(request_id: RequestId) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut flags) = cancellation_flags().lock() {
        flags.insert(request_id, flag.clone());
    }
    flag
}

fn cancel_request_flag(request_id: RequestId) {
    if let Ok(flags) = cancellation_flags().lock()
        && let Some(flag) = flags.get(&request_id)
    {
        flag.store(true, Ordering::SeqCst);
    }
}

fn remove_request_flag(request_id: RequestId) {
    if let Ok(mut flags) = cancellation_flags().lock() {
        flags.remove(&request_id);
    }
}

fn record_transition(state: &mut HttpLabState, status: QueryStatus, label: &str, note: &str) {
    state
        .transition_log
        .insert(0, format!("{}: {label} ({note})", status.label()));
    state.transition_log.truncate(24);
}

fn push_history(state: &mut HttpLabState, exchange: HttpExchange) {
    state.history.insert(0, exchange);
    state.history.truncate(24);
}

fn run_blocking_action(
    action: HttpLabAction,
    cancellation: Arc<AtomicBool>,
) -> Result<Vec<ActionExchange>, String> {
    fail_if_cancelled(&cancellation)?;
    match action {
        HttpLabAction::FullFlow => run_full_flow(cancellation),
        _ => run_single(action).map(|exchange| vec![(action, exchange)]),
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
        HttpLabAction::Cookies => run_cookies_with_client(&client),
        HttpLabAction::Failure => execute_get(
            &client,
            "Failure",
            &format!("{HTTPBIN_BASE}/status/500"),
            HttpBodyKind::Text,
        ),
        HttpLabAction::FullFlow => unreachable!(),
    }
}

fn run_full_flow(cancellation: Arc<AtomicBool>) -> Result<Vec<ActionExchange>, String> {
    let client = client()?;
    let get_json = execute_get(
        &client,
        "GET JSON",
        &format!("{HTTPBIN_BASE}/json"),
        HttpBodyKind::Json,
    )?;
    fail_if_shutdown_requested()?;
    fail_if_cancelled(&cancellation)?;
    let failure = execute_get(
        &client,
        "Failure",
        &format!("{HTTPBIN_BASE}/status/503"),
        HttpBodyKind::Text,
    )?;
    fail_if_shutdown_requested()?;
    fail_if_cancelled(&cancellation)?;
    let post_json = execute_post_json(&client)?;
    fail_if_shutdown_requested()?;
    fail_if_cancelled(&cancellation)?;
    let cookies = run_cookies_with_client(&client)?;

    Ok(vec![
        (HttpLabAction::GetJson, get_json),
        (HttpLabAction::Failure, failure),
        (HttpLabAction::PostJson, post_json),
        (HttpLabAction::Cookies, cookies),
    ])
}

fn fail_if_cancelled(cancellation: &AtomicBool) -> Result<(), String> {
    if cancellation.load(Ordering::SeqCst) {
        Err("cancelled by user".to_string())
    } else {
        Ok(())
    }
}

fn fail_if_shutdown_requested() -> Result<(), String> {
    if crate::tasks::is_shutdown_requested() {
        Err("cancelled during app shutdown".to_string())
    } else {
        Ok(())
    }
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

fn now_ms() -> u128 {
    static STARTED_AT: OnceLock<Instant> = OnceLock::new();
    STARTED_AT.get_or_init(Instant::now).elapsed().as_millis()
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
#[path = "http_lab.test.rs"]
mod http_lab_test;
