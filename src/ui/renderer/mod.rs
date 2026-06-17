mod charts;
mod detail_view;
mod gauges;
mod main_view;

use super::app::App;
use ratatui::Frame;

/// Main render dispatcher - delegates to appropriate view
pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    // Show loading screen if no data yet
    if app.loading {
        main_view::render_loading_screen(app, frame, area);
        return;
    }

    // Show detail view if active, otherwise show main view
    if app.showing_detail {
        detail_view::render_detail_view(app, frame, area);
    } else {
        main_view::render_main_view(app, frame, area);
    }
}
