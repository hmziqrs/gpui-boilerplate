use gpui_component_assets::Assets;
use gpui_starter::{app, events, single_instance};

fn main() {
    let preflight = single_instance::preflight();
    if !preflight.should_start {
        return;
    }
    let startup_runtime = preflight.runtime;
    let startup_deep_link = preflight.initial_deep_link;

    let app_runtime = gpui_platform::application().with_assets(Assets);
    app_runtime.run(move |cx| {
        app::init(cx);
        if let Some(runtime) = startup_runtime {
            single_instance::install(runtime, cx);
        }
        if let Some(link) = startup_deep_link {
            events::emit(events::AppEventKind::DeepLinkReceived(link), cx);
        }

        #[cfg(target_os = "macos")]
        gpui_starter::tray::setup(cx);

        cx.activate(true);
        app::create_new_window("My App", cx);
    });
}
