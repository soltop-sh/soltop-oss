use crate::ui::app::App;
use crate::ui::formatting::{format_cu, format_duration, format_large_number};
use crate::ui::renderer::gauges::meter_line;
use crate::ui::types::{SortColumn, ViewMode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

/// Display scales for the aggregate gauges. These are soft caps used only to
/// size the bar fill — values above the cap simply render as full. Chosen to
/// match typical Solana mainnet ranges.
const TPS_FULL: f64 = 5_000.0;
const CU_FULL: f64 = 150_000_000.0; // 150M CU/s
const LAG_FULL: f64 = 10.0; // slots behind = "full"

/// Render the main view with table, header, meters, and footer
pub fn render_main_view(app: &mut App, frame: &mut Frame, area: Rect) {
    // Create main layout: header + meters + table + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Header (slot + status)
            Constraint::Length(4), // Meters: aggregate gauges (2×2)
            Constraint::Min(6),    // Table (takes remaining space)
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Render sections
    render_header(app, frame, chunks[0]);
    render_meters(app, frame, chunks[1]);
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
    let mut logo = vec![
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

    // Show RPC error below the logo if present
    if app.rpc_error.is_some() {
        logo.push("");
        logo.push("");
    }

    let logo_text = Paragraph::new(logo.join("\n"))
        .style(app.theme.normal_style())
        .alignment(Alignment::Center);
    frame.render_widget(logo_text, content_area);

    // Render error message below if present
    if let Some(ref err) = app.rpc_error {
        let err_area = ratatui::layout::Rect {
            x: content_area.x,
            y: content_area.y + content_area.height.saturating_sub(2),
            width: content_area.width,
            height: 2,
        };
        let err_text = Paragraph::new(err.as_str())
            .style(Style::default().fg(ratatui::style::Color::Red))
            .alignment(Alignment::Center);
        frame.render_widget(err_text, err_area);
    }
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

    // Line 2: Uptime, Window, Programs with mode indicators.
    // Built from spans so the [RPC ERROR] indicator can stand out in red while
    // the rest of the line stays muted.
    let base = format!(
        "Uptime: {} │ Window: {} │ Programs: {}",
        format_duration(stats.uptime),
        format_duration(stats.window_duration),
        stats.program_count
    );
    let mut spans = vec![Span::styled(base, app.theme.muted_style())];

    // RPC error is the most important indicator — show it first, in red+bold.
    if app.rpc_error.is_some() {
        spans.push(Span::styled(" │ ", app.theme.muted_style()));
        spans.push(Span::styled(
            "[RPC ERROR]",
            Style::default()
                .fg(ratatui::style::Color::Red)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
    }

    // Remaining mode indicators stay muted.
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
        spans.push(Span::styled(
            format!(" │ {}", indicators.join(" ")),
            app.theme.muted_style(),
        ));
    }

    let stats_text = Paragraph::new(Line::from(spans));
    frame.render_widget(stats_text, info_chunks[1]);
}

/// Render the meters region: aggregate network gauges (TPS, CU/s, Success, Lag)
/// laid out 2×2 as htop-style bars filling the panel width.
fn render_meters(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Network ")
        .borders(Borders::ALL)
        .border_style(app.theme.border_style())
        .title_style(app.theme.header_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 12 || inner.height == 0 {
        return;
    }

    let stats = &app.cached_network_stats;
    let lag = stats.latest_network_slot.saturating_sub(stats.current_slot);

    let label_w = 8usize;
    // Two side-by-side columns, two gauges each.
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);
    let bar_w = |w: u16| (w as usize).saturating_sub(label_w + 4).max(6);

    let left = vec![
        meter_line(
            "TPS",
            stats.total_tps / TPS_FULL,
            &format!("{:.0}", stats.total_tps),
            label_w,
            bar_w(cols[0].width),
            &app.theme,
        ),
        meter_line(
            "CU/s",
            stats.total_cu_per_sec / CU_FULL,
            &format_cu(stats.total_cu_per_sec),
            label_w,
            bar_w(cols[0].width),
            &app.theme,
        ),
    ];
    let right = vec![
        meter_line(
            "Success",
            stats.avg_success_rate / 100.0,
            &format!("{:.1}%", stats.avg_success_rate),
            label_w,
            bar_w(cols[1].width),
            &app.theme,
        ),
        meter_line(
            "Lag",
            lag as f64 / LAG_FULL,
            &format!("{lag} sl"),
            label_w,
            bar_w(cols[1].width),
            &app.theme,
        ),
    ];

    frame.render_widget(Paragraph::new(left), cols[0]);
    frame.render_widget(Paragraph::new(right), cols[1]);
}

/// Render the theme picker overlay (toggle with 'T'). Lists presets and
/// highlights the active one; ↑/↓ switch live, Enter/Esc close.
pub(super) fn render_theme_menu(app: &App, frame: &mut Frame, area: Rect) {
    use ratatui::widgets::Clear;

    let names: Vec<&'static str> = crate::ui::theme::Theme::presets()
        .iter()
        .map(|t| t.name)
        .collect();

    // Centered popup.
    let w: u16 = 28;
    let h: u16 = names.len() as u16 + 2;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect {
        x,
        y,
        width: w.min(area.width),
        height: h.min(area.height),
    };

    let block = Block::default()
        .title(" Theme (↑/↓, Enter) ")
        .borders(Borders::ALL)
        .border_style(app.theme.header_style())
        .title_style(app.theme.header_style());
    let inner = block.inner(popup);

    let lines: Vec<Line> = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let marker = if i == app.theme_index { "› " } else { "  " };
            let style = if i == app.theme_index {
                app.theme.selection_style()
            } else {
                app.theme.normal_style()
            };
            Line::from(Span::styled(format!("{marker}{name}"), style))
        })
        .collect();

    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Render the statistics table
fn render_table(app: &mut App, frame: &mut Frame, area: Rect) {
    // Table header with neon green. The active sort column gets a ▼ marker and
    // is rendered reversed so it stands out (htop-style).
    let head_cell = |label: &str, col: Option<SortColumn>| -> Cell {
        if col == Some(app.sort_column) {
            Cell::from(format!("{label} ▼")).style(
                Style::default()
                    .fg(app.theme.neon_green)
                    .add_modifier(Modifier::REVERSED | Modifier::BOLD),
            )
        } else {
            Cell::from(label.to_string())
        }
    };
    let header = Row::new(vec![
        head_cell("Program ID", None),
        head_cell("Txs/s", Some(SortColumn::TxPerSec)),
        head_cell("CU/s", Some(SortColumn::CuPerSec)),
        head_cell("Avg CU", Some(SortColumn::AvgCu)),
        head_cell("Min CU", None),
        head_cell("Max CU", None),
        head_cell("Total", Some(SortColumn::Total)),
        head_cell("Success%", Some(SortColumn::SuccessRate)),
    ])
    .style(app.theme.table_header_style())
    .height(1);

    // Convert cached stats to color-coded rows. Cells carry the theme bg so the
    // themed background fills the whole table; the selected row is highlighted
    // full-width by the table's row_highlight_style (htop-style), not per-cell.
    let bg = app.theme.background;
    let cell = |text: String, fg: ratatui::style::Color| {
        Cell::from(text).style(Style::default().fg(fg).bg(bg))
    };
    let rows: Vec<Row> = app
        .get_cached_stats()
        .iter()
        .map(|stat| {
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

            Row::new(vec![
                cell(program_display, app.theme.gray),
                cell(format!("{:.1}", stat.tx_per_sec), tps_color),
                cell(format_cu(stat.cu_per_sec), cu_per_sec_color),
                cell(format_cu(stat.avg_cu), avg_cu_color),
                cell(format_cu(stat.min_cu as f64), app.theme.white),
                cell(format_cu(stat.max_cu as f64), app.theme.white),
                cell(format!("{}", stat.total_txs), app.theme.white),
                cell(format!("{:.1}%", stat.success_rate), success_color),
            ])
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
    .row_highlight_style(app.theme.selection_style())
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
            ("ENTER", "Details"),
            ("s", "Sort"),
            ("T", "Theme"),
            ("t", "IDs"),
            ("u", "Filter"),
            ("w", "Window"),
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

#[cfg(test)]
mod scroll_tests {
    use super::*;
    use crate::stats::NetworkState;
    use crate::ui::app::App;
    use crate::ui::types::ProgramStatsDisplay;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::RwLock;

    fn app_with_n(n: usize, selected: usize) -> App {
        let ns = Arc::new(RwLock::new(NetworkState::new(
            Duration::from_secs(300),
            750,
        )));
        let mut app = App::new(ns);
        app.loading = false;
        app.cached_stats = (0..n)
            .map(|i| ProgramStatsDisplay {
                program_id: format!("P{i:02}"),
                tx_per_sec: 0.0,
                total_txs: i as u32,
                success_rate: 0.0,
                cu_per_sec: 0.0,
                avg_cu: 0.0,
                min_cu: 0,
                max_cu: 0,
            })
            .collect();
        app.selected_row = selected;
        app.table_state.select(Some(selected));
        app
    }

    fn render_to_string(app: &mut App, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_main_view(app, f, f.area()))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn last_row_visible_when_selected() {
        let mut app = app_with_n(30, 29);
        let content = render_to_string(&mut app, 80, 20);
        assert!(
            content.contains("P29"),
            "selected last row P29 should be scrolled into view; got:\n{content}"
        );
    }

    #[test]
    fn meters_show_aggregate_gauges() {
        let mut app = app_with_n(30, 0);
        app.cached_network_stats.total_tps = 1234.0;
        let content = render_to_string(&mut app, 110, 24);
        // All four aggregate gauges should be present in the Network panel.
        assert!(content.contains("TPS"), "TPS gauge missing");
        assert!(content.contains("CU/s"), "CU/s gauge missing");
        assert!(content.contains("Success"), "Success gauge missing");
        assert!(content.contains("Lag"), "Lag gauge missing");
    }

    #[test]
    fn theme_menu_lists_presets_when_open() {
        let mut app = app_with_n(5, 0);
        app.show_theme_menu = true;
        let content = render_to_string(&mut app, 110, 24);
        // render_to_string only draws main_view; exercise the overlay directly.
        let backend = TestBackend::new(110, 24);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            render_main_view(&mut app, f, f.area());
            render_theme_menu(&app, f, f.area());
        })
        .unwrap();
        let overlay: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(overlay.contains("Flatline"), "preset list missing");
        assert!(overlay.contains("Matrix"));
        let _ = content; // (kept to ensure main view renders with menu flag set)
    }

    #[test]
    fn active_sort_column_marked() {
        let mut app = app_with_n(10, 0);
        app.sort_column = SortColumn::CuPerSec;
        let content = render_to_string(&mut app, 110, 24);
        // the ▼ marker should sit next to the active column header
        assert!(
            content.contains("CU/s ▼"),
            "active sort marker missing; got:\n{content}"
        );
    }
}
