use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

use super::network::NetworkState;
use crate::rpc::RpcClient;

/// Configuration for the network monitor
pub struct MonitorConfig {
    pub rpc_url: String,
    pub window_duration: Duration,
    pub buffer_capacity: usize,
    pub poll_interval: Duration, // How often to fetch new slots
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            window_duration: Duration::from_secs(5 * 60), // 5 minutes
            buffer_capacity: 750,                         // ~5 minutes at 400ms/slot
            poll_interval: Duration::from_millis(400),    // Match slot time
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

    /// Producer task: continuously fetch slots and send to channel
    async fn produce_slots(
        rpc_client: RpcClient,
        poll_interval: Duration,
        tx: mpsc::Sender<u64>,
        state: Arc<RwLock<NetworkState>>,
    ) -> Result<()> {
        // Initial connection with retry
        let mut current_slot;
        let mut retry_delay = Duration::from_secs(1);
        loop {
            match rpc_client.get_latest_slot().await {
                Ok(slot) => {
                    current_slot = slot;
                    let mut s = state.write().await;
                    s.rpc_error = None;
                    break;
                }
                Err(e) => {
                    let msg = format!(
                        "RPC connection failed: {e} — retrying in {}s",
                        retry_delay.as_secs()
                    );
                    {
                        let mut s = state.write().await;
                        s.rpc_error = Some(msg);
                    }
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = (retry_delay * 2).min(Duration::from_secs(30));
                }
            }
        }

        let mut consecutive_errors = 0u32;
        loop {
            match rpc_client.get_latest_slot().await {
                Ok(latest_slot) => {
                    consecutive_errors = 0;
                    {
                        let mut s = state.write().await;
                        s.rpc_error = None;
                        s.update_latest_network_slot(latest_slot);
                    }

                    if current_slot <= latest_slot {
                        if tx.send(current_slot).await.is_err() {
                            break;
                        }
                        current_slot += 1;
                    } else {
                        tokio::time::sleep(poll_interval).await;
                    }
                }
                Err(e) => {
                    consecutive_errors += 1;
                    let backoff = Duration::from_secs((2u64).pow(consecutive_errors.min(5)));
                    let msg = format!(
                        "RPC error: {e} — retry #{consecutive_errors} in {}s",
                        backoff.as_secs()
                    );
                    {
                        let mut s = state.write().await;
                        s.rpc_error = Some(msg);
                    }
                    tokio::time::sleep(backoff).await;
                }
            }
        }
        Ok(())
    }

    /// Consumer task: receive slots from channel and update state
    async fn consume_slots(
        state: Arc<RwLock<NetworkState>>,
        rpc_client: RpcClient,
        mut rx: mpsc::Receiver<u64>,
    ) -> Result<()> {
        while let Some(slot) = rx.recv().await {
            match rpc_client.get_block(slot).await {
                Ok(Some(block_response)) if block_response.result.is_some() => {
                    // Happy path: block exists and has data
                    let block_data = block_response.result.unwrap();

                    {
                        // Explicit scope for lock
                        let mut state = state.write().await;
                        state.process_block(slot, &block_data, false);
                    } // Lock dropped here
                }
                Ok(_) => {
                    // Block skipped or no data
                }
                Err(_e) => {
                    // Transient block-fetch error. Don't print — stdout/stderr
                    // is the same terminal ratatui is drawing on, so any print
                    // corrupts the TUI. Overall RPC health is already surfaced
                    // via NetworkState.rpc_error by the producer.
                }
            }
        }

        // Channel closed → producer gone. Nothing to print (would corrupt the TUI).
        Ok(())
    }

    /// Start the monitoring pipeline
    /// This function runs forever (until Ctrl+C)
    pub async fn start(&self) -> Result<()> {
        let (tx, rx) = mpsc::channel::<u64>(100);

        // Clone data for consumer
        let consumer_state = Arc::clone(&self.state);
        let rpc_client = RpcClient::new(self.config.rpc_url.clone());

        // Clone data for producer
        let producer_client = RpcClient::new(self.config.rpc_url.clone());
        let producer_state = Arc::clone(&self.state);
        let poll_interval = self.config.poll_interval;

        // Spawn producer. produce_slots loops forever; if it ever does return
        // an error, surface it via shared state instead of printing onto the
        // TUI (stdout is the same terminal ratatui draws on).
        let producer = tokio::spawn(async move {
            if let Err(e) = Self::produce_slots(
                producer_client,
                poll_interval,
                tx,
                Arc::clone(&producer_state),
            )
            .await
            {
                let mut s = producer_state.write().await;
                s.rpc_error = Some(format!("Producer stopped: {e}"));
            }
        });

        // Spawn consumer
        let consumer = tokio::spawn(async move {
            if let Err(e) = Self::consume_slots(Arc::clone(&consumer_state), rpc_client, rx).await {
                let mut s = consumer_state.write().await;
                s.rpc_error = Some(format!("Consumer stopped: {e}"));
            }
        });

        let _ = tokio::join!(producer, consumer);
        Ok(())
    }
}
