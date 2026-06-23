mod charts;
mod detail_view;
mod gauges;
mod main_view;

use super::app::App;
use ratatui::widgets::Block;
use ratatui::{style::Style, Frame};

/// Main render dispatcher - delegates to appropriate view
pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    // Paint the whole screen with the theme background so the terminal's own
    // background never shows through (forces the themed/black background).
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.background)),
        area,
    );

    // Show loading screen if no data yet
    if app.loading {
        main_view::render_loading_screen(app, frame, area);
    } else if app.showing_detail {
        detail_view::render_detail_view(app, frame, area);
    } else {
        main_view::render_main_view(app, frame, area);
    }

    // Theme picker overlays everything when open.
    if app.show_theme_menu {
        main_view::render_theme_menu(app, frame, area);
    }
}
