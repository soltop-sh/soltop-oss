use super::charts;
use crate::ui::app::App;
use crate::ui::formatting::{format_cu, format_duration, format_large_number};
use crate::ui::types::{ChartType, DetailViewMode, ProgramDetail};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the detail view with program statistics and charts
pub fn render_detail_view(app: &App, frame: &mut Frame, area: Rect) {
    let detail = match &app.cached_program_detail {
        Some(d) => d,
        None => {
            // Show loading message
            let error_text = Paragraph::new("Loading program details...")
                .style(app.theme.muted_style())
                .alignment(Alignment::Center);
            frame.render_widget(error_text, area);
            return;
        }
    };

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(10), // Stats summary
            Constraint::Min(12),    // Chart area
            Constraint::Length(1),  // Footer
        ])
        .split(area);

    // Header
    let header_block = Block::default()
        .title(format!(" Program Details: {} ", detail.program_id))
        .borders(Borders::ALL)
        .border_style(app.theme.border_style())
        .title_style(app.theme.header_style());
    frame.render_widget(header_block, chunks[0]);

    // Stats summary
    render_detail_stats(app, frame, chunks[1], detail);

    // Chart
    render_detail_chart(app, frame, chunks[2]);

    // Footer - update based on mode
    let footer_text = match app.detail_view_mode {
        DetailViewMode::AllCharts => "Press TAB for full-screen | ESC to return",
        DetailViewMode::FullScreen => "Press TAB to cycle/exit | ESC to return",
    };
    let footer = Paragraph::new(footer_text)
        .style(app.theme.muted_style())
        .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[3]);
}

/// Render the statistics summary panel
fn render_detail_stats(app: &App, frame: &mut Frame, area: Rect, detail: &ProgramDetail) {
    let stats_block = Block::default()
        .title(" Statistics ")
        .borders(Borders::ALL)
        .border_style(app.theme.border_style());

    let stats_inner = stats_block.inner(area);
    frame.render_widget(stats_block, area);

    // Format statistics text
    let mut stats_text = vec![
        format!(
            "Total Transactions: {}  │  Transactions/sec: {:.2}",
            format_large_number(detail.total_txs as u64),
            detail.tx_per_sec
        ),
        format!(
            "Success Rate: {:.2}%  │  Compute Units/sec: {}",
            detail.success_rate,
            format_cu(detail.cu_per_sec)
        ),
        format!(
            "Avg CU/tx: {}  │  Min CU: {}  │  Max CU: {}",
            format_cu(detail.avg_cu),
            format_cu(detail.min_cu as f64),
            format_cu(detail.max_cu as f64)
        ),
        format!("Active Slots: {}", detail.slot_count),
    ];

    // Add first seen / last seen if available
    if let Some(first_seen) = detail.first_seen {
        let elapsed = first_seen.elapsed();
        stats_text.push(format!("First Seen: {} ago", format_duration(elapsed)));
    }
    if let Some(last_seen) = detail.last_seen {
        let elapsed = last_seen.elapsed();
        stats_text.push(format!("Last Activity: {} ago", format_duration(elapsed)));
    }

    let stats_paragraph = Paragraph::new(stats_text.join("\n")).style(app.theme.normal_style());
    frame.render_widget(stats_paragraph, stats_inner);
}

/// Render charts based on detail view mode
fn render_detail_chart(app: &App, frame: &mut Frame, area: Rect) {
    match app.detail_view_mode {
        DetailViewMode::AllCharts => render_all_charts(app, frame, area),
        DetailViewMode::FullScreen => render_single_chart(app, frame, area),
    }
}

/// Render all three charts in a grid layout
fn render_all_charts(app: &App, frame: &mut Frame, area: Rect) {
    let detail = match &app.cached_program_detail {
        Some(d) => d,
        None => return,
    };

    if detail.slot_timeline.is_empty() {
        let placeholder = Paragraph::new("No timeline data available")
            .style(app.theme.muted_style())
            .alignment(Alignment::Center);
        frame.render_widget(placeholder, area);
        return;
    }

    // Split into two rows: top full width, bottom split
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Top row: TX chart
            Constraint::Percentage(50), // Bottom row: CU + Success
        ])
        .split(area);

    // Top row: Transaction chart (full width)
    charts::render_tx_chart(app, frame, rows[0], detail);

    // Bottom row: Split into two columns
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Left: CU chart
            Constraint::Percentage(50), // Right: Success chart
        ])
        .split(rows[1]);

    charts::render_cu_chart(app, frame, columns[0], detail);
    charts::render_success_chart(app, frame, columns[1], detail);
}

/// Render a single chart in full-screen mode
fn render_single_chart(app: &App, frame: &mut Frame, area: Rect) {
    let detail = match &app.cached_program_detail {
        Some(d) => d,
        None => return,
    };

    if detail.slot_timeline.is_empty() {
        let placeholder = Paragraph::new("No timeline data available")
            .style(app.theme.muted_style())
            .alignment(Alignment::Center);
        frame.render_widget(placeholder, area);
        return;
    }

    // Render the currently selected chart
    match app.current_chart {
        ChartType::Transactions => charts::render_tx_chart(app, frame, area, detail),
        ChartType::ComputeUnits => charts::render_cu_chart(app, frame, area, detail),
        ChartType::SuccessRate => charts::render_success_chart(app, frame, area, detail),
    }
}
