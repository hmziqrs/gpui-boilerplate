use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryStatus {
    Idle,
    LoadingEmpty,
    LoadingWithData,
    Success,
    Failure,
    Cancelled,
}

impl QueryStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::LoadingEmpty => "Loading empty",
            Self::LoadingWithData => "Loading with data",
            Self::Success => "Success",
            Self::Failure => "Failure",
            Self::Cancelled => "Cancelled",
        }
    }

    pub fn is_loading(self) -> bool {
        matches!(self, Self::LoadingEmpty | Self::LoadingWithData)
    }
}
