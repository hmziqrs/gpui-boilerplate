mod app;
mod app_menu;
mod app_state;
mod capabilities;
mod commands;
mod config_migrations;
mod connectivity;
mod desktop_actions;
mod errors;
mod events;
mod first_run;
mod i18n;
mod ids;
mod launcher;
mod lifecycle;
mod logging;
mod menus;
mod notifications;
mod paths;
mod root;
mod routes;
mod secure_storage;
mod session;
mod shortcuts;
mod sidebar;
mod single_instance;
mod status_bar;
mod storage;
mod tasks;
mod telemetry;
#[cfg(test)]
mod testing;
mod time;
mod title_bar;
mod undo_stack;
mod views;

#[cfg(target_os = "macos")]
mod tray;

use gpui_component_assets::Assets;

fn main() {
    let preflight = single_instance::preflight();
    if !preflight.should_start {
        return;
    }
    let startup_runtime = preflight.runtime;
    let startup_deep_link = preflight.initial_deep_link;

    let app = gpui_platform::application().with_assets(Assets);
    app.run(move |cx| {
        app::init(cx);
        if let Some(runtime) = startup_runtime {
            single_instance::install(runtime, cx);
        }
        if let Some(link) = startup_deep_link {
            events::emit(events::AppEventKind::DeepLinkReceived(link), cx);
        }

        #[cfg(target_os = "macos")]
        tray::setup(cx);

        cx.activate(true);
        app::create_new_window("My App", cx);
    });
}
