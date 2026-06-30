use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

use super::network::NetworkState;
use super::transport::{Transport, TransportKind};
use crate::rpc::BlockData;

/// Configuration for the network monitor
pub struct MonitorConfig {
    pub rpc_url: String,
    pub window_duration: Duration,
    pub buffer_capacity: usize,
    pub poll_interval: Duration, // How often to fetch new slots (HTTP transport)
    pub transport: TransportKind,
    pub ws_url: Option<String>, // Explicit WebSocket URL; derived from rpc_url when None
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            window_duration: Duration::from_secs(5 * 60), // 5 minutes
            buffer_capacity: 750,                         // ~5 minutes at 400ms/slot
            poll_interval: Duration::from_millis(400),    // Match slot time
            transport: TransportKind::Http,
            ws_url: None,
        }
    }
}

/// Main network monitoring coordinator
pub struct NetworkMonitor {
    config: MonitorConfig,
    state: Arc<RwLock<NetworkState>>,
}

impl NetworkMonitor {
    /// Create a new network monitor
    pub fn new(config: MonitorConfig) -> Self {
        let state = Arc::new(RwLock::new(NetworkState::new(
            config.window_duration,
            config.buffer_capacity,
        )));

        Self { config, state }
    }

    /// Get a clone of the shared state (for consumers to access)
    pub fn get_state(&self) -> Arc<RwLock<NetworkState>> {
        Arc::clone(&self.state)
    }

    /// Start the monitoring pipeline
    /// This function runs forever (until Ctrl+C)
    pub async fn start(&self) -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<(u64, BlockData)>(100);

        // Producer: the selected transport feeds (slot, block) pairs in. It
        // loops forever; if it ever returns an error, surface it via shared
        // state instead of printing onto the TUI (stdout is the same terminal
        // ratatui draws on).
        let transport = Transport::new(
            self.config.transport,
            self.config.rpc_url.clone(),
            self.config.ws_url.clone(),
            self.config.poll_interval,
        );
        let producer_state = Arc::clone(&self.state);
        let producer = tokio::spawn(async move {
            if let Err(e) = transport.run(tx, Arc::clone(&producer_state)).await {
                let mut s = producer_state.write().await;
                s.rpc_error = Some(format!("Transport stopped: {e}"));
            }
        });

        // Consumer: process each block regardless of how it arrived.
        let consumer_state = Arc::clone(&self.state);
        let consumer = tokio::spawn(async move {
            while let Some((slot, block)) = rx.recv().await {
                let mut s = consumer_state.write().await;
                s.process_block(slot, &block, false);
            }
        });

        let _ = tokio::join!(producer, consumer);
        Ok(())
    }
}
