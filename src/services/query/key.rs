use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QueryKey(String);

impl QueryKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for QueryKey {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for QueryKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
