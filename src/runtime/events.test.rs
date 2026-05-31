use super::*;

#[test]
fn app_event_new_sets_id_and_timestamp() {
    let event = AppEvent::new(AppEventKind::DiagnosticsChanged);
    assert!(!event.id.to_string().is_empty());
    assert!(!event.emitted_at.to_rfc3339().is_empty());
}

#[test]
fn queue_preserves_event_order() {
    let first = AppEvent::new(AppEventKind::DiagnosticsChanged);
    let second = AppEvent::new(AppEventKind::AppError {
        message: "oops".to_string(),
        severity: crate::errors::AppErrorSeverity::Error,
    });
    let third = AppEvent::new(AppEventKind::DeepLinkReceived(
        "gpui-starter://settings".to_string(),
    ));
    let queue = AppEventQueue(vec![first.clone(), second.clone(), third.clone()]);
    assert!(matches!(queue.0[0].kind, AppEventKind::DiagnosticsChanged));
    assert!(matches!(queue.0[1].kind, AppEventKind::AppError { .. }));
    assert!(matches!(queue.0[2].kind, AppEventKind::DeepLinkReceived(_)));
}
