use reqwest::{Client, multipart};
use tokio_util::sync::CancellationToken;

use crate::services::http_lab::{
    response::{content_type_for, response_to_exchange},
    types::{
        ActionExchange, HttpBodyKind, HttpExchange, HttpLabAction, HttpRequestBodyKind,
        HttpRequestSnapshot,
    },
};

pub(super) const HTTP_LAB_BASE: &str = "https://httpbingo.org";
const LOG: &str = "gpui_starter::http_lab::client";

/// Run an HTTP action. Must be called inside a tokio runtime context.
pub(super) async fn run_http_action(
    client: &Client,
    action: HttpLabAction,
    cancellation: CancellationToken,
) -> Result<Vec<ActionExchange>, String> {
    tracing::info!(
        target: LOG,
        action = action.id(),
        cancelled = cancellation.is_cancelled(),
        "HTTP Lab run_http_action entered"
    );
    fail_if_cancelled(&cancellation)?;
    let result = match action {
        HttpLabAction::FullFlow => run_full_flow(client, cancellation).await,
        _ => run_single(client, action, cancellation)
            .await
            .map(|exchange| vec![(action, exchange)]),
    };
    tracing::info!(
        target: LOG,
        action = action.id(),
        ok = result.is_ok(),
        "HTTP Lab run_http_action exiting"
    );
    result
}

async fn run_single(
    client: &Client,
    action: HttpLabAction,
    cancellation: CancellationToken,
) -> Result<HttpExchange, String> {
    match action {
        HttpLabAction::GetText => {
            execute_get(
                client,
                "GET text",
                &format!("{HTTP_LAB_BASE}/encoding/utf8"),
                HttpBodyKind::Text,
                cancellation,
            )
            .await
        }
        HttpLabAction::GetJson => {
            execute_get(
                client,
                "GET JSON",
                &format!("{HTTP_LAB_BASE}/json"),
                HttpBodyKind::Json,
                cancellation,
            )
            .await
        }
        HttpLabAction::GetXml => {
            execute_get(
                client,
                "GET XML",
                &format!("{HTTP_LAB_BASE}/xml"),
                HttpBodyKind::Xml,
                cancellation,
            )
            .await
        }
        HttpLabAction::PostJson => execute_post_json(client, cancellation).await,
        HttpLabAction::PostForm => execute_post_form(client, cancellation).await,
        HttpLabAction::PostMultipart => execute_post_multipart(client, cancellation).await,
        HttpLabAction::Cookies => run_cookies_with_client(client, cancellation).await,
        HttpLabAction::Failure => {
            execute_get(
                client,
                "Failure",
                &format!("{HTTP_LAB_BASE}/status/500"),
                HttpBodyKind::Text,
                cancellation,
            )
            .await
        }
        HttpLabAction::FullFlow => unreachable!(),
    }
}

async fn run_full_flow(
    client: &Client,
    cancellation: CancellationToken,
) -> Result<Vec<ActionExchange>, String> {
    tracing::info!(target: LOG, "HTTP Lab full flow started");
    let get_json = execute_get(
        client,
        "GET JSON",
        &format!("{HTTP_LAB_BASE}/json"),
        HttpBodyKind::Json,
        cancellation.clone(),
    )
    .await?;
    fail_if_shutdown_requested()?;
    fail_if_cancelled(&cancellation)?;
    let failure = execute_get(
        client,
        "Failure",
        &format!("{HTTP_LAB_BASE}/status/503"),
        HttpBodyKind::Text,
        cancellation.clone(),
    )
    .await?;
    fail_if_shutdown_requested()?;
    fail_if_cancelled(&cancellation)?;
    let post_json = execute_post_json(client, cancellation.clone()).await?;
    fail_if_shutdown_requested()?;
    fail_if_cancelled(&cancellation)?;
    let cookies = run_cookies_with_client(client, cancellation).await?;

    tracing::info!(target: LOG, "HTTP Lab full flow completed");
    Ok(vec![
        (HttpLabAction::GetJson, get_json),
        (HttpLabAction::Failure, failure),
        (HttpLabAction::PostJson, post_json),
        (HttpLabAction::Cookies, cookies),
    ])
}

fn fail_if_cancelled(cancellation: &CancellationToken) -> Result<(), String> {
    if cancellation.is_cancelled() {
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

async fn execute_get(
    client: &Client,
    label: &str,
    url: &str,
    expected_kind: HttpBodyKind,
    cancellation: CancellationToken,
) -> Result<HttpExchange, String> {
    let started = std::time::Instant::now();
    tracing::info!(
        target: LOG,
        method = "GET",
        label,
        url,
        "HTTP Lab request build started"
    );
    let request = HttpRequestSnapshot {
        method: "GET".to_string(),
        url: url.to_string(),
        request_body_kind: HttpRequestBodyKind::None,
        request_body_preview: String::new(),
    };
    let response = send_with_cancel(
        client
            .get(url)
            .header("accept", content_type_for(expected_kind)),
        "GET",
        label,
        url,
        cancellation.clone(),
    )
    .await?;
    response_to_exchange(
        label,
        request,
        response,
        started,
        expected_kind,
        cancellation,
    )
    .await
}

async fn execute_post_json(
    client: &Client,
    cancellation: CancellationToken,
) -> Result<HttpExchange, String> {
    let url = format!("{HTTP_LAB_BASE}/post");
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
    let started = std::time::Instant::now();
    tracing::info!(
        target: LOG,
        method = "POST",
        label = "POST JSON",
        url,
        "HTTP Lab request build started"
    );
    let response = send_with_cancel(
        client.post(&url).json(&payload),
        "POST",
        "POST JSON",
        &url,
        cancellation.clone(),
    )
    .await?;
    response_to_exchange(
        "POST JSON",
        request,
        response,
        started,
        HttpBodyKind::Json,
        cancellation,
    )
    .await
}

async fn execute_post_form(
    client: &Client,
    cancellation: CancellationToken,
) -> Result<HttpExchange, String> {
    let url = format!("{HTTP_LAB_BASE}/post");
    let form = [("framework", "gpui"), ("body", "form-urlencoded")];
    let request = HttpRequestSnapshot {
        method: "POST".to_string(),
        url: url.clone(),
        request_body_kind: HttpRequestBodyKind::FormUrlEncoded,
        request_body_preview: "framework=gpui&body=form-urlencoded".to_string(),
    };
    let started = std::time::Instant::now();
    tracing::info!(
        target: LOG,
        method = "POST",
        label = "POST form",
        url,
        "HTTP Lab request build started"
    );
    let response = send_with_cancel(
        client.post(&url).form(&form),
        "POST",
        "POST form",
        &url,
        cancellation.clone(),
    )
    .await?;
    response_to_exchange(
        "POST form",
        request,
        response,
        started,
        HttpBodyKind::Json,
        cancellation,
    )
    .await
}

async fn execute_post_multipart(
    client: &Client,
    cancellation: CancellationToken,
) -> Result<HttpExchange, String> {
    let url = format!("{HTTP_LAB_BASE}/post");
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
    let started = std::time::Instant::now();
    tracing::info!(
        target: LOG,
        method = "POST",
        label = "POST multipart",
        url,
        "HTTP Lab request build started"
    );
    let response = send_with_cancel(
        client.post(&url).multipart(form),
        "POST",
        "POST multipart",
        &url,
        cancellation.clone(),
    )
    .await?;
    response_to_exchange(
        "POST multipart",
        request,
        response,
        started,
        HttpBodyKind::Json,
        cancellation,
    )
    .await
}

async fn run_cookies_with_client(
    client: &Client,
    cancellation: CancellationToken,
) -> Result<HttpExchange, String> {
    let set_url = format!("{HTTP_LAB_BASE}/response-headers?Set-Cookie=session%3Dgpui-starter");
    tracing::info!(
        target: LOG,
        method = "GET",
        label = "Cookies setup",
        url = set_url,
        "HTTP Lab cookie setup request build started"
    );
    let set_response = send_with_cancel(
        client.get(&set_url),
        "GET",
        "Cookies setup",
        &set_url,
        cancellation.clone(),
    )
    .await?;
    let set_cookie_header = set_response
        .headers()
        .iter()
        .find(|(name, _)| name.as_str().eq_ignore_ascii_case("set-cookie"))
        .map(|(_, value)| value.to_str().unwrap_or("<non-utf8>").to_string());

    let mut exchange = execute_get(
        client,
        "Cookies",
        &format!("{HTTP_LAB_BASE}/cookies"),
        HttpBodyKind::Json,
        cancellation,
    )
    .await?;

    if let (Some(response), Some(set_cookie_header)) =
        (exchange.response.as_mut(), set_cookie_header)
    {
        response
            .headers
            .push(("set-cookie".to_string(), set_cookie_header));
    }

    Ok(exchange)
}

async fn send_with_cancel(
    request: reqwest::RequestBuilder,
    method: &'static str,
    label: &str,
    url: &str,
    cancellation: CancellationToken,
) -> Result<reqwest::Response, String> {
    let started = std::time::Instant::now();
    tracing::info!(
        target: LOG,
        method,
        label,
        url,
        "HTTP Lab request send started"
    );
    tokio::select! {
        _ = cancellation.cancelled() => {
            tracing::warn!(
                target: LOG,
                method,
                label,
                url,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "HTTP Lab request send cancelled"
            );
            Err("cancelled by user".to_string())
        },
        response = request.send() => {
            match response {
                Ok(response) => {
                    tracing::info!(
                        target: LOG,
                        method,
                        label,
                        url,
                        status = response.status().as_u16(),
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "HTTP Lab request send completed"
                    );
                    Ok(response)
                }
                Err(err) => {
                    tracing::warn!(
                        target: LOG,
                        method,
                        label,
                        url,
                        error = %err,
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "HTTP Lab request send failed"
                    );
                    Err(err.to_string())
                }
            }
        },
    }
}
