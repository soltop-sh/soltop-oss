use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, Stream, StreamExt};
use serde_json::json;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::rpc::{BlockData, BlockNotification, RpcClient, SlotNotification, SubscriptionAck};
use crate::stats::NetworkState;

const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// WebSocket push transport.
///
/// Subscribes with `blockSubscribe` so the validator streams full blocks as
/// they are produced — no `getSlot`/`getBlock` polling. Endpoints that don't
/// enable `blockSubscribe` fall back to `slotSubscribe` + `getBlock`, which
/// still removes the slot poll. Dropped connections reconnect with exponential
/// backoff, with the state surfaced through `rpc_error`.
pub struct WsTransport {
    ws_url: String,
    /// Used for `getBlock` in the `slotSubscribe` fallback.
    rpc_url: String,
}

impl WsTransport {
    pub fn new(ws_url: String, rpc_url: String) -> Self {
        Self { ws_url, rpc_url }
    }

    pub async fn run(
        self,
        tx: mpsc::Sender<(u64, BlockData)>,
        state: Arc<RwLock<NetworkState>>,
    ) -> Result<()> {
        let mut backoff = Duration::from_secs(1);
        loop {
            match self.connect_and_stream(&tx, &state).await {
                // Consumer dropped: the pipeline is shutting down, stop.
                Ok(()) => break,
                Err(e) => {
                    {
                        let mut s = state.write().await;
                        s.rpc_error = Some(format!(
                            "WebSocket error: {e} — reconnecting in {}s",
                            backoff.as_secs()
                        ));
                    }
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                }
            }
        }
        Ok(())
    }

    /// One connection lifecycle: connect, subscribe, and stream until the
    /// socket drops. Returns `Ok(())` only when the consumer is gone (clean
    /// shutdown); any connection or stream failure is `Err` so `run` reconnects.
    async fn connect_and_stream(
        &self,
        tx: &mpsc::Sender<(u64, BlockData)>,
        state: &Arc<RwLock<NetworkState>>,
    ) -> Result<()> {
        let (ws_stream, _) = connect_async(self.ws_url.as_str()).await?;
        let (mut write, mut read) = ws_stream.split();

        // Prefer blockSubscribe; fall back to slotSubscribe if it isn't enabled.
        let block_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "blockSubscribe",
            "params": ["all", {
                "encoding": "json",
                "transactionDetails": "full",
                "showRewards": false,
                "maxSupportedTransactionVersion": 0
            }]
        });
        write.send(Message::Text(block_req.to_string())).await?;

        let use_block_sub = if read_ack(&mut read).await?.error.is_some() {
            // blockSubscribe is unavailable on this endpoint; try slotSubscribe.
            let slot_req = json!({"jsonrpc": "2.0", "id": 1, "method": "slotSubscribe"});
            write.send(Message::Text(slot_req.to_string())).await?;
            if let Some(err) = read_ack(&mut read).await?.error {
                return Err(anyhow!(
                    "endpoint supports neither blockSubscribe nor slotSubscribe: {} \
                     (the validator must run with --rpc-pubsub-enable-block-subscription)",
                    err.message
                ));
            }
            false
        } else {
            true
        };

        // Connected and subscribed: healthy.
        {
            let mut s = state.write().await;
            s.rpc_error = None;
        }

        // The fallback path still needs getBlock.
        let rpc_client = RpcClient::new(self.rpc_url.clone());

        while let Some(msg) = read.next().await {
            let msg = msg?;

            if msg.is_ping() {
                write.send(Message::Pong(msg.into_data())).await?;
                continue;
            }
            if msg.is_close() {
                return Err(anyhow!("server closed the connection"));
            }

            let Ok(text) = msg.to_text() else {
                continue;
            };

            let block = if use_block_sub {
                match serde_json::from_str::<BlockNotification>(text) {
                    Ok(notif) => {
                        let value = notif.params.result.value;
                        value.block.map(|b| (value.slot, b))
                    }
                    Err(_) => None,
                }
            } else {
                match serde_json::from_str::<SlotNotification>(text) {
                    Ok(notif) => {
                        let slot = notif.params.result.slot;
                        fetch_block(&rpc_client, slot).await.map(|b| (slot, b))
                    }
                    Err(_) => None,
                }
            };

            if let Some((slot, block)) = block {
                {
                    let mut s = state.write().await;
                    s.update_latest_network_slot(slot);
                }
                if tx.send((slot, block)).await.is_err() {
                    return Ok(()); // consumer gone
                }
            }
        }

        Err(anyhow!("connection closed"))
    }
}

/// Read until the subscription reply arrives, ignoring anything else. The reply
/// carries either a subscription id (`result`) or an `error`; notifications
/// have neither, so they're skipped.
async fn read_ack<S>(read: &mut S) -> Result<SubscriptionAck>
where
    S: Stream<Item = Result<Message, WsError>> + Unpin,
{
    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Ok(text) = msg.to_text() {
            if let Ok(ack) = serde_json::from_str::<SubscriptionAck>(text) {
                if ack.result.is_some() || ack.error.is_some() {
                    return Ok(ack);
                }
            }
        }
    }
    Err(anyhow!(
        "connection closed before the subscription was confirmed"
    ))
}

/// Fetch a single block for the slotSubscribe fallback, returning `None` for a
/// skipped slot or a transient error (health is surfaced elsewhere).
async fn fetch_block(rpc_client: &RpcClient, slot: u64) -> Option<BlockData> {
    match rpc_client.get_block(slot).await {
        Ok(Some(response)) => response.result,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::rpc::{BlockNotification, SlotNotification, SubscriptionAck};

    // A blockNotification carries the same block shape as getBlock, so the
    // existing BlockData parses straight out of `params.result.value.block`.
    const BLOCK_NOTIFICATION: &str = r#"{
        "jsonrpc": "2.0",
        "method": "blockNotification",
        "params": {
            "subscription": 42,
            "result": {
                "context": { "slot": 112301554 },
                "value": {
                    "slot": 112301554,
                    "block": {
                        "transactions": [
                            {
                                "meta": {
                                    "err": null,
                                    "logMessages": [
                                        "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 3158 of 200207 compute units"
                                    ]
                                },
                                "transaction": {
                                    "message": {
                                        "accountKeys": ["TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"],
                                        "instructions": [ { "programIdIndex": 0 } ]
                                    }
                                }
                            }
                        ]
                    },
                    "err": null
                }
            }
        }
    }"#;

    #[test]
    fn parses_block_notification_into_block_data() {
        let notif: BlockNotification = serde_json::from_str(BLOCK_NOTIFICATION).unwrap();
        let value = notif.params.result.value;

        assert_eq!(value.slot, 112301554);
        let block = value.block.expect("block should be present");
        assert_eq!(block.transactions.len(), 1);
    }

    #[test]
    fn skipped_slot_has_no_block() {
        let json = r#"{
            "jsonrpc": "2.0",
            "method": "blockNotification",
            "params": {
                "subscription": 42,
                "result": {
                    "context": { "slot": 7 },
                    "value": { "slot": 7, "block": null, "err": null }
                }
            }
        }"#;

        let notif: BlockNotification = serde_json::from_str(json).unwrap();
        assert_eq!(notif.params.result.value.slot, 7);
        assert!(notif.params.result.value.block.is_none());
    }

    #[test]
    fn parses_slot_notification() {
        let json = r#"{
            "jsonrpc": "2.0",
            "method": "slotNotification",
            "params": {
                "subscription": 1,
                "result": { "parent": 99, "root": 50, "slot": 100 }
            }
        }"#;

        let notif: SlotNotification = serde_json::from_str(json).unwrap();
        assert_eq!(notif.params.result.slot, 100);
    }

    #[test]
    fn subscription_ack_reports_success_and_error() {
        let confirm: SubscriptionAck =
            serde_json::from_str(r#"{"jsonrpc":"2.0","result":2401,"id":1}"#).unwrap();
        assert_eq!(confirm.result, Some(2401));
        assert!(confirm.error.is_none());

        let rejected: SubscriptionAck = serde_json::from_str(
            r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"Method not found"},"id":1}"#,
        )
        .unwrap();
        assert!(rejected.result.is_none());
        assert_eq!(rejected.error.unwrap().message, "Method not found");
    }

    // read_ack relies on notifications deserializing to an ack with neither
    // field set, so they can be skipped while waiting for the real reply.
    #[test]
    fn notification_is_not_mistaken_for_an_ack() {
        let ack: SubscriptionAck = serde_json::from_str(BLOCK_NOTIFICATION).unwrap();
        assert!(ack.result.is_none() && ack.error.is_none());
    }
}
