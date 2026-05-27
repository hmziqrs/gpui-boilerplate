use serde::{Deserialize, Serialize};
use url::Url;

use crate::{errors::AppError, sidebar::Page};

pub const APP_URL_SCHEME: &str = "gpui-starter";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppRoute {
    Page(Page),
    SettingsNotifications,
}

impl AppRoute {
    pub fn page(page: Page) -> Self {
        Self::Page(page)
    }

    pub fn page_for_render(&self) -> Page {
        match self {
            Self::Page(page) => *page,
            Self::SettingsNotifications => Page::Settings,
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::Page(page) => page.title(),
            Self::SettingsNotifications => "Settings",
        }
    }

    pub fn to_url(&self) -> String {
        match self {
            Self::Page(Page::Home) => "gpui-starter://home".to_string(),
            Self::Page(Page::Form) => "gpui-starter://form".to_string(),
            Self::Page(Page::Settings) => "gpui-starter://settings".to_string(),
            Self::Page(Page::Notifications) => "gpui-starter://notifications".to_string(),
            Self::Page(Page::Diagnostics) => "gpui-starter://diagnostics".to_string(),
            Self::Page(Page::About) => "gpui-starter://about".to_string(),
            Self::SettingsNotifications => "gpui-starter://settings/notifications".to_string(),
        }
    }

    pub fn parse_deep_link(input: &str) -> Result<Self, AppError> {
        let url = Url::parse(input).map_err(|err| AppError::InvalidDeepLink {
            input: input.to_string(),
            reason: err.to_string(),
        })?;

        if url.scheme() != APP_URL_SCHEME {
            return Err(AppError::InvalidDeepLink {
                input: input.to_string(),
                reason: format!("unsupported scheme `{}`", url.scheme()),
            });
        }

        let host = url.host_str().unwrap_or_default();
        let segments = url
            .path_segments()
            .map(|segments| {
                segments
                    .filter(|segment| !segment.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        match (host, segments.as_slice()) {
            ("home", []) => Ok(Self::Page(Page::Home)),
            ("form", []) => Ok(Self::Page(Page::Form)),
            ("settings", []) => Ok(Self::Page(Page::Settings)),
            ("settings", ["notifications"]) => Ok(Self::SettingsNotifications),
            ("notifications", []) => Ok(Self::Page(Page::Notifications)),
            ("diagnostics", []) => Ok(Self::Page(Page::Diagnostics)),
            ("about", []) => Ok(Self::Page(Page::About)),
            _ => Err(AppError::InvalidDeepLink {
                input: input.to_string(),
                reason: "unknown route".to_string(),
            }),
        }
    }
}

impl Default for AppRoute {
    fn default() -> Self {
        Self::Page(Page::Home)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_deep_links() {
        let home = AppRoute::parse_deep_link("gpui-starter://home").unwrap();
        assert_eq!(home, AppRoute::Page(Page::Home));
        assert_eq!(home.to_url(), "gpui-starter://home");
        assert_eq!(
            AppRoute::parse_deep_link("gpui-starter://settings/notifications").unwrap(),
            AppRoute::SettingsNotifications
        );
        assert_eq!(
            AppRoute::parse_deep_link("gpui-starter://diagnostics").unwrap(),
            AppRoute::Page(Page::Diagnostics)
        );
        assert_eq!(
            AppRoute::parse_deep_link("gpui-starter://notifications").unwrap(),
            AppRoute::Page(Page::Notifications)
        );
    }

    #[test]
    fn rejects_unknown_deep_links() {
        assert!(AppRoute::parse_deep_link("https://example.com").is_err());
        assert!(AppRoute::parse_deep_link("gpui-starter://missing").is_err());
    }
}
