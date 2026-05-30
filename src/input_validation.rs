#![allow(dead_code)]

use std::path::{Path, PathBuf};

use thiserror::Error;
use url::Url;

use crate::errors::AppError;
use crate::routes::APP_URL_SCHEME;

// ---------------------------------------------------------------------------
// Deep-link URL validation
// ---------------------------------------------------------------------------

/// Validate that a deep-link URL uses the expected scheme (`gpui-starter://`)
/// and reject unexpected hosts.
pub fn validate_deep_link_url(url: &str) -> Result<Url, AppError> {
    let parsed = Url::parse(url).map_err(|err| AppError::InvalidDeepLink {
        input: url.to_string(),
        reason: err.to_string(),
    })?;

    if parsed.scheme() != APP_URL_SCHEME {
        return Err(AppError::InvalidDeepLink {
            input: url.to_string(),
            reason: format!("unsupported scheme `{}`", parsed.scheme()),
        });
    }

    let allowed_hosts = [
        "home",
        "form",
        "settings",
        "notifications",
        "diagnostics",
        "about",
    ];

    let host = parsed.host_str().unwrap_or_default();
    if !allowed_hosts.contains(&host) {
        return Err(AppError::InvalidDeepLink {
            input: url.to_string(),
            reason: format!("unexpected host `{host}`"),
        });
    }

    Ok(parsed)
}

// ---------------------------------------------------------------------------
// File-path validation
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("path traversal detected in `{input}`")]
    PathTraversal { input: String },

    #[error("path `{input}` escapes allowed directory")]
    EscapesAllowedDir { input: String },

    #[error("path does not exist: `{input}`")]
    NotFound { input: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Canonicalize a path and verify it contains no `..` traversal components
/// and stays within one of the given `allowed_dirs`.
pub fn validate_file_path(path: &str, allowed_dirs: &[PathBuf]) -> Result<PathBuf, ValidationError> {
    let raw = Path::new(path);

    // Reject any component that is a parent-directory marker.
    for component in raw.components() {
        if component == std::path::Component::ParentDir {
            return Err(ValidationError::PathTraversal {
                input: path.to_string(),
            });
        }
    }

    // Resolve to a canonical absolute path (file must exist).
    let canonical = raw
        .canonicalize()
        .map_err(|_| ValidationError::NotFound {
            input: path.to_string(),
        })?;

    // Verify the canonical path starts with at least one allowed dir.
    let permitted = allowed_dirs.iter().any(|dir| {
        let Ok(canonical_dir) = dir.canonicalize() else {
            return false;
        };
        canonical.starts_with(&canonical_dir)
    });

    if !permitted {
        return Err(ValidationError::EscapesAllowedDir {
            input: path.to_string(),
        });
    }

    Ok(canonical)
}

// ---------------------------------------------------------------------------
// String sanitization
// ---------------------------------------------------------------------------

/// Maximum length for a sanitized string.
pub const MAX_SANITIZED_LENGTH: usize = 4096;

/// Strip ASCII control characters (except newline and tab), trim whitespace,
/// and enforce a length limit.
pub fn sanitize_string(input: &str) -> String {
    let mut out: String = input
        .chars()
        .filter(|c| {
            if c.is_control() {
                // Keep newline and tab — they are legitimate whitespace.
                *c == '\n' || *c == '\t'
            } else {
                true
            }
        })
        .collect();

    out.truncate(MAX_SANITIZED_LENGTH);
    out.trim().to_string()
}

// ---------------------------------------------------------------------------
// Notification ID validation
// ---------------------------------------------------------------------------

/// Return `true` when `id` is non-empty and consists solely of ASCII
/// alphanumeric characters and hyphens.
pub fn validate_notification_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn valid_deep_link_urls() {
        assert!(validate_deep_link_url("gpui-starter://home").is_ok());
        assert!(validate_deep_link_url("gpui-starter://settings").is_ok());
        assert!(validate_deep_link_url("gpui-starter://settings/notifications").is_ok());
        assert!(validate_deep_link_url("gpui-starter://diagnostics").is_ok());
        assert!(validate_deep_link_url("gpui-starter://notifications").is_ok());
        assert!(validate_deep_link_url("gpui-starter://about").is_ok());
        assert!(validate_deep_link_url("gpui-starter://form").is_ok());
    }

    #[test]
    fn rejects_wrong_scheme() {
        let err = validate_deep_link_url("https://example.com").unwrap_err();
        assert!(
            matches!(err, AppError::InvalidDeepLink { ref reason, .. } if reason.contains("unsupported scheme"))
        );
    }

    #[test]
    fn rejects_unexpected_host() {
        let err = validate_deep_link_url("gpui-starter://evil-host").unwrap_err();
        assert!(
            matches!(err, AppError::InvalidDeepLink { ref reason, .. } if reason.contains("unexpected host"))
        );
    }

    #[test]
    fn rejects_malformed_url() {
        assert!(validate_deep_link_url("://missing-scheme").is_err());
    }

    #[test]
    fn file_path_rejects_traversal() {
        let err = validate_file_path("../../etc/passwd", &[]).unwrap_err();
        assert!(matches!(err, ValidationError::PathTraversal { .. }));
    }

    #[test]
    fn file_path_rejects_escape_from_allowed_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("inner.txt");
        fs::write(&file_path, "hello").unwrap();

        // Using a sibling temp dir as the allowed dir should fail.
        let other_tmp = tempfile::tempdir().unwrap();
        let err =
            validate_file_path(file_path.to_str().unwrap(), &[other_tmp.path().to_path_buf()])
                .unwrap_err();
        assert!(matches!(err, ValidationError::EscapesAllowedDir { .. }));
    }

    #[test]
    fn file_path_accepts_within_allowed_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("good.txt");
        fs::write(&file_path, "data").unwrap();

        let result = validate_file_path(file_path.to_str().unwrap(), &[tmp.path().to_path_buf()]);
        assert!(result.is_ok());
    }

    #[test]
    fn file_path_rejects_nonexistent() {
        let err =
            validate_file_path("/no/such/file/ever", &[PathBuf::from("/")]).unwrap_err();
        assert!(matches!(err, ValidationError::NotFound { .. }));
    }

    #[test]
    fn sanitize_strips_control_chars() {
        let dirty = "  hello\x00\x01\x02 world\t\n  ";
        assert_eq!(sanitize_string(dirty), "hello world\t\n");
    }

    #[test]
    fn sanitize_trims_and_limits_length() {
        let long: String = "a".repeat(5000);
        let result = sanitize_string(&long);
        assert_eq!(result.len(), MAX_SANITIZED_LENGTH);
        // Result is trimmed so no surrounding whitespace.
        assert!(result.starts_with('a'));
    }

    #[test]
    fn sanitize_keeps_newline_and_tab() {
        assert_eq!(sanitize_string("line1\nline2\ttab"), "line1\nline2\ttab");
    }

    #[test]
    fn valid_notification_ids() {
        assert!(validate_notification_id("abc123"));
        assert!(validate_notification_id("a-b-c"));
        assert!(validate_notification_id("ABC-123-xyz"));
        assert!(validate_notification_id("550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn invalid_notification_ids() {
        assert!(!validate_notification_id(""));
        assert!(!validate_notification_id("has space"));
        assert!(!validate_notification_id("under_score"));
        assert!(!validate_notification_id("special!char"));
        assert!(!validate_notification_id("dot.name"));
    }
}
