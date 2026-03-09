use crate::theme::Theme;
use crate::widgets::{WidgetModule, WidgetAction};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};
use std::process::Command;

struct PortEntry {
    proto: String,
    local_addr: String,
    pid_name: String,
}

pub struct PortsWidget {
    ports: Vec<PortEntry>,
    state: ListState,
}

impl PortsWidget {
    pub fn new() -> Self {
        Self {
            ports: Vec::new(),
            state: ListState::default(),
        }
    }

    fn refresh(&mut self) {
        self.ports.clear();

        // Try ss first, fall back to /proc/net parsing
        if let Some(entries) = self.scan_ss() {
            self.ports = entries;
        }

        if self.state.selected().is_none() && !self.ports.is_empty() {
            self.state.select(Some(0));
        }
    }

    fn scan_ss(&self) -> Option<Vec<PortEntry>> {
        let output = Command::new("ss")
            .args(["-tlnp"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let entries: Vec<PortEntry> = text
            .lines()
            .skip(1) // header
            .filter_map(|line| {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.len() >= 5 {
                    let local = cols.get(3).unwrap_or(&"").to_string();
                    let process = cols.get(5..).map(|s| s.join(" ")).unwrap_or_default();
                    // Extract process name from users:(("name",pid=...,fd=...))
                    let pid_name = process
                        .split('"')
                        .nth(1)
                        .unwrap_or("—")
                        .to_string();
                    Some(PortEntry {
                        proto: "tcp".into(),
                        local_addr: local,
                        pid_name,
                    })
                } else {
                    None
                }
            })
            .collect();

        Some(entries)
    }
}

impl WidgetModule for PortsWidget {
    fn name(&self) -> &str {
        "Ports"
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
            .title(format!(" ◈ Listening Ports [{}] ", self.ports.len()))
            .title_style(title_style)
            .style(Style::default().bg(Theme::BG));

        let items: Vec<ListItem> = if self.ports.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "No listening ports detected",
                Theme::label(),
            )))]
        } else {
            self.ports
                .iter()
                .map(|p| {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!(" {} ", p.proto),
                            Style::default()
                                .fg(Theme::CYAN)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{:<25} ", p.local_addr),
                            Theme::text(),
                        ),
                        Span::styled(
                            p.pid_name.clone(),
                            Style::default().fg(Theme::MAGENTA),
                        ),
                    ]))
                })
                .collect()
        };

        let list = List::new(items)
            .block(block)
            .highlight_style(Theme::highlight());

        let mut state = self.state.clone();
        frame.render_stateful_widget(list, area, &mut state);
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
                if i + 1 < self.ports.len() {
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
        !self.ports.is_empty()
    }

    fn status_hint(&self) -> String {
        "↑↓: scroll  Enter: refresh".into()
    }
}
