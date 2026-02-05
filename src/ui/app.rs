use super::Theme;
use crate::stats::{is_system_program, NetworkState, SlotStats};
use crate::ui::input;
use crate::ui::renderer;
use crate::ui::types::{
    ChartType, NetworkStatsDisplay, ProgramDetail, ProgramStatsDisplay, ViewMode,
};
use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::{backend::Backend, Frame, Terminal};
use std::cmp::Reverse;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Main TUI application
pub struct App {
    /// Reference to shared network state (updated by NetworkMonitor)
    network_state: Arc<RwLock<NetworkState>>,

    /// Whether the app should keep running
    pub running: bool,

    /// Currently selected row in the table
    pub selected_row: usize,

    /// Whether we're showing the detail view (true) or main table (false)
    pub showing_detail: bool,

    /// Program ID of the currently selected program (for detail view)
    pub selected_program_id: Option<String>,

    /// Cached program detail for the currently selected program
    pub cached_program_detail: Option<ProgramDetail>,

    pub cached_stats: Vec<ProgramStatsDisplay>,

    pub cached_network_stats: NetworkStatsDisplay,

    /// Theme configuration
    pub theme: Theme,

    /// Whether to truncate program IDs (toggle with 't')
    pub truncate_ids: bool,

    /// Whether to hide system programs (toggle with 'u')
    pub hide_system_programs: bool,

    /// Current view mode (toggle with 'w')
    pub view_mode: ViewMode,

    /// Loading state - true until first data arrives
    pub loading: bool,

    /// Current chart type being displayed
    pub current_chart: ChartType,
}

impl App {
    /// Create a new App with reference to network state
    pub fn new(network_state: Arc<RwLock<NetworkState>>) -> Self {
        Self {
            network_state,
            running: true,
            selected_row: 0,
            showing_detail: false,
            selected_program_id: None,
            cached_program_detail: None,
            cached_stats: vec![],
            cached_network_stats: NetworkStatsDisplay {
                current_slot: 0,
                latest_network_slot: 0,
                uptime: Duration::from_secs(0),
                window_duration: Duration::from_secs(0),
                program_count: 0,
                total_tps: 0.0,
                total_txs: 0,
                avg_success_rate: 0.0,
                total_cu_per_sec: 0.0,
            },
            theme: Theme::flatline(),
            truncate_ids: false,
            hide_system_programs: false,
            view_mode: ViewMode::Live,
            loading: true,
            current_chart: ChartType::Transactions,
        }
    }

    /// Update cached stats from network state
    async fn update_stats(&mut self) {
        let (program_stats, network_stats) = self.get_stats().await;
        self.cached_stats = program_stats;
        self.cached_network_stats = network_stats;

        // Update detail view if showing
        if self.showing_detail {
            if let Some(program_id) = &self.selected_program_id {
                self.cached_program_detail = self.get_program_detail(program_id).await;
            }
        }

        // Exit loading state once we have data
        if self.cached_network_stats.current_slot > 0 {
            self.loading = false;
        }
    }

    /// Get cached stats for rendering
    pub fn get_cached_stats(&self) -> &[ProgramStatsDisplay] {
        &self.cached_stats
    }

    /// Run the main event loop
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Tick rate: how often we update the UI
        let tick_rate = Duration::from_millis(500);
        let mut last_tick = tokio::time::Instant::now();

        loop {
            self.update_stats().await;

            // 1. Draw UI
            terminal.draw(|frame| {
                self.render(frame);
            })?;

            // 2. Handle events with timeout
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    // Handle keyboard input
                    input::handle_key(self, key.code);
                }
            }

            // 3. Tick if enough time has elapsed
            if last_tick.elapsed() >= tick_rate {
                // This is where we'd update app state
                // (Currently no-op since data updates happen in background)
                last_tick = tokio::time::Instant::now();
            }

            // 4. Check exit condition
            if !self.running {
                break;
            }
        }

        Ok(())
    }

    /// Render the entire UI
    pub fn render(&self, frame: &mut Frame) {
        renderer::render(self, frame);
    }

    /// Get current network statistics
    async fn get_stats(&self) -> (Vec<ProgramStatsDisplay>, NetworkStatsDisplay) {
        let state = self.network_state.read().await;

        let mut display = Vec::new();

        // Note: ViewMode (Live vs Window) both read from the same ring buffer
        // The difference is conceptual - Live shows "streaming" while Window shows "accumulated"
        // Both calculate from the configured time window stored in the ring buffer
        // Future enhancement: could adjust time ranges or aggregation methods per mode

        // Aggregate network-wide statistics
        let mut total_tps = 0.0;
        let mut total_txs = 0u64;
        let mut total_success_txs = 0u64;
        let mut total_cu_per_sec = 0.0;

        for (program_id, stats) in state.programs.iter() {
            // Skip system programs if filter is enabled
            if self.hide_system_programs && is_system_program(program_id) {
                continue;
            }

            let tx_per_sec = stats.transactions_per_second();
            let total_program_txs = stats.total_transactions();
            let success_rate = stats.success_rate();
            let cu_per_sec = stats.cu_per_second();
            let avg_cu = stats.avg_cu_per_transaction();
            let min_cu = stats.min_cu();
            let max_cu = stats.max_cu();

            // Accumulate network totals
            total_tps += tx_per_sec;
            total_txs += total_program_txs as u64;
            total_success_txs += ((success_rate / 100.0) * total_program_txs as f64) as u64;
            total_cu_per_sec += cu_per_sec;

            display.push(ProgramStatsDisplay {
                program_id: program_id.clone(),
                tx_per_sec,
                total_txs: total_program_txs,
                success_rate,
                cu_per_sec,
                avg_cu,
                min_cu,
                max_cu,
            });
        }

        // Sort by total_txs descending
        display.sort_by_key(|s| Reverse(s.total_txs));

        // Calculate average success rate (weighted)
        let avg_success_rate = if total_txs > 0 {
            (total_success_txs as f64 / total_txs as f64) * 100.0
        } else {
            0.0
        };

        let network_stats = NetworkStatsDisplay {
            current_slot: state.current_slot,
            latest_network_slot: state.latest_network_slot,
            uptime: state.uptime(),
            window_duration: state.actual_window(),
            program_count: state.program_count(),
            total_tps,
            total_txs,
            avg_success_rate,
            total_cu_per_sec,
        };

        (display, network_stats)
    }

    /// Get the program stats for the selected program
    async fn get_program_detail(&self, program_id: &str) -> Option<ProgramDetail> {
        let state = self.network_state.read().await;

        // Find the program in the network state
        let program_stats = state.programs.get(program_id)?;

        // Calculate detailed metrics
        let total_txs = program_stats.total_transactions();
        let success_rate = program_stats.success_rate();
        let tx_per_sec = program_stats.transactions_per_second();
        let cu_per_sec = program_stats.cu_per_second();
        let avg_cu = program_stats.avg_cu_per_transaction();
        let min_cu = program_stats.min_cu();
        let max_cu = program_stats.max_cu();

        // Get slot timeline data using the new methods
        let slot_count = program_stats.slot_count();
        let first_seen = program_stats.first_slot_timestamp();
        let last_seen = program_stats.last_slot_timestamp();

        // Clone slot timeline data for rendering
        let slot_timeline: Vec<SlotStats> = program_stats
            .get_slot_timeline()
            .into_iter()
            .cloned() // Clone each SlotStats
            .collect();

        Some(ProgramDetail {
            program_id: program_id.to_string(),
            total_txs,
            success_rate,
            tx_per_sec,
            cu_per_sec,
            avg_cu,
            min_cu,
            max_cu,
            slot_count,
            first_seen,
            last_seen,
            slot_timeline,
        })
    }
}
