use std::ops::Deref;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A structured, hierarchical cache key for query resources.
///
/// Inspired by TanStack Query's array keys (`["todos", id]`), a `QueryKey`
/// is an ordered sequence of string segments. Two operations use this structure:
///
/// - **Exact match**: keys are equal when all segments match
/// - **Prefix match**: [`starts_with`](QueryKey::starts_with) checks if a key
///   begins with a given prefix — used by [`invalidate_queries`](crate::client::QueryClient::invalidate_queries)
///
/// # Cheap cloning
///
/// Internally uses `Arc<[Arc<str>]>`, so cloning is a single atomic ref-count
/// increment regardless of key length.
///
/// # Serde
///
/// Serializes as a JSON array of strings for interop.
///
/// # Examples
///
/// ```
/// use gpui_query::QueryKey;
///
/// let key = QueryKey::from(["users", "42", "posts"]);
/// assert!(key.starts_with(&QueryKey::from(["users"])));
/// assert!(key.starts_with(&QueryKey::from(["users", "42"])));
/// assert!(!key.starts_with(&QueryKey::from(["posts"])));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryKey(Arc<[Arc<str>]>);

impl QueryKey {
    /// Create a key from an iterator of string-like parts.
    pub fn new(parts: impl IntoIterator<Item: AsRef<str>>) -> Self {
        let segments: Vec<Arc<str>> = parts
            .into_iter()
            .map(|s| Arc::from(s.as_ref()))
            .collect();
        Self(segments.into())
    }

    /// Create a single-segment key from a string.
    pub fn from_single(value: impl AsRef<str>) -> Self {
        Self(Arc::from([Arc::from(value.as_ref())]))
    }

    /// The key segments.
    pub fn parts(&self) -> &[Arc<str>] {
        &self.0
    }

    /// Returns the single segment if this key has exactly one part, else `None`.
    ///
    /// Useful for backward compatibility with code that treated keys as plain strings.
    pub fn as_single(&self) -> Option<&str> {
        if self.0.len() == 1 {
            Some(&self.0[0])
        } else {
            None
        }
    }

    /// Returns the first segment as a string slice.
    ///
    /// Equivalent to `self.parts().first().map(|s| s.as_ref())`.
    pub fn as_str(&self) -> &str {
        match self.0.first() {
            Some(s) => s.as_ref(),
            None => "",
        }
    }

    /// Returns `true` if this key starts with the given `prefix`.
    ///
    /// An empty prefix matches everything. A key always starts with itself.
    pub fn starts_with(&self, prefix: &QueryKey) -> bool {
        if prefix.0.is_empty() {
            return true;
        }
        if prefix.0.len() > self.0.len() {
            return false;
        }
        self.0[..prefix.0.len()]
            .iter()
            .zip(prefix.0.iter())
            .all(|(a, b)| a == b)
    }

    /// Create a new key by appending an extra segment.
    pub fn join(&self, extra: &str) -> QueryKey {
        let mut parts: Vec<Arc<str>> = self.0.to_vec();
        parts.push(Arc::from(extra));
        Self(parts.into())
    }
}

impl Deref for QueryKey {
    type Target = [Arc<str>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ── From impls for backward compatibility ──────────────────────────────

impl From<&str> for QueryKey {
    fn from(value: &str) -> Self {
        Self::from_single(value)
    }
}

impl From<String> for QueryKey {
    fn from(value: String) -> Self {
        Self::from_single(value)
    }
}

impl<const N: usize> From<[&str; N]> for QueryKey {
    fn from(parts: [&str; N]) -> Self {
        Self::new(parts)
    }
}

impl From<Vec<&str>> for QueryKey {
    fn from(parts: Vec<&str>) -> Self {
        Self::new(parts)
    }
}

impl From<Vec<String>> for QueryKey {
    fn from(parts: Vec<String>) -> Self {
        Self::new(parts)
    }
}

// ── Serde: serialize as Vec<String>, deserialize from Vec<String> or String ──

impl Serialize for QueryKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let vec: Vec<&str> = self.0.iter().map(|s| s.as_ref()).collect();
        vec.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for QueryKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Accept either a JSON string or a JSON array of strings
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum KeyRepr {
            Array(Vec<String>),
            String(String),
        }

        match KeyRepr::deserialize(deserializer)? {
            KeyRepr::Array(parts) => Ok(Self::new(parts)),
            KeyRepr::String(s) => Ok(Self::from_single(s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_part_key_from_str() {
        let key = QueryKey::from("users");
        assert_eq!(key.parts().len(), 1);
        assert_eq!(key.as_str(), "users");
        assert_eq!(key.as_single(), Some("users"));
    }

    #[test]
    fn multi_part_key_from_array() {
        let key = QueryKey::from(["users", "42", "posts"]);
        assert_eq!(key.parts().len(), 3);
        assert_eq!(key.as_single(), None);
        assert_eq!(&*key[0], "users");
        assert_eq!(&*key[1], "42");
        assert_eq!(&*key[2], "posts");
    }

    #[test]
    fn starts_with_self() {
        let key = QueryKey::from(["users", "42"]);
        assert!(key.starts_with(&key));
    }

    #[test]
    fn starts_with_prefix() {
        let key = QueryKey::from(["users", "42", "posts"]);
        assert!(key.starts_with(&QueryKey::from(["users"])));
        assert!(key.starts_with(&QueryKey::from(["users", "42"])));
        assert!(!key.starts_with(&QueryKey::from(["users", "43"])));
        assert!(!key.starts_with(&QueryKey::from(["posts"])));
    }

    #[test]
    fn starts_with_empty_matches_everything() {
        let empty: QueryKey = QueryKey::new([] as [&str; 0]);
        let key = QueryKey::from("anything");
        assert!(key.starts_with(&empty));
    }

    #[test]
    fn starts_with_longer_prefix_fails() {
        let short = QueryKey::from("a");
        let long = QueryKey::from(["a", "b"]);
        assert!(!short.starts_with(&long));
    }

    #[test]
    fn join_appends_segment() {
        let base = QueryKey::from(["users", "42"]);
        let extended = base.join("posts");
        assert_eq!(extended.parts().len(), 3);
        assert_eq!(&*extended[2], "posts");
        // Original unchanged
        assert_eq!(base.parts().len(), 2);
    }

    #[test]
    fn clone_is_cheap() {
        let key = QueryKey::from(["a", "b", "c"]);
        let cloned = key.clone();
        assert_eq!(key, cloned);
        // Both share the same Arc allocation
        assert!(Arc::ptr_eq(&key.0, &cloned.0));
    }

    #[test]
    fn serde_roundtrip_array() {
        let key = QueryKey::from(["users", "42"]);
        let json = serde_json::to_string(&key).unwrap();
        assert_eq!(json, r#"["users","42"]"#);
        let back: QueryKey = serde_json::from_str(&json).unwrap();
        assert_eq!(key, back);
    }

    #[test]
    fn serde_roundtrip_single_string() {
        let key = QueryKey::from("users");
        let json = serde_json::to_string(&key).unwrap();
        // Single-part keys serialize as a one-element array
        let back: QueryKey = serde_json::from_str(&json).unwrap();
        assert_eq!(key, back);
    }

    #[test]
    fn serde_deserialize_from_plain_string() {
        // Backward compat: accept a JSON string for single-part keys
        let back: QueryKey = serde_json::from_str(r#""users""#).unwrap();
        assert_eq!(back, QueryKey::from("users"));
    }

    #[test]
    fn equality_and_ordering() {
        let a = QueryKey::from(["a", "b"]);
        let b = QueryKey::from(["a", "c"]);
        assert_ne!(a, b);
        assert!(a < b);
    }
}
