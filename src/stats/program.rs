use super::RingBuffer;
use std::time::Instant;

/// Statistics for a single Solana program
pub struct ProgramStats {
    /// The program's public key (e.g., "JUP4Fb2c...")
    pub program_id: String,

    /// Ring buffer of slot-level statistics
    /// Each entry = aggregated stats for ONE SLOT
    slot_timeline: RingBuffer<SlotStats>,
}

/// Statistics for a single slot
#[derive(Debug, Clone)]
pub struct SlotStats {
    /// When this slot was processed
    pub timestamp: Instant,

    /// Total compute units consumed in this slot
    pub total_cu: u64,

    /// Number of transactions in this slot
    pub tx_count: u32,

    /// Number of successful transactions
    pub success_count: u32,

    /// Average CU per transaction (precomputed)
    pub avg_cu: f64,

    /// Minimum CU in this slot
    pub min_cu: u64,

    /// Maximum CU in this slot
    pub max_cu: u64,
}

impl ProgramStats {
    /// Create new program stats tracker
    pub fn new(program_id: String, capacity: usize) -> Self {
        Self {
            program_id,
            slot_timeline: RingBuffer::new(capacity),
        }
    }

    /// Record statistics for a slot
    pub fn record_slot(&mut self, slot_stats: SlotStats) {
        self.slot_timeline.push(slot_stats);
    }

    /// Get total transaction count across all slots in buffer
    pub fn total_transactions(&self) -> u32 {
        self.slot_timeline.iter().map(|s| s.tx_count).sum()
    }

    /// Calculate success rate (0.0 to 100.0)
    pub fn success_rate(&self) -> f64 {
        let success_txs: u32 = self.slot_timeline.iter().map(|s| s.success_count).sum();
        let all_txs = self.total_transactions();

        if all_txs == 0 {
            100.0
        } else {
            (success_txs as f64 / all_txs as f64) * 100.0
        }
    }

    /// Calculate transactions per second
    pub fn transactions_per_second(&self) -> f64 {
        if self.slot_timeline.is_empty() {
            0.0
        } else {
            let time_span = self.get_time_span();
            let total_txs = self.total_transactions();

            total_txs as f64 / time_span
        }
    }

    /// Calculate compute units per second
    pub fn cu_per_second(&self) -> f64 {
        if self.slot_timeline.is_empty() {
            0.0
        } else {
            let time_span = self.get_time_span();
            let total_cu: u64 = self.slot_timeline.iter().map(|s| s.total_cu).sum();

            total_cu as f64 / time_span
        }
    }

    /// Calculate average CU per transaction across all slots
    pub fn avg_cu_per_transaction(&self) -> f64 {
        if self.slot_timeline.is_empty() {
            0.0
        } else {
            let total_cu: u64 = self.slot_timeline.iter().map(|s| s.total_cu).sum();
            let total_txs = self.total_transactions();

            total_cu as f64 / total_txs as f64
        }
    }

    /// Get minimum CU from all slots
    pub fn min_cu(&self) -> u64 {
        self.slot_timeline
            .iter()
            .map(|s| s.min_cu)
            .min()
            .unwrap_or(0)
    }

    /// Get maximum CU from all slots
    pub fn max_cu(&self) -> u64 {
        self.slot_timeline
            .iter()
            .map(|s| s.max_cu)
            .max()
            .unwrap_or(0)
    }

    // Calculate time passed between oldest and newest timestamps
    fn get_time_span(&self) -> f64 {
        if self.slot_timeline.is_empty() {
            return 1.0; // Default to 1 second to avoid division by zero
        }

        if self.slot_timeline.len() == 1 {
            return 1.0; // Single slot, use 1 second minimum
        }

        // Get first and last elements without consuming iterator
        let slots: Vec<_> = self.slot_timeline.iter().collect();
        let first_slot = slots.first().unwrap();
        let last_slot = slots.last().unwrap();

        let duration = last_slot.timestamp.duration_since(first_slot.timestamp);
        let time_span = duration.as_secs_f64();

        // Use minimum of 1 second to avoid infinity/huge numbers at startup
        time_span.max(1.0)
    }

    /// Get a reference to the slot timeline (for detail views)
    /// Returns a vector of slot stats for display
    pub fn get_slot_timeline(&self) -> Vec<&SlotStats> {
        self.slot_timeline.iter().collect()
    }
    
    /// Get the number of slots with activity
    pub fn slot_count(&self) -> usize {
        self.slot_timeline.len()
    }
    
    /// Get the timestamp of the first slot (oldest data)
    pub fn first_slot_timestamp(&self) -> Option<Instant> {
        self.slot_timeline.iter().next().map(|s| s.timestamp)
    }
    
    /// Get the timestamp of the last slot (newest data)
    pub fn last_slot_timestamp(&self) -> Option<Instant> {
        self.slot_timeline.iter().last().map(|s| s.timestamp)
    }
}
