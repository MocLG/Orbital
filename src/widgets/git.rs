use crate::theme::Theme;
use crate::widgets::WidgetModule;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};
use std::process::Command;

struct GitInfo {
    branch: String,
    changes: Vec<String>,
    recent_commits: Vec<String>,
    last_action: Option<String>,
}

pub struct GitWidget {
    info: GitInfo,
    state: ListState,
    view_mode: ViewMode,
}

#[derive(PartialEq)]
enum ViewMode {
    Changes,
    Log,
}

impl GitWidget {
    pub fn new() -> Self {
        Self {
            info: GitInfo {
                branch: String::new(),
                changes: Vec::new(),
                recent_commits: Vec::new(),
                last_action: None,
            },
            state: ListState::default(),
            view_mode: ViewMode::Changes,
        }
    }

    fn refresh(&mut self) {
        // Branch
        self.info.branch = run_git(&["rev-parse", "--abbrev-ref", "HEAD"])
            .unwrap_or_else(|| "detached".into())
            .trim()
            .to_string();

        // Status (short)
        if let Some(status) = run_git(&["status", "--porcelain"]) {
            self.info.changes = status
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect();
        }

        // Recent commits
        if let Some(log) = run_git(&[
            "log",
            "--oneline",
            "--no-decorate",
            "-10",
        ]) {
            self.info.recent_commits = log
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect();
        }
    }
}

impl WidgetModule for GitWidget {
    fn name(&self) -> &str {
        "Git"
    }

    fn init(&mut self) {
        self.refresh();
        if !self.info.changes.is_empty() || !self.info.recent_commits.is_empty() {
            self.state.select(Some(0));
        }
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

        let branch_display = format!(" ◈ Git [{} ← {}] ", 
            if self.view_mode == ViewMode::Changes { "changes" } else { "log" },
            self.info.branch
        );

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(branch_display)
            .title_style(title_style)
            .style(Style::default().bg(Theme::BG));

        let items: Vec<ListItem> = match self.view_mode {
            ViewMode::Changes => {
                if self.info.changes.is_empty() {
                    vec![ListItem::new(Line::from(Span::styled(
                        "✓ Working tree clean",
                        Theme::good(),
                    )))]
                } else {
                    self.info
                        .changes
                        .iter()
                        .map(|c| {
                            let (indicator, color) = if c.starts_with(" M") || c.starts_with("M ") {
                                ("~", Theme::AMBER)
                            } else if c.starts_with("??") {
                                ("+", Theme::NEON_GREEN)
                            } else if c.starts_with(" D") || c.starts_with("D ") {
                                ("-", Theme::RED)
                            } else {
                                ("•", Theme::SOFT_WHITE)
                            };
                            ListItem::new(Line::from(vec![
                                Span::styled(
                                    format!(" {indicator} "),
                                    Style::default()
                                        .fg(color)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    c.get(3..).unwrap_or(c).to_string(),
                                    Theme::text(),
                                ),
                            ]))
                        })
                        .collect()
                }
            }
            ViewMode::Log => self
                .info
                .recent_commits
                .iter()
                .map(|c| {
                    let (hash, msg) = c.split_at(c.find(' ').unwrap_or(c.len()).min(8));
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{hash} "),
                            Style::default().fg(Theme::MAGENTA),
                        ),
                        Span::styled(msg.trim().to_string(), Theme::text()),
                    ]))
                })
                .collect(),
        };

        // Action feedback
        let mut all_items = items;
        if let Some(ref action) = self.info.last_action {
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

    fn handle_input(&mut self, event: KeyEvent) -> bool {
        match event.code {
            KeyCode::Up => {
                let i = self.state.selected().unwrap_or(0);
                if i > 0 {
                    self.state.select(Some(i - 1));
                }
                true
            }
            KeyCode::Down => {
                let i = self.state.selected().unwrap_or(0);
                let max = match self.view_mode {
                    ViewMode::Changes => self.info.changes.len(),
                    ViewMode::Log => self.info.recent_commits.len(),
                };
                if i + 1 < max {
                    self.state.select(Some(i + 1));
                }
                true
            }
            KeyCode::Char('l') => {
                self.view_mode = match self.view_mode {
                    ViewMode::Changes => ViewMode::Log,
                    ViewMode::Log => ViewMode::Changes,
                };
                self.state.select(Some(0));
                true
            }
            KeyCode::Char('c') => {
                if !self.info.changes.is_empty() {
                    let _ = run_git(&["add", "-A"]);
                    let result = run_git(&["commit", "-m", "chore: quick commit from Orbital"]);
                    self.info.last_action = Some(
                        result
                            .map(|o| o.lines().next().unwrap_or("committed").to_string())
                            .unwrap_or_else(|| "commit failed".into()),
                    );
                    self.refresh();
                }
                true
            }
            KeyCode::Char('p') => {
                let result = run_git(&["push"]);
                self.info.last_action = Some(
                    result
                        .map(|_| "pushed successfully".into())
                        .unwrap_or_else(|| "push failed".into()),
                );
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
        "↑↓: select  l: toggle view  c: commit all  p: push  Enter: refresh".into()
    }
}

fn run_git(args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
}
