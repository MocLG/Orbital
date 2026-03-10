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
use std::path::Path;

const CONFIG_FILES: &[&str] = &[
    ".env",
    ".env.local",
    "Cargo.toml",
    "package.json",
    "docker-compose.yml",
    "docker-compose.yaml",
    "Makefile",
    "Dockerfile",
    ".gitignore",
    "tsconfig.json",
    "pyproject.toml",
    "go.mod",
];

pub struct VaultWidget {
    files: Vec<String>,
    state: ListState,
}

impl VaultWidget {
    pub fn new() -> Self {
        let files: Vec<String> = CONFIG_FILES
            .iter()
            .filter(|f| Path::new(f).exists())
            .map(|f| f.to_string())
            .collect();

        Self {
            files,
            state: ListState::default(),
        }
    }

    pub fn has_files(&self) -> bool {
        !self.files.is_empty()
    }
}

impl WidgetModule for VaultWidget {
    fn name(&self) -> &str {
        "Vault"
    }

    fn init(&mut self) {
        if !self.files.is_empty() {
            self.state.select(Some(0));
        }
    }

    fn update_state(&mut self) {}

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
            .title(format!(" ◈ Vault [{}] ", self.files.len()))
            .title_style(title_style)
            .style(Style::default().bg(Theme::BG));

        let items: Vec<ListItem> = self
            .files
            .iter()
            .map(|f| {
                let icon = if f.ends_with(".toml") || f.ends_with(".json") || f.ends_with(".yaml") || f.ends_with(".yml") {
                    "⚙"
                } else if f.starts_with(".env") {
                    "🔑"
                } else if f.starts_with("Dockerfile") || f.starts_with("docker-compose") {
                    "🐳"
                } else if f == "Makefile" {
                    "⚒"
                } else {
                    "📄"
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {icon} "),
                        Style::default().fg(Theme::CYAN),
                    ),
                    Span::styled(f.clone(), Theme::text()),
                ]))
            })
            .collect();

        let mut all_items = items;
        if is_focused {
            all_items.push(ListItem::new(Line::from("")));
            all_items.push(ListItem::new(Line::from(Span::styled(
                " Press 'e' to edit in $EDITOR",
                Style::default()
                    .fg(Theme::DIM)
                    .add_modifier(Modifier::ITALIC),
            ))));
        }

        let list = List::new(all_items)
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
                if i + 1 < self.files.len() {
                    self.state.select(Some(i + 1));
                }
                WidgetAction::None
            }
            KeyCode::Char('e') => {
                if let Some(idx) = self.state.selected() {
                    if let Some(file) = self.files.get(idx) {
                        return WidgetAction::SuspendAndEdit(file.clone());
                    }
                }
                WidgetAction::None
            }
            _ => WidgetAction::None,
        }
    }

    fn status_hint(&self) -> String {
        "↑↓: select  e: edit in $EDITOR".into()
    }
}
