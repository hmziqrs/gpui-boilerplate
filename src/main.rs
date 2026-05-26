mod app;
mod i18n;
mod launcher;
mod menus;
mod root;
mod sidebar;
mod title_bar;
mod views;

use gpui_component_assets::Assets;

fn main() {
    let app = gpui_platform::application().with_assets(Assets);
    app.run(move |cx| {
        app::init(cx);
        cx.activate(true);

        app::create_new_window("My App", cx);
    });
}
