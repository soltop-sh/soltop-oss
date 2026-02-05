use crate::ui::app::App;
use crate::ui::formatting::format_cu;
use crate::ui::types::ProgramDetail;
use ratatui::{
    layout::Rect,
    style::Style,
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Frame,
};

/// Render transaction count chart
pub fn render_tx_chart(app: &App, frame: &mut Frame, area: Rect, detail: &ProgramDetail) {
    // Transform slot data to chart points
    let data: Vec<(f64, f64)> = detail
        .slot_timeline
        .iter()
        .enumerate()
        .map(|(i, slot)| (i as f64, slot.tx_count as f64))
        .collect();

    if data.is_empty() {
        return;
    }

    // Calculate bounds
    let max_tx = detail
        .slot_timeline
        .iter()
        .map(|s| s.tx_count)
        .max()
        .unwrap_or(1) as f64;

    let y_max = (max_tx * 1.1).max(1.0); // Add 10% padding

    // Create dataset
    let dataset = Dataset::default()
        .name("Transactions")
        .marker(symbols::Marker::Braille) // Use Braille for smooth lines
        .graph_type(GraphType::Line)
        .style(Style::default().fg(app.theme.neon_green))
        .data(&data);

    // Create X axis
    let x_labels = generate_time_labels(app, detail.slot_timeline.len());
    let x_axis = Axis::default()
        .title("Time")
        .style(app.theme.muted_style())
        .bounds([0.0, data.len() as f64])
        .labels(x_labels);

    // Create Y axis
    let y_axis = Axis::default()
        .title("Tx Count")
        .style(app.theme.muted_style())
        .bounds([0.0, y_max])
        .labels(vec![
            Span::raw("0"),
            Span::raw(format!("{:.0}", y_max / 2.0)),
            Span::raw(format!("{:.0}", y_max)),
        ]);

    // Create chart
    let chart = Chart::new(vec![dataset])
        .block(
            Block::default()
                .title(" Transaction Activity (Last 5min) ")
                .borders(Borders::ALL)
                .border_style(app.theme.border_style())
                .title_style(app.theme.header_style()),
        )
        .x_axis(x_axis)
        .y_axis(y_axis);

    frame.render_widget(chart, area);
}

/// Render compute units chart
pub fn render_cu_chart(app: &App, frame: &mut Frame, area: Rect, detail: &ProgramDetail) {
    // Transform slot data to CU points
    let data: Vec<(f64, f64)> = detail
        .slot_timeline
        .iter()
        .enumerate()
        .map(|(i, slot)| (i as f64, slot.total_cu as f64))
        .collect();

    if data.is_empty() {
        return;
    }

    // Calculate bounds
    let max_cu = detail
        .slot_timeline
        .iter()
        .map(|s| s.total_cu)
        .max()
        .unwrap_or(1) as f64;

    let y_max = (max_cu * 1.1).max(1.0);

    // Create dataset with cyan color
    let dataset = Dataset::default()
        .name("Compute Units")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(app.theme.cyan))
        .data(&data);

    // Create axes
    let x_labels = generate_time_labels(app, detail.slot_timeline.len());
    let x_axis = Axis::default()
        .title("Time")
        .style(app.theme.muted_style())
        .bounds([0.0, data.len() as f64])
        .labels(x_labels);

    let y_axis = Axis::default()
        .title("Compute Units")
        .style(app.theme.muted_style())
        .bounds([0.0, y_max])
        .labels(vec![
            Span::raw("0"),
            Span::raw(format_cu(y_max / 2.0)),
            Span::raw(format_cu(y_max)),
        ]);

    // Create chart
    let chart = Chart::new(vec![dataset])
        .block(
            Block::default()
                .title(" Compute Units Usage (Last 5min) ")
                .borders(Borders::ALL)
                .border_style(app.theme.border_style())
                .title_style(app.theme.header_style()),
        )
        .x_axis(x_axis)
        .y_axis(y_axis);

    frame.render_widget(chart, area);
}

/// Render success rate chart
pub fn render_success_chart(app: &App, frame: &mut Frame, area: Rect, detail: &ProgramDetail) {
    // Calculate success rate per slot
    let data: Vec<(f64, f64)> = detail
        .slot_timeline
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let rate = if slot.tx_count > 0 {
                (slot.success_count as f64 / slot.tx_count as f64) * 100.0
            } else {
                100.0 // No transactions = 100% success by default
            };
            (i as f64, rate)
        })
        .collect();

    if data.is_empty() {
        return;
    }

    // Create dataset with color based on average success rate
    let color = app.theme.success_rate_color(detail.success_rate);
    let dataset = Dataset::default()
        .name("Success Rate")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(color))
        .data(&data);

    // Create axes
    let x_labels = generate_time_labels(app, detail.slot_timeline.len());
    let x_axis = Axis::default()
        .title("Time")
        .style(app.theme.muted_style())
        .bounds([0.0, data.len() as f64])
        .labels(x_labels);

    let y_axis = Axis::default()
        .title("Success Rate (%)")
        .style(app.theme.muted_style())
        .bounds([0.0, 100.0])
        .labels(vec![
            Span::raw("0%"),
            Span::raw("50%"),
            Span::raw("100%"),
        ]);

    // Create chart
    let chart = Chart::new(vec![dataset])
        .block(
            Block::default()
                .title(" Success Rate (Last 5min) ")
                .borders(Borders::ALL)
                .border_style(app.theme.border_style())
                .title_style(app.theme.header_style()),
        )
        .x_axis(x_axis)
        .y_axis(y_axis);

    frame.render_widget(chart, area);
}

/// Generate time labels for chart X-axis
fn generate_time_labels(app: &App, slot_count: usize) -> Vec<Span<'static>> {
    if slot_count == 0 {
        return vec![Span::raw("now")];
    }

    // Assuming ~400ms per slot, calculate approximate minutes
    let total_seconds = (slot_count as f64 * 0.4).round() as u64;
    let total_minutes = total_seconds / 60;

    vec![
        Span::styled(
            format!("-{}m", total_minutes),
            app.theme.muted_style(),
        ),
        Span::styled(
            format!("-{}m", total_minutes / 2),
            app.theme.muted_style(),
        ),
        Span::styled("now", app.theme.muted_style()),
    ]
}
