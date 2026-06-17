//! htop-style meter bars rendered with the soltop brand gradient.
//!
//! A meter looks like `LABEL [|||||||        VALUE]` where the filled portion is
//! coloured green→amber→red across its length (the same gradient as the soltop
//! website's "Live Status" bars and htop's CPU meters), and the numeric value is
//! overlaid on the right of the track.

use crate::ui::theme::Theme;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Linear interpolate between two RGB triples.
fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> Color {
    let f = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
    Color::Rgb(f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}

/// Brand gradient across `ratio` (0..1): green → amber → red.
/// Matches the website's Live Status bars and htop's load coloring.
pub fn gradient_color(ratio: f64) -> Color {
    let r = ratio.clamp(0.0, 1.0);
    const GREEN: (u8, u8, u8) = (0, 255, 0);
    const AMBER: (u8, u8, u8) = (251, 191, 36);
    const RED: (u8, u8, u8) = (239, 68, 68);
    if r < 0.5 {
        lerp_rgb(GREEN, AMBER, r / 0.5)
    } else {
        lerp_rgb(AMBER, RED, (r - 0.5) / 0.5)
    }
}

/// Build one htop-style meter line.
///
/// * `label`   – left-hand label, padded to `label_w`.
/// * `ratio`   – fill fraction 0..1 (drives both bar length and gradient).
/// * `value`   – text overlaid on the right of the track (e.g. "2713", "81.4%").
/// * `bar_w`   – number of cells inside the brackets.
pub fn meter_line(
    label: &str,
    ratio: f64,
    value: &str,
    label_w: usize,
    bar_w: usize,
    theme: &Theme,
) -> Line<'static> {
    let r = ratio.clamp(0.0, 1.0);
    let filled = (r * bar_w as f64).round() as usize;

    // Overlay the value text right-aligned inside the track.
    let val: Vec<char> = value.chars().collect();
    let val_start = bar_w.saturating_sub(val.len());

    let mut cells: Vec<Span> = Vec::with_capacity(bar_w);
    for i in 0..bar_w {
        let in_value = i >= val_start && (i - val_start) < val.len();
        if in_value {
            // Value digits: bright so they read over the track.
            cells.push(Span::styled(
                val[i - val_start].to_string(),
                Style::default()
                    .fg(theme.white)
                    .bg(theme.background)
                    .add_modifier(Modifier::BOLD),
            ));
        } else if i < filled {
            cells.push(Span::styled(
                "|".to_string(),
                Style::default()
                    .fg(gradient_color(i as f64 / bar_w as f64))
                    .bg(theme.background),
            ));
        } else {
            // Dim empty track.
            cells.push(Span::styled(
                " ".to_string(),
                Style::default().fg(theme.border).bg(theme.background),
            ));
        }
    }

    let mut spans: Vec<Span> = Vec::with_capacity(bar_w + 3);
    spans.push(Span::styled(
        format!("{label:<label_w$} "),
        theme.muted_style(),
    ));
    spans.push(Span::styled("[".to_string(), theme.muted_style()));
    spans.extend(cells);
    spans.push(Span::styled("]".to_string(), theme.muted_style()));
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_endpoints_and_midpoint() {
        assert_eq!(gradient_color(0.0), Color::Rgb(0, 255, 0)); // green
        assert_eq!(gradient_color(1.0), Color::Rgb(239, 68, 68)); // red
        assert_eq!(gradient_color(0.5), Color::Rgb(251, 191, 36)); // amber
                                                                   // clamps out-of-range
        assert_eq!(gradient_color(-1.0), Color::Rgb(0, 255, 0));
        assert_eq!(gradient_color(2.0), Color::Rgb(239, 68, 68));
    }

    #[test]
    fn meter_line_has_brackets_and_value() {
        let theme = Theme::flatline();
        let line = meter_line("TPS", 0.5, "2713", 8, 20, &theme);
        let text: String = line
            .spans
            .iter()
            .map(|s| s.content.clone().into_owned())
            .collect();
        assert!(text.starts_with("TPS"));
        assert!(text.contains('['));
        assert!(text.ends_with(']'));
        assert!(text.contains("2713"));
    }
}
