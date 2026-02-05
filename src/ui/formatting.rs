use std::time::Duration;

/// Format large numbers with comma separators (e.g., "1,234,567")
pub fn format_large_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

/// Format large numbers with K/M/B suffixes (e.g., "2.3M", "450.2K")
pub fn format_cu(n: f64) -> String {
    if n >= 1_000_000_000.0 {
        format!("{:.1}B", n / 1_000_000_000.0)
    } else if n >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("{:.1}K", n / 1_000.0)
    } else {
        format!("{:.0}", n)
    }
}

/// Format duration in human-readable form (e.g., "2m 34s", "1h 23m")
pub fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();

    if total_secs >= 3600 {
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    } else if total_secs >= 60 {
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", total_secs)
    }
}
