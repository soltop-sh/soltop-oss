use serde::{Deserialize, Serialize};

/// Generic JSON-RPC response wrapper
#[derive(Debug, Deserialize, Serialize)]
pub struct RpcResponse<T> {
    pub jsonrpc: String,
    pub result: T,
    pub id: String,
}

/// Response for getSlot method
pub type SlotResponse = RpcResponse<u64>;

/// A transaction within a block
#[derive(Debug, Deserialize, Serialize)]
pub struct TransactionData {
    pub meta: Option<TransactionMeta>,
    pub transaction: Transaction,
}

/// Transaction metadata (includes logs and status)
#[derive(Debug, Deserialize, Serialize)]
pub struct TransactionMeta {
    pub err: Option<serde_json::Value>, // null if success, error details if failed
    #[serde(rename = "logMessages", default)]
    pub log_messages: Option<Vec<String>>,
}

/// Transaction details
#[derive(Debug, Deserialize, Serialize)]
pub struct Transaction {
    pub message: Message,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Instruction {
    #[serde(rename = "programIdIndex")]
    pub program_id_index: u8,
    // Note: There are other fields (accounts, data) but we don't need them yet
}

// Update Message to include instructions
#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    #[serde(rename = "accountKeys")]
    pub account_keys: Vec<String>,
    pub instructions: Vec<Instruction>,
}

/// Block data response
#[derive(Debug, Deserialize, Serialize)]
pub struct BlockData {
    pub transactions: Vec<TransactionData>,
}

pub type BlockResponse = RpcResponse<Option<BlockData>>;

/// Log message extracted from transaction
#[derive(Debug, Clone)]
pub struct LogMessage {
    pub program_id: String,
    pub message: String,
}

/// JSON-RPC error object, returned when a pubsub method isn't enabled on the
/// endpoint (e.g. `blockSubscribe` without `--rpc-pubsub-enable-block-subscription`).
#[derive(Debug, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

/// Reply to a `*Subscribe` request: `result` holds the subscription id on
/// success, `error` is populated when the method is unavailable.
#[derive(Debug, Deserialize)]
pub struct SubscriptionAck {
    pub result: Option<u64>,
    pub error: Option<RpcError>,
}

/// `blockNotification` pushed by `blockSubscribe`.
#[derive(Debug, Deserialize)]
pub struct BlockNotification {
    pub params: BlockNotificationParams,
}

#[derive(Debug, Deserialize)]
pub struct BlockNotificationParams {
    pub result: BlockNotificationResult,
}

#[derive(Debug, Deserialize)]
pub struct BlockNotificationResult {
    pub value: BlockNotificationValue,
}

#[derive(Debug, Deserialize)]
pub struct BlockNotificationValue {
    pub slot: u64,
    /// Absent when the slot was skipped or the block isn't available yet.
    pub block: Option<BlockData>,
}

/// `slotNotification` pushed by `slotSubscribe` (used by the fallback path).
#[derive(Debug, Deserialize)]
pub struct SlotNotification {
    pub params: SlotNotificationParams,
}

#[derive(Debug, Deserialize)]
pub struct SlotNotificationParams {
    pub result: SlotNotificationResult,
}

#[derive(Debug, Deserialize)]
pub struct SlotNotificationResult {
    pub slot: u64,
}
