use crate::theme::Theme;
use crate::widgets::{WidgetModule, WidgetAction};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Sparkline},
    Frame,
};
use sysinfo::Networks;

const HISTORY_LEN: usize = 40;

struct NetIface {
    name: String,
    rx_bytes: u64,
    tx_bytes: u64,
    rx_rate: u64,
    tx_rate: u64,
}

pub struct NetworkWidget {
    interfaces: Vec<NetIface>,
    scroll: usize,
    rx_history: Vec<u64>,
    tx_history: Vec<u64>,
    networks: Networks,
    prev_rx: u64,
    prev_tx: u64,
}

impl NetworkWidget {
    pub fn new() -> Self {
        Self {
            interfaces: Vec::new(),
            scroll: 0,
            rx_history: Vec::with_capacity(HISTORY_LEN),
            tx_history: Vec::with_capacity(HISTORY_LEN),
            networks: Networks::new_with_refreshed_list(),
            prev_rx: 0,
            prev_tx: 0,
        }
    }

    fn refresh(&mut self) {
        self.networks.refresh(true);
        self.interfaces = self.networks
            .iter()
            .filter(|(name, data)| {
                !name.starts_with("lo")
                    && (data.total_received() > 0 || data.total_transmitted() > 0)
            })
            .map(|(name, data)| NetIface {
                name: name.clone(),
                rx_bytes: data.total_received(),
                tx_bytes: data.total_transmitted(),
                rx_rate: data.received(),
                tx_rate: data.transmitted(),
            })
            .collect();
        self.interfaces
            .sort_by(|a, b| b.rx_bytes.cmp(&a.rx_bytes));

        // Aggregate total RX/TX rate for sparkline
        let total_rx: u64 = self.interfaces.iter().map(|i| i.rx_rate).sum();
        let total_tx: u64 = self.interfaces.iter().map(|i| i.tx_rate).sum();
        self.rx_history.push(total_rx / 1024); // KB/s
        if self.rx_history.len() > HISTORY_LEN {
            self.rx_history.remove(0);
        }
        self.tx_history.push(total_tx / 1024);
        if self.tx_history.len() > HISTORY_LEN {
            self.tx_history.remove(0);
        }
        self.prev_rx = total_rx;
        self.prev_tx = total_tx;
    }
}

impl WidgetModule for NetworkWidget {
    fn name(&self) -> &str {
        "Network"
    }

    fn init(&mut self) {
        self.refresh();
    }

    fn update_state(&mut self) {
        self.refresh();
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool) {
        let (border_style, title_style) = if is_focused {
            (Theme::border_focused(), Theme::title_focused())
        } else {
            (Theme::border_unfocused(), Theme::title_unfocused())
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(" ◈ Network ")
            .title_style(title_style)
            .style(Style::default().bg(Theme::BG));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.interfaces.is_empty() {
            let msg = Paragraph::new("No interfaces detected").style(Theme::label());
            frame.render_widget(msg, inner);
            return;
        }

        // Layout: sparklines on top, then interface list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // RX sparkline
                Constraint::Length(2), // TX sparkline
                Constraint::Min(2),   // interface list
            ])
            .split(inner);

        // RX sparkline
        let rx_label = format!("▼ RX {} KB/s", self.prev_rx / 1024);
        let rx_spark = Sparkline::default()
            .data(&self.rx_history)
            .style(Style::default().fg(Theme::NEON_GREEN).bg(Theme::SURFACE));
        let rx_line = Paragraph::new(Line::from(Span::styled(rx_label, Style::default().fg(Theme::NEON_GREEN))));
        frame.render_widget(rx_line, Rect::new(chunks[0].x, chunks[0].y, chunks[0].width, 1));
        frame.render_widget(rx_spark, Rect::new(chunks[0].x, chunks[0].y + 1, chunks[0].width, 1));

        // TX sparkline
        let tx_label = format!("▲ TX {} KB/s", self.prev_tx / 1024);
        let tx_spark = Sparkline::default()
            .data(&self.tx_history)
            .style(Style::default().fg(Theme::MAGENTA).bg(Theme::SURFACE));
        let tx_line = Paragraph::new(Line::from(Span::styled(tx_label, Style::default().fg(Theme::MAGENTA))));
        frame.render_widget(tx_line, Rect::new(chunks[1].x, chunks[1].y, chunks[1].width, 1));
        frame.render_widget(tx_spark, Rect::new(chunks[1].x, chunks[1].y + 1, chunks[1].width, 1));

        // Interface list below
        let list_area = chunks[2];
        let visible = list_area.height as usize / 2;
        let end = (self.scroll + visible).min(self.interfaces.len());
        let shown = &self.interfaces[self.scroll..end];

        let lines: Vec<Line> = shown
            .iter()
            .flat_map(|iface| {
                vec![
                    Line::from(vec![
                        Span::styled(
                            format!("● {} ", iface.name),
                            Style::default()
                                .fg(Theme::CYAN)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("  ▼ ", Style::default().fg(Theme::NEON_GREEN)),
                        Span::styled(
                            format_bytes(iface.rx_bytes),
                            Theme::text(),
                        ),
                        Span::styled("  ▲ ", Style::default().fg(Theme::MAGENTA)),
                        Span::styled(
                            format_bytes(iface.tx_bytes),
                            Theme::text(),
                        ),
                    ]),
                ]
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, list_area);
    }

    fn handle_input(&mut self, event: KeyEvent) -> WidgetAction {
        match event.code {
            KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                WidgetAction::None
            }
            KeyCode::Down => {
                if self.scroll + 1 < self.interfaces.len() {
                    self.scroll += 1;
                }
                WidgetAction::None
            }
            KeyCode::Enter => {
                self.refresh();
                WidgetAction::None
            }
            _ => WidgetAction::None,
        }
    }

    fn is_visible(&self) -> bool {
        !self.interfaces.is_empty()
    }

    fn status_hint(&self) -> String {
        "↑↓: scroll  Enter: refresh".into()
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
