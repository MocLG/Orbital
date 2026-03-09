use crate::theme::Theme;
use crate::widgets::{WidgetModule, WidgetAction};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Sparkline},
    Frame,
};
use sysinfo::System;

const HISTORY_LEN: usize = 40;

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

        let (border_style, title_style) = if alert {
            (Theme::bad(), Style::default().fg(Theme::RED).add_modifier(Modifier::BOLD))
        } else if is_focused {
            (Theme::border_focused(), Theme::title_focused())
        } else {
            (Theme::border_unfocused(), Theme::title_unfocused())
        };

        let title = if alert { " ◈ System ⚠ " } else { " ◈ System " };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
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
                Constraint::Length(2), // cpu gauge
                Constraint::Length(2), // cpu sparkline
                Constraint::Length(2), // mem gauge
                Constraint::Min(0),   // padding
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

        // CPU gauge
        let cpu_pct = self.cpu_usage as f64 / 100.0;
        let cpu_color = if self.cpu_usage > 85.0 {
            Theme::RED
        } else if self.cpu_usage > 50.0 {
            Theme::AMBER
        } else {
            Theme::CYAN
        };
        let cpu_gauge = Gauge::default()
            .label(Span::styled(
                format!("CPU {:.1}%", self.cpu_usage),
                Style::default()
                    .fg(Theme::SOFT_WHITE)
                    .add_modifier(Modifier::BOLD),
            ))
            .gauge_style(Style::default().fg(cpu_color).bg(Theme::SURFACE))
            .ratio(cpu_pct.clamp(0.0, 1.0));
        frame.render_widget(cpu_gauge, chunks[1]);

        // CPU sparkline
        let cpu_spark = Sparkline::default()
            .data(&self.cpu_history)
            .max(100)
            .style(Style::default().fg(Theme::CYAN).bg(Theme::SURFACE));
        frame.render_widget(cpu_spark, chunks[2]);

        // Memory gauge
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
        let mem_gauge = Gauge::default()
            .label(Span::styled(
                format!("RAM {:.1}/{:.1} GB", mem_used_gb, mem_total_gb),
                Style::default()
                    .fg(Theme::SOFT_WHITE)
                    .add_modifier(Modifier::BOLD),
            ))
            .gauge_style(Style::default().fg(mem_color).bg(Theme::SURFACE))
            .ratio(mem_pct.clamp(0.0, 1.0));
        frame.render_widget(mem_gauge, chunks[3]);
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
