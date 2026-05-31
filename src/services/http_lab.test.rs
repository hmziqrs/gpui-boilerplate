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
