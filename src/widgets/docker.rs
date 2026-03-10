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

struct Container {
    id: String,
    name: String,
    image: String,
    status: String,
    running: bool,
}

pub struct DockerWidget {
    containers: Vec<Container>,
    state: ListState,
    last_action: Option<String>,
}

impl DockerWidget {
    pub fn new() -> Self {
        Self {
            containers: Vec::new(),
            state: ListState::default(),
            last_action: None,
        }
    }

    fn refresh(&mut self) {
        let output = Command::new("docker")
            .args(["ps", "-a", "--format", "{{.ID}}|{{.Names}}|{{.Image}}|{{.Status}}|{{.State}}"])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                self.containers = text
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|line| {
                        let parts: Vec<&str> = line.splitn(5, '|').collect();
                        Container {
                            id: parts.first().unwrap_or(&"").to_string(),
                            name: parts.get(1).unwrap_or(&"").to_string(),
                            image: parts.get(2).unwrap_or(&"").to_string(),
                            status: parts.get(3).unwrap_or(&"").to_string(),
                            running: parts.get(4).unwrap_or(&"") == &"running",
                        }
                    })
                    .collect();
            }
        }

        if self.state.selected().is_none() && !self.containers.is_empty() {
            self.state.select(Some(0));
        }
    }

    fn docker_action(&mut self, action: &str) {
        if let Some(idx) = self.state.selected() {
            if let Some(container) = self.containers.get(idx) {
                let result = Command::new("docker")
                    .args([action, &container.id])
                    .output();
                self.last_action = Some(match result {
                    Ok(o) if o.status.success() => {
                        format!("{action} {} ✓", container.name)
                    }
                    Ok(o) => {
                        let err = String::from_utf8_lossy(&o.stderr);
                        format!("{action} failed: {}", err.lines().next().unwrap_or("error"))
                    }
                    Err(e) => format!("{action} error: {e}"),
                });
                self.refresh();
            }
        }
    }
}

impl WidgetModule for DockerWidget {
    fn name(&self) -> &str {
        "Docker"
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

        let running = self.containers.iter().filter(|c| c.running).count();
        let total = self.containers.len();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style)
            .title(format!("[ DOCKER {running}/{total} ]"))
            .title_style(title_style)
            .style(Style::default().bg(Theme::BG));

        let items: Vec<ListItem> = if self.containers.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "No containers found",
                Theme::label(),
            )))]
        } else {
            self.containers
                .iter()
                .map(|c| {
                    let (icon, state_color) = if c.running {
                        ("▶", Theme::NEON_GREEN)
                    } else {
                        ("■", Theme::RED)
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!(" {icon} "),
                            Style::default()
                                .fg(state_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{:<20} ", truncate(&c.name, 20)),
                            Theme::text(),
                        ),
                        Span::styled(
                            format!("{:<25} ", truncate(&c.image, 25)),
                            Style::default().fg(Theme::BLUE),
                        ),
                        Span::styled(
                            truncate(&c.status, 20),
                            Style::default().fg(Theme::DIM),
                        ),
                    ]))
                })
                .collect()
        };

        let mut all_items = items;
        if let Some(ref action) = self.last_action {
            all_items.push(ListItem::new(Line::from("")));
            all_items.push(ListItem::new(Line::from(Span::styled(
                format!("▸ {action}"),
                Style::default()
                    .fg(Theme::NEON_GREEN)
                    .add_modifier(Modifier::ITALIC),
            ))));
        }

        let list = List::new(all_items)
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
                if i + 1 < self.containers.len() {
                    self.state.select(Some(i + 1));
                }
                WidgetAction::None
            }
            KeyCode::Char('r') => {
                self.docker_action("restart");
                WidgetAction::None
            }
            KeyCode::Char('s') => {
                self.docker_action("stop");
                WidgetAction::None
            }
            KeyCode::Char('u') => {
                self.docker_action("start");
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
        !self.containers.is_empty()
    }

    fn status_hint(&self) -> String {
        "↑↓: select  r: restart  s: stop  u: start  Enter: refresh".into()
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max - 1])
    } else {
        s.to_string()
    }
}
