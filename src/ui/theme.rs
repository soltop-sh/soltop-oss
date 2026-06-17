use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy)]
pub struct Theme {
    /// Display name (shown in the theme menu).
    pub name: &'static str,
    pub background: Color,
    pub border: Color,
    /// Primary accent (headers, logo, selection tint). Named for the original
    /// neon-green theme; in other themes it's just the primary accent color.
    pub neon_green: Color,
    pub cyan: Color,
    pub amber: Color,
    pub success: Color,
    pub error: Color,
    pub white: Color,
    pub gray: Color,
}

impl Theme {
    /// Original neon-green-on-black theme.
    pub const fn flatline() -> Self {
        Self {
            name: "Flatline",
            background: Color::Rgb(0, 0, 0),
            border: Color::Rgb(51, 51, 51),
            neon_green: Color::Rgb(0, 255, 0),
            cyan: Color::Rgb(34, 211, 238),
            amber: Color::Rgb(251, 191, 36),
            success: Color::Rgb(52, 211, 153),
            error: Color::Rgb(239, 68, 68),
            white: Color::Rgb(255, 255, 255),
            gray: Color::Rgb(156, 163, 175),
        }
    }

    /// Deep-green phosphor / "Matrix" look on a dark-green background.
    pub const fn matrix() -> Self {
        Self {
            name: "Matrix",
            background: Color::Rgb(0, 18, 4),
            border: Color::Rgb(0, 70, 20),
            neon_green: Color::Rgb(0, 255, 65),
            cyan: Color::Rgb(0, 220, 120),
            amber: Color::Rgb(150, 255, 0),
            success: Color::Rgb(0, 255, 65),
            error: Color::Rgb(255, 80, 80),
            white: Color::Rgb(190, 255, 190),
            gray: Color::Rgb(0, 150, 40),
        }
    }

    /// Grayscale / monochrome on a neutral dark-gray background.
    pub const fn mono() -> Self {
        Self {
            name: "Mono",
            background: Color::Rgb(24, 24, 24),
            border: Color::Rgb(80, 80, 80),
            neon_green: Color::Rgb(235, 235, 235),
            cyan: Color::Rgb(200, 200, 200),
            amber: Color::Rgb(170, 170, 170),
            success: Color::Rgb(230, 230, 230),
            error: Color::Rgb(255, 120, 120),
            white: Color::Rgb(255, 255, 255),
            gray: Color::Rgb(140, 140, 140),
        }
    }

    /// Retro amber CRT on a warm dark-brown background.
    pub const fn amber_crt() -> Self {
        Self {
            name: "Amber",
            background: Color::Rgb(26, 14, 0),
            border: Color::Rgb(110, 70, 0),
            neon_green: Color::Rgb(255, 176, 0),
            cyan: Color::Rgb(255, 214, 90),
            amber: Color::Rgb(255, 176, 0),
            success: Color::Rgb(255, 200, 60),
            error: Color::Rgb(255, 90, 40),
            white: Color::Rgb(255, 232, 176),
            gray: Color::Rgb(170, 115, 35),
        }
    }

    /// Classic Windows console / PowerShell — light text on navy blue.
    pub const fn blue() -> Self {
        Self {
            name: "Blue",
            background: Color::Rgb(1, 36, 86), // PowerShell navy (#012456)
            border: Color::Rgb(70, 110, 160),
            neon_green: Color::Rgb(230, 232, 240), // light chrome (title/header bars)
            cyan: Color::Rgb(86, 170, 255),        // selection
            amber: Color::Rgb(255, 214, 90),
            success: Color::Rgb(120, 220, 150),
            error: Color::Rgb(255, 110, 110),
            white: Color::Rgb(240, 242, 248),
            gray: Color::Rgb(160, 175, 205),
        }
    }

    /// All selectable presets, in menu order.
    pub fn presets() -> [Theme; 5] {
        [
            Self::flatline(),
            Self::matrix(),
            Self::mono(),
            Self::amber_crt(),
            Self::blue(),
        ]
    }

    // --- Style helpers. All carry the theme background so the whole screen is
    // painted in the theme's color rather than the terminal default. ---

    fn base(&self) -> Style {
        Style::default().bg(self.background)
    }

    pub fn header_style(&self) -> Style {
        self.base().fg(self.neon_green).add_modifier(Modifier::BOLD)
    }

    pub fn border_style(&self) -> Style {
        self.base().fg(self.border)
    }

    pub fn table_header_style(&self) -> Style {
        self.base().fg(self.neon_green).add_modifier(Modifier::BOLD)
    }

    pub fn success_style(&self) -> Style {
        self.base().fg(self.success)
    }

    pub fn warning_style(&self) -> Style {
        self.base().fg(self.amber)
    }

    pub fn error_style(&self) -> Style {
        self.base().fg(self.error)
    }

    pub fn normal_style(&self) -> Style {
        self.base().fg(self.white)
    }

    pub fn muted_style(&self) -> Style {
        self.base().fg(self.gray)
    }

    /// Full-width selection bar (htop-style): cyan background, dark text.
    pub fn selection_style(&self) -> Style {
        Style::default()
            .bg(self.cyan)
            .fg(self.background)
            .add_modifier(Modifier::BOLD)
    }

    /// Inverted "chrome" bar (title, table header, footer): green background,
    /// dark text. Distinct from the cyan selection so the active row stands out.
    pub fn inverted_style(&self) -> Style {
        Style::default()
            .bg(self.neon_green)
            .fg(self.background)
            .add_modifier(Modifier::BOLD)
    }

    // Get color based on success rate percentage
    pub fn success_rate_color(&self, rate: f64) -> Color {
        if rate >= 95.0 {
            self.success
        } else if rate >= 80.0 {
            self.amber
        } else {
            self.error
        }
    }

    // Get color based on TPS value
    pub fn tps_color(&self, tps: f64) -> Color {
        if tps >= 100.0 {
            self.error // Very high = might be spam
        } else if tps >= 10.0 {
            self.amber // Moderate activity
        } else {
            self.success // Low = normal
        }
    }

    // Get color based on CU/s (compute units per second)
    pub fn cu_per_sec_color(&self, cu_per_sec: f64) -> Color {
        if cu_per_sec >= 10_000_000.0 {
            self.error
        } else if cu_per_sec >= 1_000_000.0 {
            self.amber
        } else {
            self.success
        }
    }

    // Get color based on average CU per transaction
    pub fn avg_cu_color(&self, avg_cu: f64) -> Color {
        if avg_cu >= 200_000.0 {
            self.error
        } else if avg_cu >= 50_000.0 {
            self.amber
        } else {
            self.success
        }
    }
}
