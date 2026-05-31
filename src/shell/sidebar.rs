use gpui_component::IconName;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Page {
    Home,
    Form,
    Settings,
    Notifications,
    Diagnostics,
    About,
}

impl Page {
    pub fn title(&self) -> &'static str {
        match self {
            Page::Home => "Home",
            Page::Form => "Form",
            Page::Settings => "Settings",
            Page::Notifications => "Notifications",
            Page::Diagnostics => "Diagnostics",
            Page::About => "About",
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            Page::Home => IconName::Inbox,
            Page::Form => IconName::File,
            Page::Settings => IconName::Settings2,
            Page::Notifications => IconName::Bell,
            Page::Diagnostics => IconName::Info,
            Page::About => IconName::Info,
        }
    }

    pub fn all() -> &'static [Page] {
        &[
            Page::Home,
            Page::Form,
            Page::Settings,
            Page::Notifications,
            Page::Diagnostics,
            Page::About,
        ]
    }
}
