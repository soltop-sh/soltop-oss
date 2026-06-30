//! RPC client for interacting with Solana nodes
//!
//! This module provides functionality to fetch slots and blocks from any Solana RPC endpoint.

mod client;
mod parser;
mod types;

pub use client::RpcClient;
pub use parser::{extract_program_cu, extract_program_cu_timed};
pub use types::{
    BlockData, BlockNotification, LogMessage, SlotNotification, SlotResponse, SubscriptionAck,
    TransactionData,
};
