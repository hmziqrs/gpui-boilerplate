use super::*;

#[test]
fn reconnect_policy_default_limits_retries() {
    let policy = ReconnectPolicy::default();
    assert_eq!(policy.max_retries, 5);
}

#[test]
fn reconnect_delay_increases_exponentially() {
    let policy = ReconnectPolicy::default();
    let d0 = policy.delay_for_attempt(0);
    let d1 = policy.delay_for_attempt(1);
    let d2 = policy.delay_for_attempt(2);
    assert!(d1 > d0, "delay should grow with each attempt");
    assert!(d2 > d1, "delay should grow with each attempt");
}

#[test]
fn reconnect_delay_respects_cap() {
    let policy = ReconnectPolicy {
        base_delay_ms: 1000,
        max_delay_ms: Some(5000),
        ..Default::default()
    };
    // attempt 10 would be 1000 * 2^10 = 1_024_000 ms without cap
    let capped = policy.delay_for_attempt(10);
    assert_eq!(capped.as_millis(), 5000);
}

#[test]
fn connection_state_debug_format() {
    let state = ConnectionState::Reconnecting { attempt: 3 };
    let s = format!("{:?}", state);
    assert!(s.contains("attempt"));
}

#[test]
fn stub_client_returns_error_without_feature() {
    let mut client = WebSocketClient::new("wss://localhost".to_string());
    assert_eq!(client.state, ConnectionState::Disconnected);

    // The stub methods should surface a clear error.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async { client.connect_loop().await });
    assert!(result.is_err());
}
