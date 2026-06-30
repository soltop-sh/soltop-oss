use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::{mpsc, RwLock};

use crate::rpc::{BlockData, RpcClient};
use crate::stats::NetworkState;

/// HTTP polling transport: track the chain head with `getSlot` and pull each
/// slot's block with `getBlock`. This is the default, universally-supported
/// path.
pub struct HttpTransport {
    rpc_url: String,
    poll_interval: Duration,
}

impl HttpTransport {
    pub fn new(rpc_url: String, poll_interval: Duration) -> Self {
        Self {
            rpc_url,
            poll_interval,
        }
    }

    pub async fn run(
        self,
        tx: mpsc::Sender<(u64, BlockData)>,
        state: Arc<RwLock<NetworkState>>,
    ) -> Result<()> {
        // Slot discovery and block fetching run concurrently so a slow getBlock
        // never holds up head tracking (and the lag indicator).
        let (slot_tx, slot_rx) = mpsc::channel::<u64>(100);

        let producer_state = Arc::clone(&state);
        let producer_client = RpcClient::new(self.rpc_url.clone());
        let poll_interval = self.poll_interval;
        let producer = tokio::spawn(async move {
            if let Err(e) = produce_slots(
                producer_client,
                poll_interval,
                slot_tx,
                Arc::clone(&producer_state),
            )
            .await
            {
                let mut s = producer_state.write().await;
                s.rpc_error = Some(format!("Producer stopped: {e}"));
            }
        });

        let fetch_client = RpcClient::new(self.rpc_url);
        fetch_blocks(fetch_client, slot_rx, tx).await;

        let _ = producer.await;
        Ok(())
    }
}

/// Continuously advance through slots, sending each one downstream to be
/// fetched. Loops forever; RPC failures back off and are surfaced via
/// `rpc_error`.
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

/// Fetch the block for each incoming slot and forward it to the processing
/// pipeline. Transient block-fetch errors are ignored; overall RPC health is
/// already reported by `produce_slots`.
async fn fetch_blocks(
    rpc_client: RpcClient,
    mut slot_rx: mpsc::Receiver<u64>,
    tx: mpsc::Sender<(u64, BlockData)>,
) {
    while let Some(slot) = slot_rx.recv().await {
        match rpc_client.get_block(slot).await {
            Ok(Some(block_response)) if block_response.result.is_some() => {
                let block_data = block_response.result.unwrap();
                if tx.send((slot, block_data)).await.is_err() {
                    break; // consumer gone
                }
            }
            Ok(_) => {
                // Block skipped or no data
            }
            Err(_e) => {
                // Transient block-fetch error; health surfaced via rpc_error.
            }
        }
    }
}
