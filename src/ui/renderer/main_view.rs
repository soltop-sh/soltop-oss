use crate::ui::app::App;
use crate::ui::formatting::{format_cu, format_duration, format_large_number};
use crate::ui::types::ViewMode;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

/// Render the main view with table, header, and network overview
pub fn render_main_view(app: &mut App, frame: &mut Frame, area: Rect) {
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
    render_header(app, frame, chunks[0]);
    render_network_overview(app, frame, chunks[1]);
    render_table(app, frame, chunks[2]);
    render_footer(app, frame, chunks[3]);
}

/// Render the loading screen with logo
pub fn render_loading_screen(app: &App, frame: &mut Frame, area: Rect) {
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
        .style(app.theme.normal_style()) // White instead of green
        .alignment(Alignment::Center);
    frame.render_widget(logo_text, content_area);
}

/// Render the header section
fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let stats = &app.cached_network_stats;

    // Create header with neon green border
    let header_block = Block::default()
        .title(" soltop - Solana Table of Programs ")
        .borders(Borders::ALL)
        .border_style(app.theme.border_style())
        .title_style(app.theme.header_style());

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
    .style(app.theme.normal_style());
    frame.render_widget(slot_text, info_chunks[0]);

    // Line 2: Uptime, Window, Programs with mode indicators
    let mut status_parts = vec![
        format!("Uptime: {}", format_duration(stats.uptime)),
        format!("Window: {}", format_duration(stats.window_duration)),
        format!("Programs: {}", stats.program_count),
    ];

    // Add mode indicators
    let mut indicators = Vec::new();
    if app.truncate_ids {
        indicators.push("[TRUNCATED]");
    }
    if app.hide_system_programs {
        indicators.push("[FILTERED]");
    }
    if app.view_mode == ViewMode::Window {
        indicators.push("[WINDOW VIEW]");
    }

    if !indicators.is_empty() {
        status_parts.push(indicators.join(" "));
    }

    let stats_text = Paragraph::new(status_parts.join(" │ ")).style(app.theme.muted_style());
    frame.render_widget(stats_text, info_chunks[1]);
}

/// Render the network overview panel
fn render_network_overview(app: &App, frame: &mut Frame, area: Rect) {
    let stats = &app.cached_network_stats;

    let overview_block = Block::default()
        .title(" Network Overview ")
        .borders(Borders::ALL)
        .border_style(app.theme.border_style())
        .title_style(app.theme.header_style());

    let inner = overview_block.inner(area);
    frame.render_widget(overview_block, area);

    // Create spans with color-coded metrics
    let spans = vec![
        Span::styled("Total TPS: ", app.theme.muted_style()),
        Span::styled(
            format!("{:.1}", stats.total_tps),
            Style::default().fg(app.theme.tps_color(stats.total_tps)),
        ),
        Span::raw("  │  "),
        Span::styled("Total Txs: ", app.theme.muted_style()),
        Span::styled(
            format_large_number(stats.total_txs),
            app.theme.normal_style(),
        ),
        Span::raw("  │  "),
        Span::styled("Avg Success: ", app.theme.muted_style()),
        Span::styled(
            format!("{:.1}%", stats.avg_success_rate),
            Style::default().fg(app.theme.success_rate_color(stats.avg_success_rate)),
        ),
        Span::raw("  │  "),
        Span::styled("Total CU/s: ", app.theme.muted_style()),
        Span::styled(
            format_cu(stats.total_cu_per_sec),
            Style::default().fg(app.theme.cu_per_sec_color(stats.total_cu_per_sec)),
        ),
    ];

    let overview_text = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);

    frame.render_widget(overview_text, inner);
}

/// Render the statistics table
fn render_table(app: &mut App, frame: &mut Frame, area: Rect) {
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
    .style(app.theme.table_header_style())
    .height(1);

    // Convert cached stats to color-coded rows
    let rows: Vec<Row> = app
        .get_cached_stats()
        .iter()
        .enumerate()
        .map(|(index, stat)| {
            // Determine if this row is selected
            let is_selected = index == app.selected_row && !app.showing_detail;

            // Color code based on metrics
            let tps_color = app.theme.tps_color(stat.tx_per_sec);
            let success_color = app.theme.success_rate_color(stat.success_rate);
            let cu_per_sec_color = app.theme.cu_per_sec_color(stat.cu_per_sec);
            let avg_cu_color = app.theme.avg_cu_color(stat.avg_cu);

            // Handle ID display based on truncation setting
            let program_display = if app.truncate_ids {
                format!("{}...", &stat.program_id[..8.min(stat.program_id.len())])
            } else {
                stat.program_id.clone()
            };

            // Apply selection highlighting
            let row_style = if is_selected {
                Style::default()
                    .bg(app.theme.border) // Background highlight
                    .fg(app.theme.neon_green) // Text color
            } else {
                Style::default()
            };

            Row::new(vec![
                // Program ID (full or truncated based on toggle)
                Cell::from(program_display).style(Style::default().fg(app.theme.gray)),
                // TPS (color coded: green=low, amber=medium, red=high)
                Cell::from(format!("{:.1}", stat.tx_per_sec)).style(Style::default().fg(tps_color)),
                // CU/s (color coded based on compute intensity)
                Cell::from(format_cu(stat.cu_per_sec)).style(Style::default().fg(cu_per_sec_color)),
                // Avg CU (color coded based on efficiency)
                Cell::from(format_cu(stat.avg_cu)).style(Style::default().fg(avg_cu_color)),
                // Min CU
                Cell::from(format_cu(stat.min_cu as f64)).style(app.theme.normal_style()),
                // Max CU
                Cell::from(format_cu(stat.max_cu as f64)).style(app.theme.normal_style()),
                // Total (normal white)
                Cell::from(format!("{}", stat.total_txs)).style(app.theme.normal_style()),
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
            .border_style(app.theme.border_style())
            .title(" Program Statistics ")
            .title_style(app.theme.header_style()),
    );

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

/// Render the footer with keyboard shortcuts
fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    // htop-style keyboard shortcuts
    let footer_text: &[(&str, &str)] = if app.showing_detail {
        // Detail view shortcuts with chart info
        &[("TAB", "Switch Chart"), ("ESC", "Back")]
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
                Span::styled(*key, app.theme.success_style()), // Green key
                Span::raw(format!("{} ", label)),              // White label
                Span::raw(" "),
            ]
        })
        .collect();

    let footer = Paragraph::new(Line::from(spans)).style(Style::default().bg(app.theme.background));

    frame.render_widget(footer, area);
}
