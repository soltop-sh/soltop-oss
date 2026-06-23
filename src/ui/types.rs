use crate::stats::SlotStats;
use std::time::{Duration, Instant};

/// View mode for displaying statistics
#[derive(Clone, Copy, PartialEq)]
pub enum ViewMode {
    Live,   // Current behavior - shows recent activity
    Window, // Shows aggregate stats for entire window
}

/// Type of chart to display in detail view
#[derive(Clone, Copy, PartialEq)]
pub enum ChartType {
    Transactions, // Transaction count over time
    ComputeUnits, // CU consumption over time
    SuccessRate,  // Success percentage over time
}

/// Display mode for detail view
#[derive(Clone, Copy, PartialEq)]
pub enum DetailViewMode {
    AllCharts,  // Show all 3 charts at once
    FullScreen, // Show single chart full size
}

/// Column the program table is sorted by (cycle with 's').
#[derive(Clone, Copy, PartialEq)]
pub enum SortColumn {
    TxPerSec,
    CuPerSec,
    AvgCu,
    Total,
    SuccessRate,
}

impl SortColumn {
    /// Cycle to the next sort column.
    pub fn next(self) -> Self {
        match self {
            SortColumn::TxPerSec => SortColumn::CuPerSec,
            SortColumn::CuPerSec => SortColumn::AvgCu,
            SortColumn::AvgCu => SortColumn::Total,
            SortColumn::Total => SortColumn::SuccessRate,
            SortColumn::SuccessRate => SortColumn::TxPerSec,
        }
    }

    /// Table header label this column sorts (for the active-sort indicator).
    pub fn header(self) -> &'static str {
        match self {
            SortColumn::TxPerSec => "Txs/s",
            SortColumn::CuPerSec => "CU/s",
            SortColumn::AvgCu => "Avg CU",
            SortColumn::Total => "Total",
            SortColumn::SuccessRate => "Success%",
        }
    }
}

/// Struct for displaying program stats in UI
pub struct ProgramStatsDisplay {
    pub program_id: String,
    pub tx_per_sec: f64,
    pub total_txs: u32,
    pub success_rate: f64,
    pub cu_per_sec: f64,
    pub avg_cu: f64,
    pub min_cu: u64,
    pub max_cu: u64,
}

/// Struct for displaying detailed program statistics
pub struct ProgramDetail {
    pub program_id: String,
    pub total_txs: u32,
    pub success_rate: f64,
    pub tx_per_sec: f64,
    pub cu_per_sec: f64,
    pub avg_cu: f64,
    pub min_cu: u64,
    pub max_cu: u64,
    pub slot_count: usize,
    pub first_seen: Option<Instant>,
    pub last_seen: Option<Instant>,
    pub slot_timeline: Vec<SlotStats>,
}

/// Struct for displaying network-wide aggregate statistics
pub struct NetworkStatsDisplay {
    pub current_slot: u64,
    pub latest_network_slot: u64,
    pub uptime: Duration,
    pub window_duration: Duration,
    pub program_count: usize,
    pub total_tps: f64,
    pub total_txs: u64,
    pub avg_success_rate: f64,
    pub total_cu_per_sec: f64,
}
