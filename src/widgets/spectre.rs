use crate::theme::Theme;
use crate::widgets::{WidgetAction, WidgetModule};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};
use std::process::Command;

struct Connection {
    local: String,
    remote: String,
    state: String,
}

pub struct SpectreWidget {
    connections: Vec<Connection>,
    state: ListState,
}

impl SpectreWidget {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            state: ListState::default(),
        }
    }

    fn refresh(&mut self) {
        self.connections.clear();

        let output = Command::new("ss")
            .args(["-tnp"])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                self.connections = text
                    .lines()
                    .skip(1)
                    .filter_map(|line| {
                        let cols: Vec<&str> = line.split_whitespace().collect();
                        if cols.len() >= 5 {
                            Some(Connection {
                                state: cols.first().unwrap_or(&"").to_string(),
                                local: cols.get(3).unwrap_or(&"").to_string(),
                                remote: cols.get(4).unwrap_or(&"").to_string(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect();
            }
        }

        if self.state.selected().is_none() && !self.connections.is_empty() {
            self.state.select(Some(0));
        }
    }
}

impl WidgetModule for SpectreWidget {
    fn name(&self) -> &str {
        "Spectre"
    }

    fn init(&mut self) {
        self.refresh();
    }

    fn update_state(&mut self) {
        self.refresh();
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool) {
        let (border_type, border_style, title_style) = if is_focused {
            (BorderType::Double, Theme::border_focused(), Theme::title_focused())
        } else {
            (BorderType::Thick, Theme::border_unfocused(), Theme::title_unfocused())
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style)
            .title(format!("[ SPECTRE {} ]", self.connections.len()))
            .title_style(title_style)
            .style(Style::default().bg(Theme::BG));

        let items: Vec<ListItem> = if self.connections.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "No active connections",
                Theme::label(),
            )))]
        } else {
            self.connections
                .iter()
                .map(|c| {
                    let is_local = c.remote.starts_with("127.0.0.1")
                        || c.remote.starts_with("[::1]")
                        || c.remote.starts_with("::1");
                    let remote_style = if is_local {
                        Theme::text()
                    } else {
                        Style::default().fg(Theme::TOXIC_ORANGE).add_modifier(Modifier::BOLD)
                    };

                    let state_color = match c.state.as_str() {
                        "ESTAB" => Theme::NEON_GREEN,
                        "TIME-WAIT" | "CLOSE-WAIT" => Theme::AMBER,
                        "SYN-SENT" | "SYN-RECV" => Theme::CYAN,
                        _ => Theme::DIM,
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!(" {:<11} ", c.state),
                            Style::default()
                                .fg(state_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{} ", truncate(&c.local, 20)),
                            Theme::text(),
                        ),
                        Span::styled("➜ ", Style::default().fg(Theme::MAGENTA).add_modifier(Modifier::BOLD)),
                        Span::styled(truncate(&c.remote, 20), remote_style),
                    ]))
                })
                .collect()
        };

        let list = List::new(items)
            .block(block)
            .highlight_style(Theme::highlight());

        let mut list_state = self.state.clone();
        frame.render_stateful_widget(list, area, &mut list_state);
    }

    fn handle_input(&mut self, event: KeyEvent) -> WidgetAction {
        match event.code {
            KeyCode::Up => {
                let i = self.state.selected().unwrap_or(0);
                if i > 0 {
                    self.state.select(Some(i - 1));
                }
                WidgetAction::None
            }
            KeyCode::Down => {
                let i = self.state.selected().unwrap_or(0);
                if i + 1 < self.connections.len() {
                    self.state.select(Some(i + 1));
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
        !self.connections.is_empty()
    }

    fn status_hint(&self) -> String {
        "↑↓: scroll  Enter: refresh".into()
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max - 1])
    } else {
        s.to_string()
    }
}
