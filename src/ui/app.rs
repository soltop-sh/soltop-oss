use super::Theme;
use crate::stats::{is_system_program, NetworkState};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame, Terminal,
};
use std::cmp::Reverse;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use std::time::Instant;

/// View mode for displaying statistics
#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Live,   // Current behavior - shows recent activity
    Window, // Shows aggregate stats for entire window
}

/// Main TUI application
pub struct App {
    /// Reference to shared network state (updated by NetworkMonitor)
    network_state: Arc<RwLock<NetworkState>>,

    /// Whether the app should keep running
    pub running: bool,

    /// Currently selected row in the table
    pub selected_row: usize,

    /// Whether we're showing the detail view (true) or main table (false)
    showing_detail: bool,

    /// Program ID of the currently selected program (for detail view)
    selected_program_id: Option<String>,

    /// Cached program detail for the currently selected program
    cached_program_detail: Option<ProgramDetail>,

    cached_stats: Vec<ProgramStatsDisplay>,

    cached_network_stats: NetworkStatsDisplay,

    /// Theme configuration
    theme: Theme,

    /// Whether to truncate program IDs (toggle with 't')
    truncate_ids: bool,

    /// Whether to hide system programs (toggle with 'u')
    hide_system_programs: bool,

    /// Current view mode (toggle with 'w')
    view_mode: ViewMode,

    /// Loading state - true until first data arrives
    loading: bool,
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
    fn get_cached_stats(&self) -> &[ProgramStatsDisplay] {
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
                    self.handle_key(key.code);
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
        let area = frame.area();

        // Show loading screen if no data yet
        if self.loading {
            self.render_loading_screen(frame, area);
            return;
        }

        // Show detail view if active, otherwise show main view
        if self.showing_detail {
            self.render_detail_view(frame, area);
        } else {
            // Create main layout: header + network overview + table + footer
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5), // Header (normal size)
                    Constraint::Length(3), // Network Overview
                    Constraint::Min(10),   // Table (takes remaining space)
                    Constraint::Length(1), // Footer
                ])
                .split(area);

            // Render sections
            self.render_header(frame, chunks[0]);
            self.render_network_overview(frame, chunks[1]);
            self.render_table(frame, chunks[2]);
            self.render_footer(frame, chunks[3]);
        }
    }

    /// Render the loading screen with logo
    fn render_loading_screen(&self, frame: &mut Frame, area: Rect) {
        // Create centered layout
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30), // Top spacer
                Constraint::Length(15),     // Logo + text
                Constraint::Percentage(30), // Bottom spacer
            ])
            .split(area);

        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Left spacer
                Constraint::Percentage(50), // Logo area
                Constraint::Percentage(25), // Right spacer
            ])
            .split(vertical_chunks[1]);

        let content_area = horizontal_chunks[1];

        // ASCII logo
        let logo = vec![
            "                  ████   █████                      ",
            "                 ░░███  ░░███                       ",
            "  █████   ██████  ░███  ███████    ██████  ████████ ",
            " ███░░   ███░░███ ░███ ░░░███░    ███░░███░░███░░███",
            "░░█████ ░███ ░███ ░███   ░███    ░███ ░███ ░███ ░███",
            " ░░░░███░███ ░███ ░███   ░███ ███░███ ░███ ░███ ░███",
            " ██████ ░░██████  █████  ░░█████ ░░██████  ░███████ ",
            "░░░░░░   ░░░░░░  ░░░░░    ░░░░░   ░░░░░░   ░███░░░  ",
            "                                           ░███     ",
            "                                           █████    ",
            "                                          ░░░░░     ",
            "",
            "              Loading Solana network data...",
        ];

        let logo_text = Paragraph::new(logo.join("\n"))
            .style(self.theme.normal_style()) // White instead of green
            .alignment(Alignment::Center);
        frame.render_widget(logo_text, content_area);
    }

    /// Render the header section
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let stats = &self.cached_network_stats;

        // Create header with neon green border
        let header_block = Block::default()
            .title(" soltop - Solana Table of Programs ")
            .borders(Borders::ALL)
            .border_style(self.theme.border_style())
            .title_style(self.theme.header_style());

        let inner = header_block.inner(area);
        frame.render_widget(header_block, area);

        // Split inner area into 3 lines (no logo in header anymore)
        let info_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Line 1: Slot
                Constraint::Length(1), // Line 2: Stats with mode indicators
                Constraint::Length(1), // Line 3: Spacer
            ])
            .split(inner);

        // Line 1: Current slot with network lag
        let lag = stats.latest_network_slot.saturating_sub(stats.current_slot);

        let slot_text = Paragraph::new(format!(
            "Slot: {} │ Network: {} ({} behind)",
            format_large_number(stats.current_slot),
            format_large_number(stats.latest_network_slot),
            lag
        ))
        .style(self.theme.normal_style());
        frame.render_widget(slot_text, info_chunks[0]);

        // Line 2: Uptime, Window, Programs with mode indicators
        let mut status_parts = vec![
            format!("Uptime: {}", format_duration(stats.uptime)),
            format!("Window: {}", format_duration(stats.window_duration)),
            format!("Programs: {}", stats.program_count),
        ];

        // Add mode indicators
        let mut indicators = Vec::new();
        if self.truncate_ids {
            indicators.push("[TRUNCATED]");
        }
        if self.hide_system_programs {
            indicators.push("[FILTERED]");
        }
        if self.view_mode == ViewMode::Window {
            indicators.push("[WINDOW VIEW]");
        }

        if !indicators.is_empty() {
            status_parts.push(indicators.join(" "));
        }

        let stats_text = Paragraph::new(status_parts.join(" │ ")).style(self.theme.muted_style());
        frame.render_widget(stats_text, info_chunks[1]);
    }

    /// Render the network overview panel
    fn render_network_overview(&self, frame: &mut Frame, area: Rect) {
        let stats = &self.cached_network_stats;

        let overview_block = Block::default()
            .title(" Network Overview ")
            .borders(Borders::ALL)
            .border_style(self.theme.border_style())
            .title_style(self.theme.header_style());

        let inner = overview_block.inner(area);
        frame.render_widget(overview_block, area);

        // Create spans with color-coded metrics
        let spans = vec![
            Span::styled("Total TPS: ", self.theme.muted_style()),
            Span::styled(
                format!("{:.1}", stats.total_tps),
                Style::default().fg(self.theme.tps_color(stats.total_tps)),
            ),
            Span::raw("  │  "),
            Span::styled("Total Txs: ", self.theme.muted_style()),
            Span::styled(
                format_large_number(stats.total_txs),
                self.theme.normal_style(),
            ),
            Span::raw("  │  "),
            Span::styled("Avg Success: ", self.theme.muted_style()),
            Span::styled(
                format!("{:.1}%", stats.avg_success_rate),
                Style::default().fg(self.theme.success_rate_color(stats.avg_success_rate)),
            ),
            Span::raw("  │  "),
            Span::styled("Total CU/s: ", self.theme.muted_style()),
            Span::styled(
                format_cu(stats.total_cu_per_sec),
                Style::default().fg(self.theme.cu_per_sec_color(stats.total_cu_per_sec)),
            ),
        ];

        let overview_text = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);

        frame.render_widget(overview_text, inner);
    }

    /// Render the statistics table
    fn render_table(&self, frame: &mut Frame, area: Rect) {
        // Table header with neon green
        let header = Row::new(vec![
            Cell::from("Program ID"),
            Cell::from("Txs/s"),
            Cell::from("CU/s"),
            Cell::from("Avg CU"),
            Cell::from("Min CU"),
            Cell::from("Max CU"),
            Cell::from("Total"),
            Cell::from("Success%"),
        ])
        .style(self.theme.table_header_style())
        .height(1);

        // Convert cached stats to color-coded rows
        let rows: Vec<Row> = self
            .get_cached_stats()
            .iter()
            .enumerate()
            .map(|(index, stat)| {
                // Determine if this row is selected
                let is_selected = index == self.selected_row && !self.showing_detail;

                // Color code based on metrics
                let tps_color = self.theme.tps_color(stat.tx_per_sec);
                let success_color = self.theme.success_rate_color(stat.success_rate);
                let cu_per_sec_color = self.theme.cu_per_sec_color(stat.cu_per_sec);
                let avg_cu_color = self.theme.avg_cu_color(stat.avg_cu);

                // Handle ID display based on truncation setting
                let program_display = if self.truncate_ids {
                    format!("{}...", &stat.program_id[..8.min(stat.program_id.len())])
                } else {
                    stat.program_id.clone()
                };

                // Apply selection highlighting
                let row_style = if is_selected {
                    Style::default()
                        .bg(self.theme.border)  // Background highlight
                        .fg(self.theme.neon_green)  // Text color
                } else {
                    Style::default()
                };

                Row::new(vec![
                    // Program ID (full or truncated based on toggle)
                    Cell::from(program_display).style(Style::default().fg(self.theme.gray)),
                    // TPS (color coded: green=low, amber=medium, red=high)
                    Cell::from(format!("{:.1}", stat.tx_per_sec))
                        .style(Style::default().fg(tps_color)),
                    // CU/s (color coded based on compute intensity)
                    Cell::from(format_cu(stat.cu_per_sec))
                        .style(Style::default().fg(cu_per_sec_color)),
                    // Avg CU (color coded based on efficiency)
                    Cell::from(format_cu(stat.avg_cu)).style(Style::default().fg(avg_cu_color)),
                    // Min CU
                    Cell::from(format_cu(stat.min_cu as f64)).style(self.theme.normal_style()),
                    // Max CU
                    Cell::from(format_cu(stat.max_cu as f64)).style(self.theme.normal_style()),
                    // Total (normal white)
                    Cell::from(format!("{}", stat.total_txs)).style(self.theme.normal_style()),
                    // Success% (color coded: green>95%, amber>80%, red<80%)
                    Cell::from(format!("{:.1}%", stat.success_rate))
                        .style(Style::default().fg(success_color)),
                ])
                .style(row_style)
            })
            .collect();

        // Table with border matching theme - adjusted column widths for full IDs
        let table = Table::new(
            rows,
            vec![
                Constraint::Percentage(30), // Program ID
                Constraint::Percentage(8),  // Txs/s
                Constraint::Percentage(9),  // CU/s
                Constraint::Percentage(9),  // Avg CU
                Constraint::Percentage(9),  // Min CU
                Constraint::Percentage(9),  // Max CU
                Constraint::Percentage(8),  // Total
                Constraint::Percentage(8),  // Success%
                Constraint::Percentage(10), // Padding
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.border_style())
                .title(" Program Statistics ")
                .title_style(self.theme.header_style()),
        );

        frame.render_widget(table, area);
    }
    
    fn render_detail_view(&self, frame: &mut Frame, area: Rect) {
        let detail = match &self.cached_program_detail {
            Some(d) => d,
            None => {
                // Show loading or error message
                let error_text = Paragraph::new("Loading program details...")
                    .style(self.theme.muted_style())
                    .alignment(Alignment::Center);
                frame.render_widget(error_text, area);
                return;
            }
        };
        
        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(15),    // Stats content
                Constraint::Length(1),  // Footer
            ])
            .split(area);
        
        // Header
        let header_block = Block::default()
            .title(format!(" Program Details: {} ", detail.program_id))
            .borders(Borders::ALL)
            .border_style(self.theme.border_style())
            .title_style(self.theme.header_style());
        frame.render_widget(header_block, chunks[0]);
        
        // Content area with statistics
        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Main stats
                Constraint::Min(5),     // Historical data (if available)
            ])
            .split(chunks[1]);
        
        // Main statistics block
        let stats_block = Block::default()
            .title(" Statistics ")
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());
        
        let stats_inner = stats_block.inner(content_chunks[0]);
        frame.render_widget(stats_block, content_chunks[0]);
        
        // Format statistics text
        let mut stats_text = vec![
            format!("Total Transactions: {}", format_large_number(detail.total_txs as u64)),
            format!("Transactions/sec:   {:.2}", detail.tx_per_sec),
            format!("Success Rate:       {:.2}%", detail.success_rate),
            format!("Compute Units/sec:  {}", format_cu(detail.cu_per_sec)),
            format!("Avg CU/tx:          {}", format_cu(detail.avg_cu)),
            format!("Min CU:             {}", format_cu(detail.min_cu as f64)),
            format!("Max CU:             {}", format_cu(detail.max_cu as f64)),
            format!("Active Slots:       {}", detail.slot_count),
        ];
        
        // Add first seen / last seen if available
        if let Some(first_seen) = detail.first_seen {
            let elapsed = first_seen.elapsed();
            stats_text.push(format!("First Seen:         {} ago", format_duration(elapsed)));
        }
        if let Some(last_seen) = detail.last_seen {
            let elapsed = last_seen.elapsed();
            stats_text.push(format!("Last Activity:      {} ago", format_duration(elapsed)));
        }
        
        let stats_paragraph = Paragraph::new(stats_text.join("\n"))
            .style(self.theme.normal_style());
        frame.render_widget(stats_paragraph, stats_inner);
        
        // Footer
        let footer_text = Paragraph::new("Press ESC to return")
            .style(self.theme.muted_style())
            .alignment(Alignment::Center);
        frame.render_widget(footer_text, chunks[2]);
    }

    /// Render the footer with keyboard shortcuts
    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        // htop-style keyboard shortcuts
        let footer_text: &[(&str, &str)] = if self.showing_detail {
            // Detail view shortcuts
            &[
                ("ESC", "Back"),
            ]
        } else {
            // Main view shortcuts
            &[
                ("↑/↓", "Navigate"),
                ("ENTER", "View Details"),
                ("t", "Toggle IDs"),
                ("u", "Filter System"),
                ("w", "Window View"),
                ("q", "Quit"),
            ]
        };

        let spans: Vec<Span> = footer_text
            .iter()
            .flat_map(|(key, label)| {
                vec![
                    Span::styled(*key, self.theme.success_style()), // Green key
                    Span::raw(format!("{} ", label)),               // White label
                    Span::raw(" "),
                ]
            })
            .collect();

        let footer =
            Paragraph::new(Line::from(spans)).style(Style::default().bg(self.theme.background));

        frame.render_widget(footer, area);
    }

    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                if self.showing_detail {
                    // Return to main view from detail view
                    self.showing_detail = false;
                    self.selected_program_id = None;
                } else {
                    // Quit from main view
                    self.running = false;
                }
            }
            KeyCode::Char('q') | KeyCode::F(10) => {
                self.running = false;
            }
            KeyCode::Char('t') => {
                // Toggle ID truncation
                self.truncate_ids = !self.truncate_ids;
            }
            KeyCode::Char('u') => {
                // Toggle system program filter
                self.hide_system_programs = !self.hide_system_programs;
            }
            KeyCode::Char('w') => {
                // Toggle view mode
                self.view_mode = match self.view_mode {
                    ViewMode::Live => ViewMode::Window,
                    ViewMode::Window => ViewMode::Live,
                };
            }
            KeyCode::Down => {
                if !self.showing_detail {
                    let max_row = self.cached_stats.len().saturating_sub(1);
                    self.selected_row = (self.selected_row + 1).min(max_row);
                }
            }
            KeyCode::Up => {
                if !self.showing_detail {
                    self.selected_row = self.selected_row.saturating_sub(1);
                }
            }

            KeyCode::Enter | KeyCode::Char(' ') => {
                if !self.showing_detail {
                    if let Some(stat) = self.cached_stats.get(self.selected_row) {
                        self.selected_program_id = Some(stat.program_id.clone());
                        self.showing_detail = true;
                    }
                }
            }
            _ => {}
        }
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
        })
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

// ============================================================================
// Number Formatting Helpers
// ============================================================================

/// Format large numbers with comma separators (e.g., "1,234,567")
fn format_large_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

/// Format large numbers with K/M/B suffixes (e.g., "2.3M", "450.2K")
fn format_cu(n: f64) -> String {
    if n >= 1_000_000_000.0 {
        format!("{:.1}B", n / 1_000_000_000.0)
    } else if n >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("{:.1}K", n / 1_000.0)
    } else {
        format!("{:.0}", n)
    }
}

/// Format duration in human-readable form (e.g., "2m 34s", "1h 23m")
fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();

    if total_secs >= 3600 {
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    } else if total_secs >= 60 {
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", total_secs)
    }
}
