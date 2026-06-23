use super::charts;
use super::gauges::meter_line;
use crate::ui::app::App;
use crate::ui::formatting::{format_cu, format_duration, format_large_number};
use crate::ui::types::{ChartType, DetailViewMode, ProgramDetail};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Full-width inverted label bar used as a section/chart header.
fn label_bar(app: &App, text: &str) -> Paragraph<'static> {
    Paragraph::new(Line::from(Span::raw(format!(" {text}")))).style(app.theme.inverted_style())
}

/// Render the detail view (borderless, htop-style) with program stats + charts.
pub fn render_detail_view(app: &App, frame: &mut Frame, area: Rect) {
    let detail = match &app.cached_program_detail {
        Some(d) => d,
        None => {
            let loading = Paragraph::new("Loading program details...")
                .style(app.theme.muted_style())
                .alignment(Alignment::Center);
            frame.render_widget(loading, area);
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title bar
            Constraint::Length(1), // Success gauge
            Constraint::Length(2), // Key/value stats
            Constraint::Length(1), // Spacer
            Constraint::Min(8),    // Charts
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Title bar
    frame.render_widget(
        label_bar(app, &format!("Program · {}", detail.program_id)),
        chunks[0],
    );

    // Success gauge (reuses the main-page meter component).
    let bar_w = (chunks[1].width as usize).saturating_sub(8 + 4).max(6);
    frame.render_widget(
        Paragraph::new(meter_line(
            "Success",
            detail.success_rate / 100.0,
            &format!("{:.1}%", detail.success_rate),
            8,
            bar_w,
            &app.theme,
        )),
        chunks[1],
    );

    render_detail_stats(app, frame, chunks[2], detail);
    render_detail_chart(app, frame, chunks[4]);

    // Footer — htop-style chips.
    let footer_keys: &[(&str, &str)] = &[("TAB", "Cycle/Fullscreen"), ("ESC", "Back")];
    let key_style = Style::default()
        .fg(app.theme.neon_green)
        .bg(app.theme.background);
    let spans: Vec<Span> = footer_keys
        .iter()
        .flat_map(|(k, l)| {
            vec![
                Span::styled(*k, key_style),
                Span::styled(*l, app.theme.inverted_style()),
                Span::styled(" ", Style::default().bg(app.theme.background)),
            ]
        })
        .collect();
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(app.theme.background)),
        chunks[5],
    );
}

/// Two compact key/value stat lines.
fn render_detail_stats(app: &App, frame: &mut Frame, area: Rect, detail: &ProgramDetail) {
    let lines = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let line1 = format!(
        " Txs: {}  │  TPS: {:.2}  │  CU/s: {}  │  Active slots: {}",
        format_large_number(detail.total_txs as u64),
        detail.tx_per_sec,
        format_cu(detail.cu_per_sec),
        detail.slot_count,
    );
    frame.render_widget(
        Paragraph::new(line1).style(app.theme.normal_style()),
        lines[0],
    );

    let mut parts = vec![format!(
        " Avg CU: {}  │  Min: {}  │  Max: {}",
        format_cu(detail.avg_cu),
        format_cu(detail.min_cu as f64),
        format_cu(detail.max_cu as f64),
    )];
    if let Some(first_seen) = detail.first_seen {
        parts.push(format!(
            "First seen: {} ago",
            format_duration(first_seen.elapsed())
        ));
    }
    if let Some(last_seen) = detail.last_seen {
        parts.push(format!(
            "Last activity: {} ago",
            format_duration(last_seen.elapsed())
        ));
    }
    frame.render_widget(
        Paragraph::new(parts.join("  │  ")).style(app.theme.muted_style()),
        lines[1],
    );
}

/// Render charts based on detail view mode
fn render_detail_chart(app: &App, frame: &mut Frame, area: Rect) {
    match app.detail_view_mode {
        DetailViewMode::AllCharts => render_all_charts(app, frame, area),
        DetailViewMode::FullScreen => render_single_chart(app, frame, area),
    }
}

/// Render an inverted label bar above a chart, then the chart below it.
fn labeled_chart(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    label: &str,
    draw: impl FnOnce(&App, &mut Frame, Rect),
) {
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);
    frame.render_widget(label_bar(app, label), parts[0]);
    draw(app, frame, parts[1]);
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

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    labeled_chart(app, frame, rows[0], "Transactions", |a, f, r| {
        if let Some(d) = &a.cached_program_detail {
            charts::render_tx_chart(a, f, r, d);
        }
    });

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    labeled_chart(app, frame, columns[0], "Compute Units", |a, f, r| {
        if let Some(d) = &a.cached_program_detail {
            charts::render_cu_chart(a, f, r, d);
        }
    });
    labeled_chart(app, frame, columns[1], "Success Rate", |a, f, r| {
        if let Some(d) = &a.cached_program_detail {
            charts::render_success_chart(a, f, r, d);
        }
    });
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

    let (label, kind) = match app.current_chart {
        ChartType::Transactions => ("Transactions", ChartType::Transactions),
        ChartType::ComputeUnits => ("Compute Units", ChartType::ComputeUnits),
        ChartType::SuccessRate => ("Success Rate", ChartType::SuccessRate),
    };
    labeled_chart(app, frame, area, label, move |a, f, r| {
        if let Some(d) = &a.cached_program_detail {
            match kind {
                ChartType::Transactions => charts::render_tx_chart(a, f, r, d),
                ChartType::ComputeUnits => charts::render_cu_chart(a, f, r, d),
                ChartType::SuccessRate => charts::render_success_chart(a, f, r, d),
            }
        }
    });
}
