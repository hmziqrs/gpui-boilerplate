use std::collections::BTreeMap;

use gpui::Global;
use serde::{Deserialize, Serialize};

use crate::services::{
    http_lab::types::{HttpCookieSnapshot, HttpExchange, HttpLabAction},
    query::{QueryResource, RequestId, RequestSequencer},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpLabState {
    pub selected_action: HttpLabAction,
    pub(super) resources: BTreeMap<HttpLabAction, QueryResource<HttpExchange>>,
    pub(super) request_sequencer: RequestSequencer,
    pub history: Vec<HttpExchange>,
    pub transition_log: Vec<String>,
    pub cookies: Option<HttpCookieSnapshot>,
}

impl Default for HttpLabState {
    fn default() -> Self {
        let mut resources = BTreeMap::new();
        for action in HttpLabAction::all() {
            resources.insert(*action, resource_for_action(*action));
        }

        Self {
            selected_action: HttpLabAction::GetJson,
            resources,
            request_sequencer: RequestSequencer::new(),
            history: Vec::new(),
            transition_log: vec!["Idle".to_string()],
            cookies: None,
        }
    }
}

impl HttpLabState {
    pub fn resource(&self, action: HttpLabAction) -> &QueryResource<HttpExchange> {
        self.resources
            .get(&action)
            .expect("all http lab actions must have resources")
    }

    pub fn selected_resource(&self) -> &QueryResource<HttpExchange> {
        self.resource(self.selected_action)
    }

    pub fn active_count(&self) -> usize {
        self.resources
            .values()
            .filter(|resource| resource.active_request_id().is_some())
            .count()
    }

    pub(super) fn reset_for_user(&mut self) -> ResetRequests {
        let cancelled_requests = self
            .resources
            .values()
            .filter_map(|resource| resource.active_request_id())
            .collect::<Vec<_>>();
        let mut request_sequencer = self.request_sequencer.clone();
        request_sequencer.advance_scope();
        *self = Self::default();
        self.request_sequencer = request_sequencer;
        ResetRequests {
            request_ids: cancelled_requests,
        }
    }
}

impl Global for HttpLabState {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ResetRequests {
    pub(super) request_ids: Vec<RequestId>,
}

fn resource_for_action(action: HttpLabAction) -> QueryResource<HttpExchange> {
    QueryResource::new(
        action.query_key(),
        action.cache_policy(),
        action.request_policy(),
    )
}
