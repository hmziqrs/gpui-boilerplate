use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryErrorKind {
    Cancelled,
    Response,
    Transport,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryError {
    kind: QueryErrorKind,
    message: String,
}

impl QueryError {
    pub fn new(kind: QueryErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::new(QueryErrorKind::Cancelled, message)
    }

    pub fn response(message: impl Into<String>) -> Self {
        Self::new(QueryErrorKind::Response, message)
    }

    pub fn transport(message: impl Into<String>) -> Self {
        Self::new(QueryErrorKind::Transport, message)
    }

    pub fn unknown(message: impl Into<String>) -> Self {
        Self::new(QueryErrorKind::Unknown, message)
    }

    pub fn kind(&self) -> QueryErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl AsRef<str> for QueryError {
    fn as_ref(&self) -> &str {
        self.message()
    }
}

impl From<String> for QueryError {
    fn from(value: String) -> Self {
        Self::unknown(value)
    }
}

impl From<&str> for QueryError {
    fn from(value: &str) -> Self {
        Self::unknown(value)
    }
}
