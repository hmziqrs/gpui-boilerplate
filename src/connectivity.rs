use gpui::{App, Global};
use network_interface::NetworkInterfaceConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectivityState {
    Unknown,
    Online,
    Offline,
    CaptiveOrFiltered,
}

#[derive(Clone, Debug)]
pub struct ConnectivitySnapshot {
    pub state: ConnectivityState,
    pub probe_url: String,
    pub interfaces: Vec<String>,
    pub last_error: Option<String>,
}

impl Default for ConnectivitySnapshot {
    fn default() -> Self {
        Self {
            state: ConnectivityState::Unknown,
            probe_url: "https://example.com".to_string(),
            interfaces: Vec::new(),
            last_error: None,
        }
    }
}

impl Global for ConnectivitySnapshot {}

pub fn initialize(cx: &mut App) {
    cx.set_global(ConnectivitySnapshot::default());
}

pub fn snapshot(cx: &App) -> ConnectivitySnapshot {
    cx.try_global::<ConnectivitySnapshot>()
        .cloned()
        .unwrap_or_default()
}

pub fn check_now(cx: &mut App) {
    let probe_url = snapshot(cx).probe_url;
    let interfaces = read_interfaces();
    cx.spawn(async move |cx| {
        let result = reqwest::get(&probe_url).await;
        cx.update(move |cx| {
            let mut next = snapshot(cx);
            next.interfaces = interfaces;
            match result {
                Ok(response) if response.status().is_success() => {
                    next.state = ConnectivityState::Online;
                    next.last_error = None;
                }
                Ok(response) => {
                    next.state = ConnectivityState::CaptiveOrFiltered;
                    next.last_error = Some(format!("probe status {}", response.status()));
                }
                Err(err) => {
                    next.state = ConnectivityState::Offline;
                    next.last_error = Some(err.to_string());
                }
            }
            tracing::info!(
                target: "gpui_starter::connectivity",
                state = ?next.state,
                probe_url = %next.probe_url,
                last_error = ?next.last_error,
                interfaces = ?next.interfaces,
                "connectivity probe completed"
            );
            cx.set_global(next);
        });
    })
    .detach();
}

fn read_interfaces() -> Vec<String> {
    network_interface::NetworkInterface::show()
        .map(|ifaces| {
            ifaces
                .into_iter()
                .map(|iface| format!("{}:{}", iface.name, iface.addr.len()))
                .collect()
        })
        .unwrap_or_default()
}
