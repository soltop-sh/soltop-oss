use super::app::App;
use super::types::{ChartType, DetailViewMode, ViewMode};
use crossterm::event::KeyCode;

/// Handle keyboard input
pub fn handle_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            if app.showing_detail {
                // Return to main view from detail view
                app.showing_detail = false;
                app.selected_program_id = None;
                app.detail_view_mode = DetailViewMode::AllCharts; // Reset to default
            } else {
                // Quit from main view
                app.running = false;
            }
        }
        KeyCode::Char('q') | KeyCode::F(10) => {
            app.running = false;
        }
        KeyCode::Char('t') => {
            // Toggle ID truncation
            app.truncate_ids = !app.truncate_ids;
        }
        KeyCode::Char('u') => {
            // Toggle system program filter
            app.hide_system_programs = !app.hide_system_programs;
        }
        KeyCode::Char('w') => {
            // Toggle view mode
            app.view_mode = match app.view_mode {
                ViewMode::Live => ViewMode::Window,
                ViewMode::Window => ViewMode::Live,
            };
        }
        KeyCode::Down if !app.showing_detail => {
            let max_row = app.cached_stats.len().saturating_sub(1);
            app.selected_row = (app.selected_row + 1).min(max_row);
            app.table_state.select(Some(app.selected_row));
        }
        KeyCode::Up if !app.showing_detail => {
            app.selected_row = app.selected_row.saturating_sub(1);
            app.table_state.select(Some(app.selected_row));
        }
        KeyCode::Enter | KeyCode::Char(' ') if !app.showing_detail => {
            if let Some(stat) = app.cached_stats.get(app.selected_row) {
                app.selected_program_id = Some(stat.program_id.clone());
                app.showing_detail = true;
            }
        }
        KeyCode::Tab if app.showing_detail => {
            match app.detail_view_mode {
                DetailViewMode::AllCharts => {
                    app.detail_view_mode = DetailViewMode::FullScreen;
                    app.current_chart = ChartType::Transactions;
                }
                DetailViewMode::FullScreen => {
                    app.current_chart = match app.current_chart {
                        ChartType::Transactions => ChartType::ComputeUnits,
                        ChartType::ComputeUnits => ChartType::SuccessRate,
                        ChartType::SuccessRate => {
                            app.detail_view_mode = DetailViewMode::AllCharts;
                            ChartType::Transactions
                        }
                    };
                }
            }
        }
        _ => {}
    }
}
