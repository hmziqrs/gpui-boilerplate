use super::*;
use crate::{
    ids::TaskId,
    services::query::{QueryError, QueryStatus},
};

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

fn seed_response(
    state: &mut HttpLabState,
    action: HttpLabAction,
    response: HttpExchange,
    now_ms: u128,
) {
    let request = begin_action(state, action, now_ms.saturating_sub(1)).expect("seed request");
    apply_result_to_state(state, action, request, Ok(vec![(action, response)]), now_ms);
}

fn error_message(resource: &QueryResource<HttpExchange>) -> Option<&str> {
    resource.error().map(QueryError::message)
}

#[test]
fn starts_each_action_with_its_own_resource() {
    let state = HttpLabState::default();

    for action in HttpLabAction::all() {
        let resource = state.resource(*action);
        assert_eq!(resource.key(), &action.query_key());
        assert_eq!(resource.status(), QueryStatus::Idle);
    }
}

#[test]
fn ttl_cache_can_short_circuit_request() {
    let mut state = HttpLabState::default();
    let now_ms = 10_000;
    seed_response(
        &mut state,
        HttpLabAction::GetText,
        exchange("GET text", 200, None),
        now_ms - 100,
    );

    let request = begin_action(&mut state, HttpLabAction::GetText, now_ms);

    assert!(request.is_none());
    let resource = state.resource(HttpLabAction::GetText);
    assert_eq!(resource.status(), QueryStatus::Success);
    assert_eq!(resource.cache_hits(), 1);
}

#[test]
fn stale_while_revalidate_keeps_data_and_starts_request() {
    let mut state = HttpLabState::default();
    let now_ms = 100_000;
    seed_response(
        &mut state,
        HttpLabAction::GetJson,
        exchange("GET JSON", 200, None),
        now_ms - 1,
    );

    let request = begin_action(&mut state, HttpLabAction::GetJson, now_ms);

    assert!(request.is_some());
    let resource = state.resource(HttpLabAction::GetJson);
    assert_eq!(resource.status(), QueryStatus::LoadingWithData);
    assert!(resource.data().is_some());
}

#[test]
fn latest_wins_cancels_previous_request_for_same_action() {
    let mut state = HttpLabState::default();
    let first = begin_action(&mut state, HttpLabAction::GetXml, 1).expect("first request");
    let second = begin_action(&mut state, HttpLabAction::GetXml, 2).expect("second request");

    assert_ne!(first, second);
    assert_eq!(state.active_count(), 1);
    assert_eq!(
        state.resource(HttpLabAction::GetXml).active_request_id(),
        Some(second)
    );
    assert_eq!(state.resource(HttpLabAction::GetXml).cancelled_count(), 1);
}

#[test]
fn ignore_while_loading_policy_does_not_start_duplicate_request() {
    let mut state = HttpLabState::default();
    let first = begin_action(&mut state, HttpLabAction::PostMultipart, 1).expect("first request");
    let duplicate = begin_action(&mut state, HttpLabAction::PostMultipart, 2);

    assert!(duplicate.is_none());
    assert_eq!(
        state
            .resource(HttpLabAction::PostMultipart)
            .active_request_id(),
        Some(first)
    );
    assert_eq!(state.active_count(), 1);
}

#[test]
fn stale_result_is_ignored_after_cancellation() {
    let mut state = HttpLabState::default();
    let request = begin_action(&mut state, HttpLabAction::GetJson, 1).expect("request");

    cancel_action_in_state(&mut state, HttpLabAction::GetJson, "test cancel");
    apply_result_to_state(
        &mut state,
        HttpLabAction::GetJson,
        request,
        Ok(vec![(
            HttpLabAction::GetJson,
            exchange("GET JSON", 200, None),
        )]),
        2,
    );

    let resource = state.resource(HttpLabAction::GetJson);
    assert_eq!(resource.status(), QueryStatus::Cancelled);
    assert!(resource.data().is_none());
    assert_eq!(resource.ignored_results(), 1);
}

#[test]
fn cancelled_request_keeps_task_tracking_until_result_arrives() {
    let mut state = HttpLabState::default();
    let request = begin_action(&mut state, HttpLabAction::GetJson, 1).expect("request");
    let task_id = TaskId::new();
    state.inflight_tasks.insert(request, task_id);
    let cancellation = register_request_flag(request);

    cancel_action_in_state(&mut state, HttpLabAction::GetJson, "test cancel");
    assert_eq!(state.inflight_tasks.get(&request), Some(&task_id));
    assert!(cancellation.load(std::sync::atomic::Ordering::SeqCst));

    let update = apply_result_to_state(
        &mut state,
        HttpLabAction::GetJson,
        request,
        Ok(vec![(
            HttpLabAction::GetJson,
            exchange("GET JSON", 200, None),
        )]),
        2,
    );

    assert_eq!(
        update,
        Some(HttpTaskUpdate::cancelled(
            Some(task_id),
            format!("ignored stale request {}", request.label()),
        ))
    );
    assert!(!state.inflight_tasks.contains_key(&request));
    assert!(cancellation_flags().lock().unwrap().get(&request).is_none());
}

#[test]
fn successful_result_completes_tracked_task() {
    let mut state = HttpLabState::default();
    let request = begin_action(&mut state, HttpLabAction::GetJson, 1).expect("request");
    let task_id = TaskId::new();
    state.inflight_tasks.insert(request, task_id);
    register_request_flag(request);

    let update = apply_result_to_state(
        &mut state,
        HttpLabAction::GetJson,
        request,
        Ok(vec![(
            HttpLabAction::GetJson,
            exchange("GET JSON", 200, None),
        )]),
        2,
    );

    assert_eq!(update, Some(HttpTaskUpdate::succeeded(Some(task_id))));
    assert!(!state.inflight_tasks.contains_key(&request));
    assert!(cancellation_flags().lock().unwrap().get(&request).is_none());
}

#[test]
fn successful_exchange_updates_only_target_resource() {
    let mut state = HttpLabState::default();
    let request = begin_action(&mut state, HttpLabAction::GetJson, 1).expect("request");

    apply_result_to_state(
        &mut state,
        HttpLabAction::GetJson,
        request,
        Ok(vec![(
            HttpLabAction::GetJson,
            exchange("GET JSON", 200, None),
        )]),
        2,
    );

    assert_eq!(
        state.resource(HttpLabAction::GetJson).status(),
        QueryStatus::Success
    );
    assert!(state.resource(HttpLabAction::GetJson).data().is_some());
    assert_eq!(
        state.resource(HttpLabAction::GetXml).status(),
        QueryStatus::Idle
    );
    assert_eq!(state.history.len(), 1);
}

#[test]
fn failed_exchange_preserves_previous_data() {
    let mut state = HttpLabState::default();
    let previous = exchange("Failure", 500, Some("HTTP 500"));
    seed_response(&mut state, HttpLabAction::Failure, previous, 0);

    let request = begin_action(&mut state, HttpLabAction::Failure, 1).expect("request");
    apply_result_to_state(
        &mut state,
        HttpLabAction::Failure,
        request,
        Ok(vec![(
            HttpLabAction::Failure,
            exchange("Failure", 503, Some("HTTP 503")),
        )]),
        2,
    );

    let resource = state.resource(HttpLabAction::Failure);
    assert_eq!(resource.status(), QueryStatus::Failure);
    assert!(resource.data().is_some());
    assert_eq!(error_message(resource), Some("HTTP 503"));
}

#[test]
fn full_flow_populates_individual_resources_and_flow_resource() {
    let mut state = HttpLabState::default();
    let request = begin_action(&mut state, HttpLabAction::FullFlow, 1).expect("flow request");
    let exchanges = vec![
        (HttpLabAction::GetJson, exchange("GET JSON", 200, None)),
        (
            HttpLabAction::Failure,
            exchange("Failure", 503, Some("HTTP 503")),
        ),
        (HttpLabAction::PostJson, exchange("POST JSON", 200, None)),
    ];

    apply_result_to_state(
        &mut state,
        HttpLabAction::FullFlow,
        request,
        Ok(exchanges),
        2,
    );

    assert_eq!(
        state.resource(HttpLabAction::FullFlow).status(),
        QueryStatus::Success
    );
    assert_eq!(
        state.resource(HttpLabAction::GetJson).status(),
        QueryStatus::Success
    );
    assert_eq!(
        state.resource(HttpLabAction::Failure).status(),
        QueryStatus::Failure
    );
    assert_eq!(
        state.resource(HttpLabAction::PostJson).status(),
        QueryStatus::Success
    );
    assert_eq!(state.history.len(), 3);
}

#[test]
fn individual_request_cancels_pending_full_flow_before_it_can_apply_results() {
    let mut state = HttpLabState::default();
    let flow_request = begin_action(&mut state, HttpLabAction::FullFlow, 1).expect("flow request");
    let individual_request =
        begin_action(&mut state, HttpLabAction::GetJson, 2).expect("individual request");

    apply_result_to_state(
        &mut state,
        HttpLabAction::FullFlow,
        flow_request,
        Ok(vec![(
            HttpLabAction::GetJson,
            exchange("GET JSON from stale flow", 200, None),
        )]),
        3,
    );

    assert_eq!(
        state.resource(HttpLabAction::FullFlow).status(),
        QueryStatus::Cancelled
    );
    assert_eq!(state.resource(HttpLabAction::FullFlow).ignored_results(), 1);
    assert_eq!(
        state.resource(HttpLabAction::GetJson).active_request_id(),
        Some(individual_request)
    );
    assert!(state.resource(HttpLabAction::GetJson).data().is_none());
}

#[test]
fn reset_makes_in_flight_request_results_silent_stale_results() {
    let mut state = HttpLabState::default();
    let request = begin_action(&mut state, HttpLabAction::GetJson, 1).expect("request");
    let task_id = TaskId::new();
    state.inflight_tasks.insert(request, task_id);

    let reset_requests = state.reset_for_user();
    let transition_log = state.transition_log.clone();
    apply_result_to_state(
        &mut state,
        HttpLabAction::GetJson,
        request,
        Ok(vec![(
            HttpLabAction::GetJson,
            exchange("GET JSON from stale scope", 200, None),
        )]),
        2,
    );

    let resource = state.resource(HttpLabAction::GetJson);
    assert_eq!(resource.status(), QueryStatus::Idle);
    assert_eq!(resource.ignored_results(), 0);
    assert_eq!(state.transition_log, transition_log);
    assert_eq!(
        reset_requests,
        ResetRequests {
            request_ids: vec![request],
            task_ids: vec![task_id],
        }
    );
}

#[test]
fn reset_prevents_old_request_id_from_colliding_with_new_request() {
    let mut state = HttpLabState::default();
    let old_request = begin_action(&mut state, HttpLabAction::GetJson, 1).expect("old request");

    let _reset_requests = state.reset_for_user();
    let new_request = begin_action(&mut state, HttpLabAction::GetJson, 2).expect("new request");
    apply_result_to_state(
        &mut state,
        HttpLabAction::GetJson,
        old_request,
        Ok(vec![(
            HttpLabAction::GetJson,
            exchange("GET JSON from stale scope", 200, None),
        )]),
        3,
    );

    let resource = state.resource(HttpLabAction::GetJson);
    assert_eq!(old_request.value(), new_request.value());
    assert_ne!(old_request, new_request);
    assert_eq!(resource.active_request_id(), Some(new_request));
    assert!(resource.data().is_none());
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
    let request = begin_action(&mut state, HttpLabAction::Cookies, 1).expect("request");

    apply_result_to_state(
        &mut state,
        HttpLabAction::Cookies,
        request,
        Ok(vec![(HttpLabAction::Cookies, cookies)]),
        2,
    );

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
