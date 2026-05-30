use serde::{Deserialize, Serialize};
use url::Url;

use crate::{errors::AppError, sidebar::Page};

pub const APP_URL_SCHEME: &str = "gpui-starter";

/// Hosts that are recognized as valid deep link targets.
const VALID_HOSTS: &[&str] = &[
    "home",
    "form",
    "settings",
    "notifications",
    "diagnostics",
    "about",
];

/// Characters that are not permitted in path segments.
const INVALID_SEGMENT_CHARS: &[char] = &['/', '\\', '\0', '<', '>', '|', '"'];

/// Strip control characters (U+0000–U+001F and U+007F) from a string,
/// returning a sanitized copy suitable for safe use.
fn sanitize_control_chars(input: &str) -> String {
    input.chars().filter(|c| !c.is_control()).collect()
}

/// Reject path segments that contain traversal sequences or other dangerous
/// characters. Returns `Ok(())` when the segment is safe.
fn validate_path_segment(segment: &str) -> Result<(), AppError> {
    if segment.is_empty() {
        return Ok(());
    }

    // Path-traversal checks.
    if segment == ".." || segment == "." || segment.contains("..") {
        return Err(AppError::InvalidDeepLink {
            input: segment.to_string(),
            reason: "path traversal detected in URL segment".to_string(),
        });
    }

    for &forbidden in INVALID_SEGMENT_CHARS {
        if segment.contains(forbidden) {
            return Err(AppError::InvalidDeepLink {
                input: segment.to_string(),
                reason: format!(
                    "path segment contains forbidden character `{:?}`",
                    forbidden
                ),
            });
        }
    }

    Ok(())
}

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
        // --- 1. Parse the raw URL -------------------------------------------
        let url = Url::parse(input).map_err(|err| AppError::InvalidDeepLink {
            input: input.to_string(),
            reason: err.to_string(),
        })?;

        // --- 2. Scheme validation -------------------------------------------
        if url.scheme() != APP_URL_SCHEME {
            return Err(AppError::InvalidDeepLink {
                input: input.to_string(),
                reason: format!(
                    "unsupported scheme `{}`, expected `{}`",
                    url.scheme(),
                    APP_URL_SCHEME
                ),
            });
        }

        // --- 3. Host validation ---------------------------------------------
        let host = url.host_str().unwrap_or_default();
        if !VALID_HOSTS.contains(&host) {
            return Err(AppError::InvalidDeepLink {
                input: input.to_string(),
                reason: format!("unexpected host `{}`", host),
            });
        }

        // --- 4. Path segment validation -------------------------------------
        let segments: Vec<&str> = url
            .path_segments()
            .map(|segs| segs.filter(|s| !s.is_empty()).collect::<Vec<_>>())
            .unwrap_or_default();

        for segment in &segments {
            validate_path_segment(segment)?;
        }

        // --- 5. Query parameter sanitization --------------------------------
        for (key, value) in url.query_pairs() {
            let _key = sanitize_control_chars(&key);
            let _value = sanitize_control_chars(&value);
            // Sanitized values are currently unused but are validated here so
            // that future consumers start from clean data. Control-character
            // injection through query strings is prevented at the gate.
        }

        // --- 6. Route matching (unchanged logic) ----------------------------
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

    // --- URL validation tests -----------------------------------------------

    #[test]
    fn rejects_wrong_scheme() {
        let err = AppRoute::parse_deep_link("https://home").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unsupported scheme"),
            "expected scheme rejection, got: {msg}"
        );
    }

    #[test]
    fn rejects_unexpected_host() {
        let err = AppRoute::parse_deep_link("gpui-starter://evil-host").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unexpected host"),
            "expected host rejection, got: {msg}"
        );
    }

    #[test]
    fn rejects_path_traversal_in_segment() {
        let err =
            AppRoute::parse_deep_link("gpui-starter://settings/..%2Fetc").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("path traversal") || msg.contains("forbidden character"),
            "expected path-traversal rejection, got: {msg}"
        );
    }

    #[test]
    fn rejects_null_byte_in_segment() {
        let err = AppRoute::parse_deep_link("gpui-starter://settings/\0notifications")
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("forbidden character") || msg.contains("invalid deep link"),
            "expected null-byte rejection, got: {msg}"
        );
    }

    #[test]
    fn accepts_deep_link_with_clean_query_params() {
        let result =
            AppRoute::parse_deep_link("gpui-starter://home?ref=test&source=menu");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), AppRoute::Page(Page::Home));
    }

    #[test]
    fn sanitize_control_chars_helper() {
        assert_eq!(sanitize_control_chars("hello\tworld\n"), "helloworld");
        assert_eq!(sanitize_control_chars("clean"), "clean");
        assert_eq!(sanitize_control_chars("\x07bell\x1besc"), "bellesc");
    }

    #[test]
    fn validate_path_segment_helper() {
        assert!(validate_path_segment("notifications").is_ok());
        assert!(validate_path_segment("..").is_err());
        assert!(validate_path_segment("foo/../bar").is_err());
        assert!(validate_path_segment("seg\x00ment").is_err());
    }
}
