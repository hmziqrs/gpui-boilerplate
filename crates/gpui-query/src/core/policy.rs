use serde::{Deserialize, Serialize};

use super::{QueryStatus, RequestId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CachePolicy {
    NoCache,
    Ttl { ttl_ms: u64 },
    StaleWhileRevalidate { ttl_ms: u64 },
}

impl CachePolicy {
    pub fn label(self) -> String {
        match self {
            Self::NoCache => "No cache".to_string(),
            Self::Ttl { ttl_ms } => format!("Cache TTL {}s", ttl_ms / 1_000),
            Self::StaleWhileRevalidate { ttl_ms } => {
                format!("Stale-while-revalidate {}s", ttl_ms / 1_000)
            }
        }
    }

    pub fn can_short_circuit(self) -> bool {
        matches!(self, Self::Ttl { .. })
    }

    pub fn ttl_ms(self) -> Option<u64> {
        match self {
            Self::NoCache => None,
            Self::Ttl { ttl_ms } | Self::StaleWhileRevalidate { ttl_ms } => Some(ttl_ms),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestPolicy {
    LatestWins,
    IgnoreWhileLoading,
}

impl RequestPolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::LatestWins => "Latest wins",
            Self::IgnoreWhileLoading => "Ignore while loading",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueryFetchMode {
    #[default]
    Normal,
    Force,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryBeginResult {
    Started {
        request_id: RequestId,
        status: QueryStatus,
        replaced_request_id: Option<RequestId>,
    },
    CacheHit,
    IgnoredWhileLoading {
        active_request_id: RequestId,
    },
}
