#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppTimestamp(pub DateTime<Utc>);

impl AppTimestamp {
    pub fn now() -> Self {
        Self(Utc::now())
    }

    pub fn to_rfc3339(self) -> String {
        self.0.to_rfc3339()
    }
}

impl Default for AppTimestamp {
    fn default() -> Self {
        Self::now()
    }
}
