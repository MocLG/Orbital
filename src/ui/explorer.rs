use crate::ops::scanner::{self, DirEntry, Scanner, ScanState};
use crate::theme::Theme;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Clear},
    Frame,
};
use std::path::PathBuf;

/// Confirmation dialog state for delete.
#[derive(Clone, Debug, PartialEq)]
enum ConfirmState {
    None,
    Pending(PathBuf, String), // path, display name
    Error(String),
}

/// Full-screen interactive disk explorer overlay.
pub struct Explorer {
    pub active: bool,
    scanner: Scanner,
    /// Current directory being viewed.
    current_path: PathBuf,
    /// Entries in the current directory (from last completed scan).
    entries: Vec<DirEntry>,
    /// Total size of current directory.
    total_size: u64,
    /// Navigation history (for Backspace).
    history: Vec<PathBuf>,
    /// Currently selected index.
    selected: usize,
    /// Scroll offset for long lists.
    scroll: usize,
    /// Delete confirmation state.
    confirm: ConfirmState,
    /// Status message (transient).
    status_msg: Option<String>,
}

impl Explorer {
    pub fn new() -> Self {
        Self {
            active: false,
            scanner: Scanner::new(),
            current_path: PathBuf::from("/"),
            entries: Vec::new(),
            total_size: 0,
            history: Vec::new(),
            selected: 0,
            scroll: 0,
            confirm: ConfirmState::None,
            status_msg: None,
        }
    }

    /// Open the explorer at the given mount point.
    pub fn open(&mut self, path: PathBuf) {
        self.active = true;
        self.history.clear();
        self.selected = 0;
        self.scroll = 0;
        self.confirm = ConfirmState::None;
        self.status_msg = None;
        self.navigate_to(path);
    }

    pub fn close(&mut self) {
        self.active = false;
        self.entries.clear();
        self.history.clear();
        self.confirm = ConfirmState::None;
    }

    fn navigate_to(&mut self, path: PathBuf) {
        self.current_path = path.clone();
        self.selected = 0;
        self.scroll = 0;
        self.scanner.scan(path);
    }

    /// Poll for scan completion (called every tick while active).
    pub fn update(&mut self) {
        match self.scanner.get_state() {
            ScanState::Done(result) => {
                if result.path == self.current_path {
                    self.entries = result.entries;
                    self.total_size = result.total_size;
                }
            }
            ScanState::Error(e) => {
                self.status_msg = Some(e);
            }
            _ => {}
        }
    }

    /// Handle keypress. Returns true if the explorer consumed the event.
    pub fn handle_input(&mut self, event: KeyEvent) -> bool {
        if !self.active {
            return false;
        }

        // If in confirmation dialog, handle y/n
        if let ConfirmState::Pending(ref path, _) = self.confirm {
            let path = path.clone();
            match event.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    match scanner::delete_entry(&path) {
                        Ok(()) => {
                            self.status_msg = Some("Deleted successfully".into());
                            self.confirm = ConfirmState::None;
                            // Rescan
                            self.scanner.scan(self.current_path.clone());
                        }
                        Err(e) => {
                            self.confirm = ConfirmState::Error(e);
                        }
                    }
                }
                _ => {
                    // Any other key cancels
                    self.confirm = ConfirmState::None;
                }
            }
            return true;
        }

        if let ConfirmState::Error(_) = self.confirm {
            // Any key dismisses error
            self.confirm = ConfirmState::None;
            return true;
        }

        match event.code {
            KeyCode::Esc | KeyCode::Char('l') | KeyCode::Char('q') => {
                self.close();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                self.adjust_scroll();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.entries.is_empty() && self.selected + 1 < self.entries.len() {
                    self.selected += 1;
                }
                self.adjust_scroll();
            }
            KeyCode::Enter => {
                if let Some(entry) = self.entries.get(self.selected) {
                    if entry.is_dir {
                        let new_path = entry.path.clone();
                        self.history.push(self.current_path.clone());
                        self.navigate_to(new_path);
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(prev) = self.history.pop() {
                    self.navigate_to(prev);
                } else if let Some(parent) = self.current_path.parent() {
                    if parent != self.current_path {
                        let parent_path = parent.to_path_buf();
                        self.navigate_to(parent_path);
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(entry) = self.entries.get(self.selected) {
                    let path = entry.path.clone();
                    if !scanner::is_safe_to_delete(&path) {
                        self.confirm = ConfirmState::Error(format!(
                            "BLOCKED: {} is a protected system path",
                            path.display()
                        ));
                    } else {
                        let name = entry.name.clone();
                        self.confirm = ConfirmState::Pending(path, name);
                    }
                }
            }
            _ => {}
        }
        true
    }

    fn adjust_scroll(&mut self) {
        // Keep selected in view (with 2-line margin)
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }
        // We'll compute visible height dynamically in render,
        // but do a rough clamp here for large jumps.
        if self.selected >= self.scroll + 30 {
            self.scroll = self.selected.saturating_sub(29);
        }
    }

    /// Render the full-screen explorer overlay.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.active {
            return;
        }

        // Slight inset from terminal edge
        let inset = 1u16;
        let rect = Rect::new(
            area.x + inset,
            area.y + inset,
            area.width.saturating_sub(inset * 2),
            area.height.saturating_sub(inset * 2),
        );

        frame.render_widget(Clear, rect);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Theme::CYAN).add_modifier(Modifier::BOLD))
            .title("[ DISK EXPLORER ]")
            .title_style(Style::default().fg(Theme::NEON_GREEN).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(Theme::BG));

        let inner = block.inner(rect);
        frame.render_widget(block, rect);

        // Inner layout: breadcrumb (2) | entries list (fill) | footer (2)
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // breadcrumb + separator
                Constraint::Min(4),   // entries
                Constraint::Length(2), // footer
            ])
            .split(inner);

        self.render_breadcrumb(frame, layout[0]);
        self.render_entries(frame, layout[1]);
        self.render_explorer_footer(frame, layout[2]);

        // Render confirmation dialog on top if active
        match &self.confirm {
            ConfirmState::Pending(_, name) => {
                self.render_confirm_dialog(frame, rect, name);
            }
            ConfirmState::Error(msg) => {
                self.render_error_dialog(frame, rect, msg);
            }
            ConfirmState::None => {}
        }
    }

    fn render_breadcrumb(&self, frame: &mut Frame, area: Rect) {
        let components: Vec<&str> = self
            .current_path
            .components()
            .map(|c| c.as_os_str().to_str().unwrap_or("?"))
            .collect();

        let mut spans = vec![Span::styled(
            "◈ ",
            Style::default().fg(Theme::MAGENTA).add_modifier(Modifier::BOLD),
        )];

        for (i, comp) in components.iter().enumerate() {
            let label = if *comp == "/" { "Root" } else { comp };
            if i > 0 {
                spans.push(Span::styled(
                    " > ",
                    Style::default().fg(Theme::DIM),
                ));
            }
            let is_last = i == components.len() - 1;
            let style = if is_last {
                Style::default().fg(Theme::CYAN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::SOFT_WHITE)
            };
            spans.push(Span::styled(label.to_string(), style));
        }

        let scanning = matches!(self.scanner.get_state(), ScanState::Scanning(_));
        if scanning {
            spans.push(Span::styled(
                "  ◌ scanning…",
                Style::default().fg(Theme::AMBER).add_modifier(Modifier::BOLD),
            ));
        }

        let breadcrumb = Paragraph::new(Line::from(spans));
        frame.render_widget(breadcrumb, area);
    }

    fn render_entries(&self, frame: &mut Frame, area: Rect) {
        if self.entries.is_empty() {
            let scanning = matches!(self.scanner.get_state(), ScanState::Scanning(_));
            let msg = if scanning {
                "Scanning directory…"
            } else {
                "Empty directory"
            };
            let p = Paragraph::new(Span::styled(msg, Theme::label()));
            frame.render_widget(p, area);
            return;
        }

        // Each entry: 2 lines (label + gauge)
        let row_height = 2u16;
        let visible_count = (area.height / row_height) as usize;

        // Adjust scroll so selected is visible
        let mut scroll = self.scroll;
        if self.selected < scroll {
            scroll = self.selected;
        }
        if self.selected >= scroll + visible_count {
            scroll = self.selected + 1 - visible_count;
        }

        let end = (scroll + visible_count).min(self.entries.len());
        let shown = &self.entries[scroll..end];

        let constraints: Vec<Constraint> = shown
            .iter()
            .map(|_| Constraint::Length(row_height))
            .chain(std::iter::once(Constraint::Min(0)))
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        for (i, entry) in shown.iter().enumerate() {
            let abs_idx = scroll + i;
            let is_selected = abs_idx == self.selected;

            let sub = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Length(1)])
                .split(chunks[i]);

            // Row 1: icon + name + size
            let icon = if entry.is_dir { " " } else { " " };
            let icon_color = if entry.is_dir { Theme::CYAN } else { Theme::SOFT_WHITE };

            let name_style = if is_selected {
                Style::default().fg(Theme::NEON_GREEN).add_modifier(Modifier::BOLD)
            } else if entry.is_dir {
                Style::default().fg(Theme::CYAN)
            } else {
                Style::default().fg(Theme::SOFT_WHITE)
            };

            let size_str = scanner::format_size(entry.size);

            let indicator = if is_selected { "▸ " } else { "  " };
            let indicator_style = Style::default().fg(Theme::NEON_GREEN).add_modifier(Modifier::BOLD);

            let label_line = Line::from(vec![
                Span::styled(indicator, indicator_style),
                Span::styled(icon, Style::default().fg(icon_color)),
                Span::styled(&entry.name, name_style),
                Span::styled(
                    format!("  {size_str}"),
                    Style::default().fg(Theme::AMBER).add_modifier(Modifier::BOLD),
                ),
            ]);

            let label_bg = if is_selected {
                Style::default().bg(Theme::SURFACE)
            } else {
                Style::default().bg(Theme::BG)
            };
            let label_paragraph = Paragraph::new(label_line).style(label_bg);
            frame.render_widget(label_paragraph, sub[0]);

            // Row 2: proportional gauge
            let ratio = if self.total_size > 0 {
                (entry.size as f64 / self.total_size as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let bar_color = if ratio > 0.5 {
                Theme::RED
            } else if ratio > 0.2 {
                Theme::AMBER
            } else if entry.is_dir {
                Theme::CYAN
            } else {
                Theme::NEON_GREEN
            };

            let pct_label = format!("{:.1}%", ratio * 100.0);

            let gauge = Gauge::default()
                .label(Span::styled(
                    pct_label,
                    Style::default().fg(Theme::SOFT_WHITE).add_modifier(Modifier::BOLD),
                ))
                .gauge_style(Style::default().fg(bar_color).bg(Theme::SURFACE))
                .ratio(ratio);

            // Indent gauge to align with name
            let gauge_area = Rect::new(
                sub[1].x + 2,
                sub[1].y,
                sub[1].width.saturating_sub(2),
                sub[1].height,
            );
            frame.render_widget(gauge, gauge_area);
        }
    }

    fn render_explorer_footer(&self, frame: &mut Frame, area: Rect) {
        let status = self.status_msg.as_deref().unwrap_or("");
        let count = self.entries.len();
        let dir_count = self.entries.iter().filter(|e| e.is_dir).count();
        let file_count = count - dir_count;
        let total_str = scanner::format_size(self.total_size);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        // Status / info line
        let info = if !status.is_empty() {
            Line::from(Span::styled(
                format!(" {status}"),
                Style::default().fg(Theme::AMBER),
            ))
        } else {
            Line::from(vec![
                Span::styled(
                    format!("  {dir_count} dirs   {file_count} files"),
                    Style::default().fg(Theme::DIM),
                ),
                Span::styled(
                    format!("  ◈ {total_str}"),
                    Style::default().fg(Theme::CYAN).add_modifier(Modifier::BOLD),
                ),
            ])
        };
        frame.render_widget(Paragraph::new(info), rows[0]);

        // Keybinds
        let keys = Line::from(vec![
            Span::styled(" ↑↓ ", Theme::key_hint()),
            Span::styled("select  ", Theme::text()),
            Span::styled("Enter ", Theme::key_hint()),
            Span::styled("open  ", Theme::text()),
            Span::styled("Bksp ", Theme::key_hint()),
            Span::styled("back  ", Theme::text()),
            Span::styled("d ", Theme::key_hint()),
            Span::styled("delete  ", Theme::text()),
            Span::styled("Esc/q ", Theme::key_hint()),
            Span::styled("close", Theme::text()),
        ]);
        frame.render_widget(
            Paragraph::new(keys).style(Style::default().bg(Theme::SURFACE)),
            rows[1],
        );
    }

    fn render_confirm_dialog(&self, frame: &mut Frame, parent: Rect, name: &str) {
        let w = 50.min(parent.width.saturating_sub(4));
        let h = 7.min(parent.height.saturating_sub(4));
        let x = parent.x + (parent.width.saturating_sub(w)) / 2;
        let y = parent.y + (parent.height.saturating_sub(h)) / 2;
        let rect = Rect::new(x, y, w, h);

        frame.render_widget(Clear, rect);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Theme::RED).add_modifier(Modifier::BOLD))
            .title("[ CONFIRM DELETE ]")
            .title_style(Style::default().fg(Theme::RED).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(Theme::BG));

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  Delete \"{name}\"?"),
                Style::default().fg(Theme::SOFT_WHITE).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  y ", Theme::key_hint()),
                Span::styled("confirm   ", Style::default().fg(Theme::RED)),
                Span::styled("any other key ", Theme::key_hint()),
                Span::styled("cancel", Theme::text()),
            ]),
        ];

        let p = Paragraph::new(lines).block(block);
        frame.render_widget(p, rect);
    }

    fn render_error_dialog(&self, frame: &mut Frame, parent: Rect, msg: &str) {
        let w = 60.min(parent.width.saturating_sub(4));
        let h = 6.min(parent.height.saturating_sub(4));
        let x = parent.x + (parent.width.saturating_sub(w)) / 2;
        let y = parent.y + (parent.height.saturating_sub(h)) / 2;
        let rect = Rect::new(x, y, w, h);

        frame.render_widget(Clear, rect);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Theme::RED))
            .title("[ ERROR ]")
            .title_style(Style::default().fg(Theme::RED).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(Theme::BG));

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {msg}"),
                Style::default().fg(Theme::RED),
            )),
            Line::from(""),
            Line::from(Span::styled("  Press any key to dismiss", Style::default().fg(Theme::DIM))),
        ];

        let p = Paragraph::new(lines).block(block);
        frame.render_widget(p, rect);
    }
}
