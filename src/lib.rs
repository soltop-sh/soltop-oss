pub mod rpc;
pub mod stats;
pub mod ui;

// Re-export for convenience (optional but nice)
// re-exports the function (so users can do soltop::get_rpc_url instead of soltop::rpc::get_rpc_url)
pub use stats::{MonitorConfig, NetworkMonitor, TransportKind};
