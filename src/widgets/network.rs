use crate::theme::Theme;
use crate::widgets::WidgetModule;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use sysinfo::Networks;

struct NetIface {
    name: String,
    rx_bytes: u64,
    tx_bytes: u64,
}

pub struct NetworkWidget {
    interfaces: Vec<NetIface>,
    scroll: usize,
}

impl NetworkWidget {
    pub fn new() -> Self {
        Self {
            interfaces: Vec::new(),
            scroll: 0,
        }
    }

    fn refresh(&mut self) {
        let networks = Networks::new_with_refreshed_list();
        self.interfaces = networks
            .iter()
            .map(|(name, data)| NetIface {
                name: name.clone(),
                rx_bytes: data.total_received(),
                tx_bytes: data.total_transmitted(),
            })
            .collect();
        self.interfaces
            .sort_by(|a, b| b.rx_bytes.cmp(&a.rx_bytes));
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

        let visible = inner.height as usize / 2;
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
        frame.render_widget(paragraph, inner);
    }

    fn handle_input(&mut self, event: KeyEvent) -> bool {
        match event.code {
            KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                true
            }
            KeyCode::Down => {
                if self.scroll + 1 < self.interfaces.len() {
                    self.scroll += 1;
                }
                true
            }
            KeyCode::Enter => {
                self.refresh();
                true
            }
            _ => false,
        }
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
