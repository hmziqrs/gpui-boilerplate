use gpui::{App, InteractiveElement as _, ParentElement as _, Styled as _, div};
use gpui_component::ActiveTheme as _;

use crate::{connectivity, notifications, routes::AppRoute, session, tasks};

pub fn render(route: &AppRoute, cx: &App) -> impl gpui::IntoElement {
    let tasks_active = tasks::active_count(cx);
    let notifications_state = notifications::snapshot(cx);
    let connectivity_state = connectivity::snapshot(cx);
    let session_state = session::snapshot(cx);
    let unread = crate::notifications::inbox::snapshot(cx)
        .iter()
        .filter(|item| !item.read)
        .count();
    let degraded = notifications_state
        .degraded_reason
        .as_deref()
        .unwrap_or("No");
    let session_label = match &session_state.state {
        session::SessionState::SignedOut => "SignedOut".to_string(),
        session::SessionState::SigningIn => "SigningIn".to_string(),
        session::SessionState::SignedIn { account_label } => format!("SignedIn({account_label})"),
        session::SessionState::Error(error) => format!("Error({error})"),
    };

    div()
        .id("status-bar")
        .w_full()
        .px_3()
        .py_2()
        .border_t_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary.opacity(0.35))
        .text_xs()
        .child(div().flex().gap_4().items_center().children([
            div().child(format!("Route: {}", route.title())),
            div().child(format!("Tasks: {tasks_active}")),
            div().child(format!("Unread: {unread}")),
            div().child(format!("Connectivity: {:?}", connectivity_state.state)),
            div().child(format!("Session: {session_label}")),
            div().child(format!(
                "Notifications: {}",
                notifications_state.active_backend
            )),
            div().child(format!("Degraded: {degraded}")),
        ]))
}
