//! WebSocket client scaffold.
//!
//! To use, add to `Cargo.toml`:
//! ```toml
//! tokio-tungstenite = { version = "0.24", features = ["native-tls"] }
//! ```
//!
//! Then enable the `websocket` feature in your build:
//! ```toml
//! [features]
//! websocket = ["dep:tokio-tungstenite"]
//! ```
//!
//! Integration with GPUI context uses `cx.spawn()` for async operations,
//! following the same pattern as `connectivity::check_now` and `tasks::start_demo_task`.

#![allow(dead_code)]

#[derive(Debug, thiserror::Error)]
pub enum WebSocketError {
    #[error("connection failed: {0}")]
    Connection(String),
    #[cfg(feature = "websocket")]
    #[error("send failed: {0}")]
    Send(#[source] tokio_tungstenite::tungstenite::Error),
    #[cfg(feature = "websocket")]
    #[error("close failed: {0}")]
    Close(#[source] tokio_tungstenite::tungstenite::Error),
    #[error("not connected")]
    NotConnected,
    #[error("websocket feature not enabled")]
    FeatureDisabled,
}

// ---------------------------------------------------------------------------
// When the `websocket` feature is enabled, pull in the real dependency.
// Everything below compiles without it when the feature is off.
// ---------------------------------------------------------------------------
#[cfg(feature = "websocket")]
use tokio_tungstenite::tungstenite::Message;

// ---------------------------------------------------------------------------
// Types that are always available (no dependency required).
// ---------------------------------------------------------------------------

/// Connection state machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u8 },
    Closed,
}

/// Callback signature for incoming WebSocket messages.
///
/// Use this to wire message handling into GPUI context:
/// ```ignore
/// let handler: MessageHandler = Box::new(|text: &str| {
///     // Parse and dispatch into app state
/// });
/// ```
pub type MessageHandler = Box<dyn Fn(&str) + Send + Sync>;

/// Configuration for reconnection behaviour.
#[derive(Clone, Debug)]
pub struct ReconnectPolicy {
    /// Maximum number of reconnection attempts before giving up.
    pub max_retries: u8,
    /// Base delay for exponential backoff (milliseconds).
    pub base_delay_ms: u64,
    /// Optional cap so backoff does not grow indefinitely (milliseconds).
    pub max_delay_ms: Option<u64>,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            base_delay_ms: 500,
            max_delay_ms: Some(30_000),
        }
    }
}

impl ReconnectPolicy {
    /// Returns the delay for a given attempt number (0-indexed).
    ///
    /// Formula: `base_delay_ms * 2^attempt`, capped at `max_delay_ms`.
    pub fn delay_for_attempt(&self, attempt: u8) -> std::time::Duration {
        let exp = 1u64.checked_shl(attempt as u32).unwrap_or(u64::MAX);
        let raw = self.base_delay_ms.saturating_mul(exp);
        let capped = self.max_delay_ms.map_or(raw, |cap| raw.min(cap));
        std::time::Duration::from_millis(capped)
    }
}

// ---------------------------------------------------------------------------
// Feature-gated implementation (requires tokio-tungstenite).
// ---------------------------------------------------------------------------

#[cfg(feature = "websocket")]
mod live {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio_tungstenite::{connect_async, tungstenite::protocol::CloseFrame};

    /// Shared inner state behind an async mutex so `cx.spawn` tasks can
    /// coordinate safely.
    type InnerSink = Arc<
        Mutex<
            Option<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
            >,
        >,
    >;

    /// Minimal WebSocket client with automatic reconnection.
    ///
    /// # GPUI integration
    ///
    /// Spawn the connect loop from a GPUI context:
    /// ```ignore
    /// let client = WebSocketClient::new("wss://example.com/ws".into());
    /// let inner = client.inner.clone();
    /// cx.spawn(async move |cx| {
    ///     cx.background_executor()
    ///         .spawn(client.connect_loop())
    ///         .await
    ///         .ok();
    /// }).detach();
    /// ```
    pub struct WebSocketClient {
        pub url: String,
        pub state: ConnectionState,
        pub reconnect: ReconnectPolicy,
        pub on_message: Option<MessageHandler>,
        inner: InnerSink,
    }

    impl WebSocketClient {
        /// Create a new client targeting the given WebSocket URL.
        pub fn new(url: String) -> Self {
            Self {
                url,
                state: ConnectionState::Disconnected,
                reconnect: ReconnectPolicy::default(),
                on_message: None,
                inner: Arc::new(Mutex::new(None)),
            }
        }

        /// Set a custom reconnection policy.
        pub fn with_reconnect_policy(mut self, policy: ReconnectPolicy) -> Self {
            self.reconnect = policy;
            self
        }

        /// Set a message handler callback.
        pub fn on_message(mut self, handler: MessageHandler) -> Self {
            self.on_message = Some(handler);
            self
        }

        /// Connect to the WebSocket server with exponential-backoff retries.
        ///
        /// This is a self-contained async loop suitable for spawning via
        /// `cx.background_executor().spawn(...)` inside a `cx.spawn` block.
        pub async fn connect_loop(&mut self) -> Result<(), WebSocketError> {
            let mut attempt: u8 = 0;

            loop {
                self.state = if attempt == 0 {
                    ConnectionState::Connecting
                } else {
                    ConnectionState::Reconnecting { attempt }
                };

                match connect_async(&self.url).await {
                    Ok((ws_stream, _response)) => {
                        tracing::info!(
                            target: "gpui_starter::websocket",
                            url = %self.url,
                            "connected"
                        );
                        self.state = ConnectionState::Connected;
                        attempt = 0;

                        let (_write, mut read) = ws_stream.split();
                        {
                            let mut guard = self.inner.lock().await;
                            // TODO(github): store `write` half in inner for `send()`.
                            *guard = None;
                        }

                        // Read messages until the stream closes or errors.
                        while let Some(msg) = tokio_stream::StreamExt::next(&mut read).await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if let Some(ref handler) = self.on_message {
                                        handler(&text);
                                    }
                                }
                                Ok(Message::Close(frame)) => {
                                    tracing::info!(
                                        target: "gpui_starter::websocket",
                                        frame = ?frame,
                                        "server closed connection"
                                    );
                                    break;
                                }
                                Ok(_) => {} // binary, ping, pong — ignored for now
                                Err(e) => {
                                    tracing::warn!(
                                        target: "gpui_starter::websocket",
                                        error = %e,
                                        "read error"
                                    );
                                    break;
                                }
                            }
                        }

                        // Stream ended — fall through to reconnect.
                        self.state = ConnectionState::Disconnected;
                        {
                            let mut guard = self.inner.lock().await;
                            *guard = None;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "gpui_starter::websocket",
                            attempt,
                            error = %e,
                            "connection failed"
                        );
                    }
                }

                attempt += 1;
                if attempt > self.reconnect.max_retries {
                    tracing::error!(
                        target: "gpui_starter::websocket",
                        max_retries = self.reconnect.max_retries,
                        "exceeded max retries, giving up"
                    );
                    self.state = ConnectionState::Closed;
                    return Err(WebSocketError::Connection(format!(
                        "failed after {} retries",
                        self.reconnect.max_retries
                    )));
                }

                let delay = self.reconnect.delay_for_attempt(attempt - 1);
                tracing::info!(
                    target: "gpui_starter::websocket",
                    attempt,
                    delay_ms = delay.as_millis() as u64,
                    "waiting before reconnect"
                );
                tokio::time::sleep(delay).await;
            }
        }

        /// Send a text message over the active connection.
        ///
        /// Returns an error if the socket is not currently connected.
        pub async fn send(&self, message: &str) -> Result<(), WebSocketError> {
            let mut guard = self.inner.lock().await;
            match guard.as_mut() {
                Some(ws) => ws
                    .send(Message::Text(message.into()))
                    .await
                    .map_err(WebSocketError::Send),
                None => Err(WebSocketError::NotConnected),
            }
        }

        /// Gracefully close the WebSocket connection.
        pub async fn close(&mut self) -> Result<(), WebSocketError> {
            let mut guard = self.inner.lock().await;
            if let Some(ws) = guard.take() {
                ws.close(None).await.map_err(WebSocketError::Close)?;
            }
            self.state = ConnectionState::Closed;
            Ok(())
        }
    }
}

#[cfg(feature = "websocket")]
pub use live::WebSocketClient;

// ---------------------------------------------------------------------------
// Stub when the feature is disabled — keeps the module compilable.
// ---------------------------------------------------------------------------

#[cfg(not(feature = "websocket"))]
mod stub {
    use super::*;

    /// Placeholder WebSocket client (feature `websocket` is not enabled).
    ///
    /// Enable the feature and add `tokio-tungstenite` to `Cargo.toml`
    /// to get the real implementation.
    pub struct WebSocketClient {
        pub url: String,
        pub state: ConnectionState,
        pub reconnect: ReconnectPolicy,
    }

    impl WebSocketClient {
        pub fn new(url: String) -> Self {
            Self {
                url,
                state: ConnectionState::Disconnected,
                reconnect: ReconnectPolicy::default(),
            }
        }

        pub fn with_reconnect_policy(self, _policy: ReconnectPolicy) -> Self {
            self
        }

        /// No-op when the websocket feature is disabled.
        pub async fn connect_loop(&mut self) -> Result<(), WebSocketError> {
            Err(WebSocketError::FeatureDisabled)
        }

        /// No-op when the websocket feature is disabled.
        pub async fn send(&self, _message: &str) -> Result<(), WebSocketError> {
            Err(WebSocketError::FeatureDisabled)
        }

        /// No-op when the websocket feature is disabled.
        pub async fn close(&mut self) -> Result<(), WebSocketError> {
            Err(WebSocketError::FeatureDisabled)
        }
    }
}

#[cfg(not(feature = "websocket"))]
#[allow(unused_imports)]
pub use stub::WebSocketClient;

// ---------------------------------------------------------------------------
// GPUI integration helpers (always compiled).
// ---------------------------------------------------------------------------

/// Spawn a WebSocket client connect loop from GPUI context.
///
/// ```ignore
/// // In your app init or action handler:
/// let url = "wss://example.com/ws".to_string();
/// websocket::spawn_connect(url, cx);
/// ```
///
/// This mirrors the pattern in `connectivity::check_now` and `tasks::start_demo_task`:
/// `cx.spawn` creates a GPUI-managed async context, and the heavy I/O is
/// delegated to `background_executor().spawn(...)`.
#[cfg(feature = "websocket")]
pub fn spawn_connect(url: String, cx: &mut gpui::App) {
    let mut client = WebSocketClient::new(url);
    cx.spawn(async move |cx| {
        cx.background_executor()
            .spawn(async move {
                if let Err(e) = client.connect_loop().await {
                    tracing::error!(
                        target: "gpui_starter::websocket",
                        error = %e,
                        "websocket connect loop terminated with error"
                    );
                }
            })
            .await
    })
    .detach();
}

// ---------------------------------------------------------------------------
// Tests (always compiled, no dependency required).
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "websocket.test.rs"]
mod websocket_test;
