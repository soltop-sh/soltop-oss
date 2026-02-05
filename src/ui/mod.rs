//! Terminal user interface for soltop
//!
//! This module contains the TUI app that displays network statistics
//! in an interactive terminal dashboard.

mod app;
mod theme;
mod types;
mod formatting;
mod input;
mod renderer;

pub use app::App;
pub use theme::Theme;
