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
use std::time::{Duration, Instant};

const REFRESH_INTERVAL: Duration = Duration::from_secs(30);

/// A file entry from git status, classified as staged or unstaged.
#[derive(Clone)]
struct FileEntry {
    path: String,
    display: String,
    staged: bool,
    kind: FileKind,
}

#[derive(Clone, Copy)]
enum FileKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Other,
}

struct GitInfo {
    branch: String,
    staged: Vec<FileEntry>,
    changed: Vec<FileEntry>,
    recent_commits: Vec<String>,
    last_action: Option<String>,
}

pub struct GitWidget {
    info: GitInfo,
    state: ListState,
    view_mode: ViewMode,
    last_refresh: Instant,
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
                staged: Vec::new(),
                changed: Vec::new(),
                recent_commits: Vec::new(),
                last_action: None,
            },
            state: ListState::default(),
            view_mode: ViewMode::Changes,
            last_refresh: Instant::now() - REFRESH_INTERVAL,
        }
    }

    fn refresh(&mut self) {
        // Branch
        self.info.branch = run_git(&["rev-parse", "--abbrev-ref", "HEAD"])
            .unwrap_or_else(|| "detached".into())
            .trim()
            .to_string();

        // Parse status into staged and changed
        self.info.staged.clear();
        self.info.changed.clear();

        if let Some(status) = run_git(&["status", "--porcelain"]) {
            for line in status.lines().filter(|l| l.len() >= 3) {
                let index_code = line.as_bytes()[0];
                let worktree_code = line.as_bytes()[1];
                let file_path = line[3..].to_string();

                // Staged changes (index column)
                if index_code != b' ' && index_code != b'?' {
                    let kind = match index_code {
                        b'M' => FileKind::Modified,
                        b'A' => FileKind::Added,
                        b'D' => FileKind::Deleted,
                        b'R' => FileKind::Renamed,
                        _ => FileKind::Other,
                    };
                    self.info.staged.push(FileEntry {
                        display: file_path.clone(),
                        path: file_path.clone(),
                        staged: true,
                        kind,
                    });
                }

                // Unstaged changes (worktree column)
                if worktree_code != b' ' {
                    let kind = match worktree_code {
                        b'M' => FileKind::Modified,
                        b'D' => FileKind::Deleted,
                        b'?' => FileKind::Untracked,
                        _ => FileKind::Other,
                    };
                    self.info.changed.push(FileEntry {
                        display: file_path.clone(),
                        path: file_path,
                        staged: false,
                        kind,
                    });
                }
            }
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

    /// Build the flat list of items for the Changes view.
    /// Returns: (display items for rendering, entry mapping for actions)
    fn changes_entries(&self) -> Vec<ChangesRow> {
        let mut rows = Vec::new();

        if !self.info.staged.is_empty() {
            rows.push(ChangesRow::Header(format!(
                "── Staged ({}) ──",
                self.info.staged.len()
            )));
            for entry in &self.info.staged {
                rows.push(ChangesRow::File(entry.clone()));
            }
        }

        if !self.info.changed.is_empty() {
            if !rows.is_empty() {
                rows.push(ChangesRow::Separator);
            }
            rows.push(ChangesRow::Header(format!(
                "── Changed ({}) ──",
                self.info.changed.len()
            )));
            for entry in &self.info.changed {
                rows.push(ChangesRow::File(entry.clone()));
            }
        }

        if rows.is_empty() {
            rows.push(ChangesRow::Empty);
        }

        rows
    }

    /// Get the file entry at the current selection (Changes view only).
    fn selected_file(&self) -> Option<FileEntry> {
        if self.view_mode != ViewMode::Changes {
            return None;
        }
        let idx = self.state.selected()?;
        let rows = self.changes_entries();
        match rows.get(idx) {
            Some(ChangesRow::File(entry)) => Some(entry.clone()),
            _ => None,
        }
    }

    /// Count of selectable rows in current view.
    fn row_count(&self) -> usize {
        match self.view_mode {
            ViewMode::Changes => self.changes_entries().len(),
            ViewMode::Log => self.info.recent_commits.len(),
        }
    }
}

#[derive(Clone)]
enum ChangesRow {
    Header(String),
    File(FileEntry),
    Separator,
    Empty,
}

impl WidgetModule for GitWidget {
    fn name(&self) -> &str {
        "Git"
    }

    fn init(&mut self) {
        self.refresh();
        if self.row_count() > 0 {
            self.state.select(Some(0));
        }
    }

    fn update_state(&mut self) {
        if self.last_refresh.elapsed() >= REFRESH_INTERVAL {
            self.refresh();
            self.last_refresh = Instant::now();
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool) {
        let (border_type, border_style, title_style) = if is_focused {
            (BorderType::Double, Theme::border_focused(), Theme::title_focused())
        } else {
            (BorderType::Thick, Theme::border_unfocused(), Theme::title_unfocused())
        };

        let mode_label = if self.view_mode == ViewMode::Changes { "changes" } else { "log" };
        let branch_display = format!("[ GIT {} :: {} ]", mode_label, self.info.branch);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style)
            .title(branch_display)
            .title_style(title_style)
            .style(Style::default().bg(Theme::BG));

        let items: Vec<ListItem> = match self.view_mode {
            ViewMode::Changes => {
                let rows = self.changes_entries();
                rows.iter()
                    .map(|row| match row {
                        ChangesRow::Header(text) => ListItem::new(Line::from(Span::styled(
                            text.clone(),
                            Theme::accent(),
                        ))),
                        ChangesRow::Separator => ListItem::new(Line::from("")),
                        ChangesRow::Empty => ListItem::new(Line::from(Span::styled(
                            "✓ Working tree clean",
                            Theme::good(),
                        ))),
                        ChangesRow::File(entry) => {
                            let (icon, color) = match entry.kind {
                                FileKind::Modified => ("~", Theme::AMBER),
                                FileKind::Added | FileKind::Untracked => ("+", Theme::NEON_GREEN),
                                FileKind::Deleted => ("-", Theme::RED),
                                FileKind::Renamed => ("→", Theme::BLUE),
                                FileKind::Other => ("•", Theme::SOFT_WHITE),
                            };
                            let stage_indicator = if entry.staged {
                                Span::styled("● ", Style::default().fg(Theme::NEON_GREEN))
                            } else {
                                Span::styled("○ ", Style::default().fg(Theme::DIM))
                            };
                            ListItem::new(Line::from(vec![
                                Span::raw(" "),
                                stage_indicator,
                                Span::styled(
                                    format!("{icon} "),
                                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(entry.display.clone(), Theme::text()),
                            ]))
                        }
                    })
                    .collect()
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
                let max = self.row_count();
                if i + 1 < max {
                    self.state.select(Some(i + 1));
                }
                WidgetAction::None
            }
            KeyCode::Char('l') => {
                self.view_mode = match self.view_mode {
                    ViewMode::Changes => ViewMode::Log,
                    ViewMode::Log => ViewMode::Changes,
                };
                self.state.select(Some(0));
                WidgetAction::None
            }
            KeyCode::Char('e') => {
                // Open selected file in editor
                if let Some(entry) = self.selected_file() {
                    return WidgetAction::SuspendAndEdit(entry.path);
                }
                WidgetAction::None
            }
            KeyCode::Char('a') => {
                // Stage or unstage the selected file
                if let Some(entry) = self.selected_file() {
                    if entry.staged {
                        let _ = run_git(&["reset", "HEAD", "--", &entry.path]);
                        self.info.last_action = Some(format!("Unstaged {}", entry.display));
                    } else {
                        let _ = run_git(&["add", "--", &entry.path]);
                        self.info.last_action = Some(format!("Staged {}", entry.display));
                    }
                    self.refresh();
                }
                WidgetAction::None
            }
            KeyCode::Char('c') => {
                if !self.info.staged.is_empty() {
                    // Suspend TUI and open editor for commit message
                    return WidgetAction::SuspendAndRun("git commit".into());
                } else {
                    self.info.last_action = Some("Nothing staged to commit".into());
                }
                WidgetAction::None
            }
            KeyCode::Char('p') => {
                let result = run_git(&["push"]);
                self.info.last_action = Some(
                    result
                        .map(|_| "pushed successfully".into())
                        .unwrap_or_else(|| "push failed".into()),
                );
                WidgetAction::None
            }
            KeyCode::Enter => {
                self.refresh();
                WidgetAction::None
            }
            _ => WidgetAction::None,
        }
    }

    fn status_hint(&self) -> String {
        "↑↓: select  a: stage/unstage  e: edit  l: view  c: commit  p: push".into()
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
