use std::{collections::BTreeMap, time::Instant};

use reqwest::header::{CONTENT_TYPE, HeaderMap};
use tokio_util::sync::CancellationToken;

use crate::services::http_lab::types::{
    HttpBodyKind, HttpCookieSnapshot, HttpExchange, HttpRequestSnapshot, HttpResponseSnapshot,
};

const LOG: &str = "gpui_starter::http_lab::response";

pub(super) async fn response_to_exchange(
    label: &str,
    request: HttpRequestSnapshot,
    response: reqwest::Response,
    started: Instant,
    expected_kind: HttpBodyKind,
    cancellation: CancellationToken,
) -> Result<HttpExchange, String> {
    let response_started = Instant::now();
    let status = response.status();
    let final_url = response.url().to_string();
    let headers = headers_to_vec(response.headers());
    let body_kind = detect_body_kind(response.headers(), expected_kind);
    tracing::info!(
        target: LOG,
        label,
        status = status.as_u16(),
        final_url,
        header_count = headers.len(),
        body_kind = body_kind.label(),
        "HTTP Lab response headers captured"
    );
    let body = response_preview(response, cancellation).await?;
    tracing::info!(
        target: LOG,
        label,
        status = status.as_u16(),
        body_bytes = body.len(),
        elapsed_ms = response_started.elapsed().as_millis() as u64,
        "HTTP Lab response preview captured"
    );
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

async fn response_preview(
    mut response: reqwest::Response,
    cancellation: CancellationToken,
) -> Result<String, String> {
    const BODY_PREVIEW_BYTES: usize = 8_000;
    let started = Instant::now();
    let mut bytes = Vec::new();
    let mut chunk_count = 0_u64;
    while bytes.len() < BODY_PREVIEW_BYTES {
        let chunk = tokio::select! {
            _ = cancellation.cancelled() => {
                tracing::warn!(
                    target: LOG,
                    chunk_count,
                    bytes = bytes.len(),
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "HTTP Lab response preview cancelled"
                );
                return Err("cancelled by user".to_string());
            },
            chunk = response.chunk() => chunk.map_err(|err| err.to_string())?,
        };
        let Some(chunk) = chunk else {
            break;
        };
        chunk_count += 1;
        tracing::debug!(
            target: LOG,
            chunk_count,
            chunk_bytes = chunk.len(),
            preview_bytes = bytes.len(),
            "HTTP Lab response preview chunk received"
        );
        let remaining = BODY_PREVIEW_BYTES - bytes.len();
        if chunk.len() <= remaining {
            bytes.extend_from_slice(&chunk);
        } else {
            bytes.extend_from_slice(&chunk[..remaining]);
            break;
        }
    }
    tracing::info!(
        target: LOG,
        chunk_count,
        bytes = bytes.len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "HTTP Lab response preview finished"
    );
    Ok(String::from_utf8_lossy(&bytes).into_owned())
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

pub(super) fn content_type_for(kind: HttpBodyKind) -> &'static str {
    match kind {
        HttpBodyKind::Text => "text/plain, */*",
        HttpBodyKind::Json => "application/json",
        HttpBodyKind::Xml => "application/xml, text/xml",
    }
}

pub(super) fn parse_json(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .map(|value| truncate(&value, 8_000))
}

pub(super) fn parse_xml_preview(body: &str, kind: HttpBodyKind) -> Option<String> {
    (kind == HttpBodyKind::Xml).then(|| truncate(body, 4_000))
}

pub(super) fn truncate(value: &str, max_len: usize) -> String {
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

pub(super) fn cookie_snapshot_from_exchange(exchange: &HttpExchange) -> Option<HttpCookieSnapshot> {
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
