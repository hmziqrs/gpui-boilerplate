use gpui::{App, Entity, SharedString};
use gpui_component::menu::AppMenuBar;

pub fn init(title: impl Into<SharedString>, cx: &mut App) -> Entity<AppMenuBar> {
    crate::menus::init(title, cx)
}
