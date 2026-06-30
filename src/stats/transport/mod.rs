//! Block sources feeding the monitoring pipeline.
//!
//! A transport produces `(slot, BlockData)` pairs for the consumer to hand to
//! `NetworkState::process_block`. The HTTP transport polls (`getSlot` +
//! `getBlock`); the WebSocket transport subscribes and receives pushed blocks.
//! Both are interchangeable, so the stats and UI layers don't care which is in
//! use.

mod http;
mod ws;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use clap::ValueEnum;
use tokio::sync::{mpsc, RwLock};

use crate::rpc::BlockData;
use crate::stats::NetworkState;

use http::HttpTransport;
use ws::WsTransport;

/// Which block source to use.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum TransportKind {
    /// HTTP polling with `getSlot` + `getBlock`. Works on every endpoint.
    #[default]
    Http,
    /// WebSocket `blockSubscribe`. Requires an endpoint that enables it.
    Ws,
}

/// A configured block source.
pub enum Transport {
    Http(HttpTransport),
    Ws(WsTransport),
}

impl Transport {
    /// Build a transport from the resolved config. For `Ws`, `ws_url` is used
    /// directly when present, otherwise it's derived from `rpc_url`.
    pub fn new(
        kind: TransportKind,
        rpc_url: String,
        ws_url: Option<String>,
        poll_interval: Duration,
    ) -> Self {
        match kind {
            TransportKind::Http => Transport::Http(HttpTransport::new(rpc_url, poll_interval)),
            TransportKind::Ws => {
                let ws_url = ws_url.unwrap_or_else(|| derive_ws_url(&rpc_url));
                Transport::Ws(WsTransport::new(ws_url, rpc_url))
            }
        }
    }

    /// Run until the consumer end of `tx` is dropped. Connection health is
    /// reported through `state.rpc_error` rather than printed (stdout is the
    /// same terminal the TUI draws on).
    pub async fn run(
        self,
        tx: mpsc::Sender<(u64, BlockData)>,
        state: Arc<RwLock<NetworkState>>,
    ) -> Result<()> {
        match self {
            Transport::Http(t) => t.run(tx, state).await,
            Transport::Ws(t) => t.run(tx, state).await,
        }
    }
}

/// Derive a WebSocket URL from an HTTP RPC URL by swapping the scheme. Anything
/// that isn't `http(s)://` is passed through unchanged.
pub fn derive_ws_url(rpc_url: &str) -> String {
    if let Some(rest) = rpc_url.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = rpc_url.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        rpc_url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_secure_ws_from_https() {
        assert_eq!(
            derive_ws_url("https://api.mainnet-beta.solana.com"),
            "wss://api.mainnet-beta.solana.com"
        );
    }

    #[test]
    fn derives_plain_ws_from_http() {
        assert_eq!(
            derive_ws_url("http://localhost:8899"),
            "ws://localhost:8899"
        );
    }

    #[test]
    fn preserves_path_and_port() {
        assert_eq!(
            derive_ws_url("https://example.com:443/rpc/key"),
            "wss://example.com:443/rpc/key"
        );
    }

    #[test]
    fn passes_through_non_http_scheme() {
        assert_eq!(derive_ws_url("wss://already.ws"), "wss://already.ws");
    }
}
