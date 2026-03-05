use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    // ── Neon accent palette ──
    pub const CYAN: Color = Color::Rgb(0, 255, 255);
    pub const MAGENTA: Color = Color::Rgb(255, 0, 200);
    pub const NEON_GREEN: Color = Color::Rgb(57, 255, 20);
    pub const AMBER: Color = Color::Rgb(255, 191, 0);
    pub const SOFT_WHITE: Color = Color::Rgb(200, 200, 210);
    pub const DIM: Color = Color::Rgb(100, 100, 120);
    pub const BG: Color = Color::Rgb(15, 15, 25);
    pub const SURFACE: Color = Color::Rgb(25, 25, 40);
    pub const RED: Color = Color::Rgb(255, 85, 85);
    pub const BLUE: Color = Color::Rgb(100, 149, 237);

    pub fn border_focused() -> Style {
        Style::default().fg(Self::CYAN)
    }

    pub fn border_unfocused() -> Style {
        Style::default().fg(Self::DIM)
    }

    pub fn title_focused() -> Style {
        Style::default()
            .fg(Self::CYAN)
            .add_modifier(Modifier::BOLD)
    }

    pub fn title_unfocused() -> Style {
        Style::default().fg(Self::DIM)
    }

    pub fn highlight() -> Style {
        Style::default()
            .fg(Self::BG)
            .bg(Self::CYAN)
            .add_modifier(Modifier::BOLD)
    }

    pub fn key_hint() -> Style {
        Style::default()
            .fg(Self::AMBER)
            .add_modifier(Modifier::BOLD)
    }

    pub fn text() -> Style {
        Style::default().fg(Self::SOFT_WHITE)
    }

    pub fn label() -> Style {
        Style::default()
            .fg(Self::DIM)
            .add_modifier(Modifier::ITALIC)
    }

    pub fn good() -> Style {
        Style::default().fg(Self::NEON_GREEN)
    }

    pub fn warn() -> Style {
        Style::default().fg(Self::AMBER)
    }

    pub fn bad() -> Style {
        Style::default().fg(Self::RED)
    }

    pub fn accent() -> Style {
        Style::default().fg(Self::MAGENTA)
    }
}
