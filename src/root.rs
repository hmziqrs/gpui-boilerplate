use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Root, Sizable as _,
    resizable::{h_resizable, resizable_panel},
    sidebar::{Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
    v_flex,
};

use crate::sidebar::Page;
use crate::title_bar::AppTitleBar;
use crate::views::{
    AboutPage, DiagnosticsPage, FormPage, HomePage, NotificationsPage, SettingsPage,
};
use crate::{
    app::ToggleSearch,
    events::{self, AppEventKind},
    routes::AppRoute,
};

pub struct AppRoot {
    focus_handle: FocusHandle,
    title_bar: Entity<AppTitleBar>,
    active_route: AppRoute,
    collapsed: bool,
    home_page: Entity<HomePage>,
    form_page: Entity<FormPage>,
    settings_page: Entity<SettingsPage>,
    notifications_page: Entity<NotificationsPage>,
    diagnostics_page: Entity<DiagnosticsPage>,
    about_page: Entity<AboutPage>,
}

impl AppRoot {
    pub fn new(
        title: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_bar = cx.new(|cx| AppTitleBar::new(title, window, cx));
        let home_page = cx.new(|_| HomePage::new());
        let form_page = cx.new(|cx| FormPage::new(window, cx));
        let settings_page = cx.new(|cx| SettingsPage::new(window, cx));
        let notifications_page = cx.new(|cx| NotificationsPage::new(window, cx));
        let diagnostics_page = cx.new(|cx| DiagnosticsPage::new(window, cx));
        let about_page = cx.new(|_| AboutPage::new());

        // React to app-wide events coming from launcher/deep links.
        cx.observe_global::<events::AppEventQueue>(|this, cx| {
            for event in events::drain(cx) {
                match event.kind {
                    AppEventKind::Navigate(route) => this.set_route(route, cx),
                    AppEventKind::DeepLinkReceived(link) => match AppRoute::parse_deep_link(&link) {
                        Ok(route) => this.set_route(route, cx),
                        Err(err) => events::emit_error(err, cx),
                    },
                    AppEventKind::AppError(error) => {
                        tracing::warn!(target: "gpui_starter::root", error, "app error event received");
                    }
                    AppEventKind::BackgroundTaskChanged(_) | AppEventKind::DiagnosticsChanged => {}
                }
            }
        })
        .detach();
        cx.observe_global::<crate::tasks::TaskRegistry>(|_, cx| {
            cx.notify();
        })
        .detach();
        cx.observe_global::<crate::notifications::NativeNotificationState>(|_, cx| {
            cx.notify();
        })
        .detach();
        cx.observe_global::<crate::notifications::inbox::NotificationInboxState>(|_, cx| {
            cx.notify();
        })
        .detach();
        cx.observe_global::<crate::connectivity::ConnectivitySnapshot>(|_, cx| {
            cx.notify();
        })
        .detach();
        cx.observe_global::<crate::session::SessionSnapshot>(|_, cx| {
            cx.notify();
        })
        .detach();

        let config = crate::app_state::config(cx);
        Self {
            focus_handle: cx.focus_handle(),
            title_bar,
            active_route: config.active_route,
            collapsed: config.sidebar_collapsed,
            home_page,
            form_page,
            settings_page,
            notifications_page,
            diagnostics_page,
            about_page,
        }
    }

    fn active_page_view(&self) -> AnyView {
        match self.active_route.page_for_render() {
            Page::Home => self.home_page.clone().into(),
            Page::Form => self.form_page.clone().into(),
            Page::Settings => self.settings_page.clone().into(),
            Page::Notifications => self.notifications_page.clone().into(),
            Page::Diagnostics => self.diagnostics_page.clone().into(),
            Page::About => self.about_page.clone().into(),
        }
    }

    fn set_route(&mut self, route: AppRoute, cx: &mut Context<Self>) {
        if self.active_route == route {
            return;
        }
        let route_url = route.to_url();
        tracing::info!(target: "gpui_starter::root", route = ?route, route_url, "navigating");
        self.active_route = route.clone();
        crate::app_state::update_config(cx, |config| {
            config.active_route = route;
        });
        cx.notify();
    }
}

impl Focusable for AppRoot {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AppRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sheet_layer = Root::render_sheet_layer(window, cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);
        let page_title = self.active_route.title();
        let active_page = self.active_route.page_for_render();

        let sidebar = Sidebar::new("app-sidebar")
            .w(relative(1.))
            .border_0()
            .collapsed(self.collapsed)
            .header(
                v_flex().w_full().gap_4().child(
                    SidebarHeader::new().w_full().child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(cx.theme().radius_lg)
                            .bg(cx.theme().primary)
                            .text_color(cx.theme().primary_foreground)
                            .size_8()
                            .flex_shrink_0()
                            .child(Icon::new(IconName::Star)),
                    ),
                ),
            )
            .child(
                SidebarGroup::new("Navigation").child(SidebarMenu::new().children(
                    Page::all().iter().map(|page| {
                        SidebarMenuItem::new(page.title())
                            .icon(Icon::new(page.icon()).small())
                            .active(active_page == *page)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.set_route(AppRoute::page(*page), cx);
                            }))
                    }),
                )),
            );

        v_flex()
            .size_full()
            .child(self.title_bar.clone())
            .child(
                div()
                    .track_focus(&self.focus_handle)
                    .on_action(cx.listener(|_, _: &ToggleSearch, _, cx| {
                        crate::launcher::open_launcher(cx);
                    }))
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        h_resizable("app-layout")
                            .child(
                                resizable_panel()
                                    .size(px(255.))
                                    .size_range(px(60.)..px(320.))
                                    .child(sidebar),
                            )
                            .child(
                                resizable_panel().child(
                                    v_flex()
                                        .flex_1()
                                        .h_full()
                                        .overflow_x_hidden()
                                        .child(
                                            div()
                                                .id("header")
                                                .p_4()
                                                .border_b_1()
                                                .border_color(cx.theme().border)
                                                .child(
                                                    div()
                                                        .text_xl()
                                                        .font_weight(FontWeight::BOLD)
                                                        .child(page_title),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .id("page")
                                                .flex_1()
                                                .overflow_y_scroll()
                                                .child(self.active_page_view()),
                                        ),
                                ),
                            ),
                    ),
            )
            .child(crate::status_bar::render(&self.active_route, cx))
            .children(sheet_layer)
            .children(dialog_layer)
            .children(notification_layer)
    }
}
