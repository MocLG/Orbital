use crate::theme::Theme;
use crate::widgets::{WidgetModule, WidgetAction};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use ratatui::widgets::canvas::{Canvas, Line as CLine};
use sysinfo::System;

const HISTORY_LEN: usize = 100;

pub struct SystemWidget {
    sys: System,
    cpu_usage: f32,
    mem_total: u64,
    mem_used: u64,
    uptime: u64,
    hostname: String,
    os_name: String,
    cpu_name: String,
    cpu_count: usize,
    cpu_history: Vec<u64>,
    mem_history: Vec<u64>,
}

impl SystemWidget {
    pub fn new() -> Self {
        Self {
            sys: System::new(),
            cpu_usage: 0.0,
            mem_total: 0,
            mem_used: 0,
            uptime: 0,
            hostname: String::new(),
            os_name: String::new(),
            cpu_name: String::new(),
            cpu_count: 0,
            cpu_history: Vec::with_capacity(HISTORY_LEN),
            mem_history: Vec::with_capacity(HISTORY_LEN),
        }
    }

    fn refresh(&mut self) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        self.cpu_usage = self.sys.global_cpu_usage();
        self.mem_total = self.sys.total_memory();
        self.mem_used = self.sys.used_memory();
        self.uptime = System::uptime();

        // Push to history ring
        self.cpu_history.push(self.cpu_usage as u64);
        if self.cpu_history.len() > HISTORY_LEN {
            self.cpu_history.remove(0);
        }
        let mem_pct = if self.mem_total > 0 {
            (self.mem_used as f64 / self.mem_total as f64 * 100.0) as u64
        } else {
            0
        };
        self.mem_history.push(mem_pct);
        if self.mem_history.len() > HISTORY_LEN {
            self.mem_history.remove(0);
        }
    }
}

impl WidgetModule for SystemWidget {
    fn name(&self) -> &str {
        "System"
    }

    fn init(&mut self) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.hostname = System::host_name().unwrap_or_else(|| "unknown".into());
        self.os_name = format!(
            "{} {}",
            System::name().unwrap_or_default(),
            System::os_version().unwrap_or_default()
        );
        self.cpu_name = self
            .sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "unknown".into());
        self.cpu_count = self.sys.cpus().len();
        self.refresh();
    }

    fn update_state(&mut self) {
        self.refresh();
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool) {
        // Threshold alert: if CPU or RAM > 85%, border goes red
        let alert = self.cpu_usage > 85.0
            || (self.mem_total > 0 && (self.mem_used as f64 / self.mem_total as f64) > 0.85);

        let (border_type, border_style, title_style) = if alert {
            (BorderType::Thick, Theme::bad(), Style::default().fg(Theme::RED).add_modifier(Modifier::BOLD))
        } else if is_focused {
            (BorderType::Double, Theme::border_focused(), Theme::title_focused())
        } else {
            (BorderType::Thick, Theme::border_unfocused(), Theme::title_unfocused())
        };

        let title = if alert { "[ SYSTEM ⚠ ]" } else { "[ SYSTEM ]" };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style)
            .title(title)
            .title_style(title_style)
            .style(Style::default().bg(Theme::BG));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // info
                Constraint::Min(3),   // CPU braille
                Constraint::Min(3),   // RAM braille
            ])
            .split(inner);

        // Info lines
        let uptime_str = format_uptime(self.uptime);
        let info = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("HOST ", Theme::label()),
                Span::styled(&self.hostname, Theme::text()),
                Span::styled("  OS ", Theme::label()),
                Span::styled(&self.os_name, Theme::text()),
            ]),
            Line::from(vec![
                Span::styled("CPU  ", Theme::label()),
                Span::styled(
                    format!("{} ({}c)", self.cpu_name, self.cpu_count),
                    Theme::text(),
                ),
            ]),
            Line::from(vec![
                Span::styled("UP   ", Theme::label()),
                Span::styled(uptime_str, Style::default().fg(Theme::NEON_GREEN)),
            ]),
        ]);
        frame.render_widget(info, chunks[0]);

        // CPU braille graph
        let cpu_color = if self.cpu_usage > 85.0 {
            Theme::RED
        } else if self.cpu_usage > 50.0 {
            Theme::AMBER
        } else {
            Theme::CYAN
        };
        let cpu_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(2)])
            .split(chunks[1]);
        let cpu_label = Paragraph::new(Span::styled(
            format!("CPU {:.1}%", self.cpu_usage),
            Style::default().fg(cpu_color).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(cpu_label, cpu_rows[0]);

        let cpu_data = self.cpu_history.clone();
        let cpu_len = cpu_data.len();
        let cpu_canvas = Canvas::default()
            .marker(Marker::Braille)
            .x_bounds([0.0, HISTORY_LEN as f64])
            .y_bounds([0.0, 100.0])
            .background_color(Theme::SURFACE)
            .paint(move |ctx| {
                for i in 1..cpu_len {
                    let age = i as f64 / cpu_len as f64;
                    let val = cpu_data[i] as f64;
                    let color = if val > 85.0 {
                        Color::Rgb(255, 0, 200)
                    } else {
                        Color::Rgb(
                            (30.0 * (1.0 - age)) as u8,
                            (60.0 + 195.0 * age) as u8,
                            (180.0 + 75.0 * age) as u8,
                        )
                    };
                    ctx.draw(&CLine {
                        x1: (i - 1) as f64,
                        y1: cpu_data[i - 1] as f64,
                        x2: i as f64,
                        y2: val,
                        color,
                    });
                }
            });
        frame.render_widget(cpu_canvas, cpu_rows[1]);

        // RAM braille graph
        let mem_pct = if self.mem_total > 0 {
            self.mem_used as f64 / self.mem_total as f64
        } else {
            0.0
        };
        let mem_color = if mem_pct > 0.85 {
            Theme::RED
        } else if mem_pct > 0.5 {
            Theme::AMBER
        } else {
            Theme::NEON_GREEN
        };
        let mem_used_gb = self.mem_used as f64 / 1_073_741_824.0;
        let mem_total_gb = self.mem_total as f64 / 1_073_741_824.0;
        let mem_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(2)])
            .split(chunks[2]);
        let mem_label = Paragraph::new(Span::styled(
            format!("RAM {:.1}/{:.1} GB ({:.0}%)", mem_used_gb, mem_total_gb, mem_pct * 100.0),
            Style::default().fg(mem_color).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(mem_label, mem_rows[0]);

        let mem_data = self.mem_history.clone();
        let mem_len = mem_data.len();
        let mem_canvas = Canvas::default()
            .marker(Marker::Braille)
            .x_bounds([0.0, HISTORY_LEN as f64])
            .y_bounds([0.0, 100.0])
            .background_color(Theme::SURFACE)
            .paint(move |ctx| {
                for i in 1..mem_len {
                    let age = i as f64 / mem_len as f64;
                    let val = mem_data[i] as f64;
                    let color = if val > 85.0 {
                        Color::Rgb(255, 0, 200)
                    } else {
                        Color::Rgb(
                            (30.0 * (1.0 - age)) as u8,
                            (60.0 + 195.0 * age) as u8,
                            (180.0 + 75.0 * age) as u8,
                        )
                    };
                    ctx.draw(&CLine {
                        x1: (i - 1) as f64,
                        y1: mem_data[i - 1] as f64,
                        x2: i as f64,
                        y2: val,
                        color,
                    });
                }
            });
        frame.render_widget(mem_canvas, mem_rows[1]);
    }

    fn handle_input(&mut self, event: KeyEvent) -> WidgetAction {
        if event.code == KeyCode::Enter {
            self.refresh();
            return WidgetAction::None;
        }
        WidgetAction::None
    }

    fn status_hint(&self) -> String {
        "Enter: refresh".into()
    }
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}
