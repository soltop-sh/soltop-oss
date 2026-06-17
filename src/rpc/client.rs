use anyhow::{Context, Result};
use reqwest;
use serde_json::json;
use std::time::Duration;

use super::types::{BlockResponse, SlotResponse};

/// Fail fast if the host can't be reached — without this, an unreachable or
/// silently-dropping endpoint makes `.send()` hang indefinitely, which stalls
/// the monitor before its retry/error-surfacing logic ever runs.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Overall per-request budget. Generous enough for large `getBlock` responses,
/// short enough that a wedged connection turns into a visible error.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Client for interacting with Solana RPC endpoints
pub struct RpcClient {
    url: String,
    client: reqwest::Client,
}

impl RpcClient {
    /// Create a new RPC client
    pub fn new(url: String) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            // builder() only fails if the TLS backend can't initialize; fall
            // back to a default client so construction stays infallible.
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { url, client }
    }

    /// Fetch the latest slot number
    pub async fn get_latest_slot(&self) -> Result<u64> {
        let params = json!([]);

        let response: SlotResponse = self
            .call_rpc("getSlot", params)
            .await
            .context("Failed to get latest slot")?;

        Ok(response.result)
    }

    /// Fetch block data for a given slot
    pub async fn get_block(&self, slot: u64) -> Result<Option<BlockResponse>> {
        let params = json!([slot, {
            "encoding": "json",
            "transactionDetails": "full",
            "rewards": false,
            "maxSupportedTransactionVersion": 0
        }]);

        let response: BlockResponse = self
            .call_rpc("getBlock", params)
            .await
            .context(format!("Failed to get block {}", slot))?;

        Ok(Some(response))
    }

    /// Helper: Make a JSON-RPC request
    async fn call_rpc<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T> {
        let request_body = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": method,
            "params": params,
        });

        let response = self
            .client
            .post(&self.url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send RPC request")?;

        let parsed = response
            .json::<T>()
            .await
            .context("Failed to parse RPC response")?;

        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a test client
    fn test_client() -> RpcClient {
        RpcClient::new("https://api.mainnet-beta.solana.com".to_string())
    }

    #[tokio::test]
    #[ignore] // Only run with --ignored flag (requires network)
    async fn test_get_latest_slot() {
        let client = test_client();
        let slot = client.get_latest_slot().await.expect("Failed to get slot");

        // Slot should be a large positive number
        assert!(slot > 0, "Slot should be positive");
        println!("Current slot: {}", slot);
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_block() {
        let client = test_client();

        // First get current slot
        let slot = client.get_latest_slot().await.expect("Failed to get slot");

        // Fetch a recent block (current - 10 to avoid skipped slots)
        let block = client
            .get_block(slot - 10)
            .await
            .expect("Failed to get block");

        assert!(block.is_some(), "Block should exist");

        if let Some(block_response) = block {
            let block_data = block_response.result.expect("Block should have data");
            println!("Block has {} transactions", block_data.transactions.len());
            assert!(
                !block_data.transactions.is_empty(),
                "Block should have transactions"
            );
        }
    }
}
