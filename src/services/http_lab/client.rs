use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use reqwest::blocking::{Client, multipart};

use crate::services::http_lab::{
    response::{content_type_for, response_to_exchange},
    types::{
        ActionExchange, HttpBodyKind, HttpExchange, HttpLabAction, HttpRequestBodyKind,
        HttpRequestSnapshot,
    },
};

pub(super) const HTTPBIN_BASE: &str = "https://httpbin.org";
const TIMEOUT: Duration = Duration::from_secs(15);

pub(super) fn run_blocking_action(
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
