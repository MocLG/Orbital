use crate::discovery;
use crate::event::{AppEvent, EventHandler};
use crate::theme::Theme;
use crate::widgets::WidgetModule;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};
use std::io::Stdout;

pub struct App {
    pub widgets: Vec<Box<dyn WidgetModule>>,
    pub focused: usize,
    pub running: bool,
    pub show_help: bool,
    tick_count: u64,
}

impl App {
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
            focused: 0,
            running: true,
            show_help: false,
            tick_count: 0,
        }
    }

    pub fn discover_widgets(&mut self) {
        self.widgets = discovery::discover();
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        mut events: EventHandler,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while self.running {
            terminal.draw(|frame| self.draw(frame))?;

            match events.next().await {
                Some(AppEvent::Key(key)) => {
                    // Global keys first
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Char('q'), _) => {
                            self.running = false;
                        }
                        (KeyCode::Char('?'), _) => {
                            self.show_help = !self.show_help;
                        }
                        (KeyCode::Tab, _) | (KeyCode::Right, _) => {
                            if !self.widgets.is_empty() {
                                // Skip hidden widgets when navigating
                                let len = self.widgets.len();
                                let mut next = (self.focused + 1) % len;
                                let start = next;
                                loop {
                                    if self.widgets[next].is_visible() {
                                        break;
                                    }
                                    next = (next + 1) % len;
                                    if next == start {
                                        break;
                                    }
                                }
                                self.focused = next;
                            }
                        }
                        (KeyCode::BackTab, _) | (KeyCode::Left, _) => {
                            if !self.widgets.is_empty() {
                                let len = self.widgets.len();
                                let mut prev = if self.focused == 0 { len - 1 } else { self.focused - 1 };
                                let start = prev;
                                loop {
                                    if self.widgets[prev].is_visible() {
                                        break;
                                    }
                                    prev = if prev == 0 { len - 1 } else { prev - 1 };
                                    if prev == start {
                                        break;
                                    }
                                }
                                self.focused = prev;
                            }
                        }
                        _ => {
                            // Route to focused widget
                            if let Some(w) = self.widgets.get_mut(self.focused) {
                                w.handle_input(key);
                            }
                        }
                    }
                }
                Some(AppEvent::Tick) => {
                    self.tick_count += 1;
                    // Update every 4 ticks (~1s at 250ms tick)
                    if self.tick_count % 4 == 0 {
                        for w in self.widgets.iter_mut() {
                            w.update_state();
                        }
                    }
                }
                None => {
                    self.running = false;
                }
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let size = frame.area();

        // Background fill
        let bg_block = Block::default().style(Style::default().bg(Theme::BG));
        frame.render_widget(bg_block, size);

        // Main layout: header + body + footer
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // header
                Constraint::Min(10),   // body
                Constraint::Length(1), // footer
            ])
            .split(size);

        self.render_header(frame, outer[0]);
        self.render_body(frame, outer[1]);
        self.render_footer(frame, outer[2]);

        if self.show_help {
            self.render_help_overlay(frame, size);
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let active = self.widgets.iter().filter(|w| w.is_visible()).count();
        let title_text = vec![Line::from(vec![
            Span::styled("◈ ", Style::default().fg(Theme::MAGENTA)),
            Span::styled(
                "O R B I T A L",
                Style::default()
                    .fg(Theme::CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ◈", Style::default().fg(Theme::MAGENTA)),
            Span::raw("  "),
            Span::styled(
                format!("▸ {active} modules active"),
                Style::default().fg(Theme::DIM),
            ),
        ])];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::MAGENTA))
            .style(Style::default().bg(Theme::BG));

        let header = Paragraph::new(title_text).block(block);
        frame.render_widget(header, area);
    }

    fn render_body(&self, frame: &mut Frame, area: Rect) {
        // Collect only visible widgets (with their original indices for focus tracking)
        let visible: Vec<(usize, &Box<dyn WidgetModule>)> = self
            .widgets
            .iter()
            .enumerate()
            .filter(|(_, w)| w.is_visible())
            .collect();

        if visible.is_empty() {
            self.render_empty_state(frame, area);
            return;
        }

        let count = visible.len();
        let areas = compute_grid(area, count);

        for (vi, (orig_idx, widget)) in visible.iter().enumerate() {
            if let Some(&rect) = areas.get(vi) {
                widget.render(frame, rect, *orig_idx == self.focused);
            }
        }
    }

    fn render_empty_state(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::DIM))
            .style(Style::default().bg(Theme::BG));

        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "◈ CYBERDECK OFFLINE ◈",
                Style::default()
                    .fg(Theme::MAGENTA)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "No active modules detected. Scan for targets?",
                Style::default().fg(Theme::DIM),
            )),
        ])
        .centered()
        .block(block);

        frame.render_widget(msg, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let focused_widget = self.widgets.get(self.focused);
        let hint = focused_widget
            .map(|w| w.status_hint())
            .unwrap_or_default();

        let focused_name = focused_widget
            .map(|w| {
                let vis = if w.is_visible() { "" } else { " (hidden)" };
                format!("{}{vis}", w.name())
            })
            .unwrap_or_default();

        let footer = Paragraph::new(Line::from(vec![
            Span::styled(" ◂ Tab ▸ ", Theme::key_hint()),
            Span::styled("navigate  ", Theme::text()),
            Span::styled("? ", Theme::key_hint()),
            Span::styled("help  ", Theme::text()),
            Span::styled("q ", Theme::key_hint()),
            Span::styled("quit  ", Theme::text()),
            Span::styled("│ ", Style::default().fg(Theme::DIM)),
            Span::styled(
                format!("▸ {focused_name}"),
                Style::default()
                    .fg(Theme::CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(hint, Style::default().fg(Theme::DIM)),
        ]))
        .style(Style::default().bg(Theme::SURFACE));

        frame.render_widget(footer, area);
    }

    fn render_help_overlay(&self, frame: &mut Frame, area: Rect) {
        let overlay_width = 50.min(area.width.saturating_sub(4));
        let overlay_height = 16.min(area.height.saturating_sub(4));
        let x = (area.width.saturating_sub(overlay_width)) / 2;
        let y = (area.height.saturating_sub(overlay_height)) / 2;
        let rect = Rect::new(x, y, overlay_width, overlay_height);

        let help_lines = vec![
            Line::from(Span::styled(
                "◈ ORBITAL — KEYBINDINGS ◈",
                Style::default()
                    .fg(Theme::CYAN)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Tab / → ", Theme::key_hint()),
                Span::styled("  Next widget", Theme::text()),
            ]),
            Line::from(vec![
                Span::styled(" S-Tab / ← ", Theme::key_hint()),
                Span::styled("Prev widget", Theme::text()),
            ]),
            Line::from(vec![
                Span::styled(" ↑ / ↓ ", Theme::key_hint()),
                Span::styled("    Scroll within widget", Theme::text()),
            ]),
            Line::from(vec![
                Span::styled(" q / C-c ", Theme::key_hint()),
                Span::styled("  Quit", Theme::text()),
            ]),
            Line::from(""),
            Line::from(Span::styled("── Widget Actions ──", Theme::accent())),
            Line::from(vec![
                Span::styled(" k ", Theme::key_hint()),
                Span::styled("Kill process (Processes)", Theme::text()),
            ]),
            Line::from(vec![
                Span::styled(" c / p ", Theme::key_hint()),
                Span::styled("Commit / Push (Git)", Theme::text()),
            ]),
            Line::from(vec![
                Span::styled(" r / s ", Theme::key_hint()),
                Span::styled("Restart / Stop (Docker)", Theme::text()),
            ]),
            Line::from(vec![
                Span::styled(" Enter ", Theme::key_hint()),
                Span::styled("Refresh / Select", Theme::text()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press ? to close",
                Style::default().fg(Theme::DIM),
            )),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::CYAN))
            .title(" Help ")
            .title_style(Theme::title_focused())
            .style(Style::default().bg(Theme::BG));

        let help = Paragraph::new(help_lines).block(block);
        frame.render_widget(help, rect);
    }
}

/// Compute an adaptive grid of Rects for `count` visible widgets.
/// Uses Constraint::Min(20) so widgets never get squashed below usable size.
fn compute_grid(area: Rect, count: usize) -> Vec<Rect> {
    if count == 0 {
        return vec![];
    }

    // Determine ideal columns based on available width
    let min_col_width: u16 = 20;
    let max_cols_by_width = (area.width / min_col_width).max(1) as usize;

    let cols = match count {
        1 => 1,
        2 => 2.min(max_cols_by_width),
        3 => 3.min(max_cols_by_width),
        4 => 2.min(max_cols_by_width),
        5..=6 => 3.min(max_cols_by_width),
        _ => 3.min(max_cols_by_width),
    };

    let full_rows = count / cols;
    let remainder = count % cols;
    let rows = if remainder > 0 { full_rows + 1 } else { full_rows };

    let min_row_height: u16 = 6;
    let row_constraints: Vec<Constraint> = (0..rows)
        .map(|_| Constraint::Min(min_row_height))
        .collect();
    let row_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    let mut rects = Vec::with_capacity(count);
    let mut idx = 0;

    for (r, &row_area) in row_areas.iter().enumerate() {
        let items_in_row = if r < full_rows {
            cols
        } else {
            remainder
        };
        if items_in_row == 0 {
            break;
        }
        let col_constraints: Vec<Constraint> = (0..items_in_row)
            .map(|_| Constraint::Min(min_col_width))
            .collect();
        let col_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints)
            .split(row_area);

        for &cell in col_areas.iter() {
            if idx < count {
                // Collapse shared borders: remove top border for rows > 0,
                // remove left border for cols > 0 within the row.
                let col_in_row = idx - (r * cols);
                let adjusted = if r > 0 && col_in_row > 0 {
                    Rect::new(
                        cell.x.saturating_sub(1),
                        cell.y.saturating_sub(1),
                        cell.width + 1,
                        cell.height + 1,
                    )
                } else if r > 0 {
                    Rect::new(cell.x, cell.y.saturating_sub(1), cell.width, cell.height + 1)
                } else if col_in_row > 0 {
                    Rect::new(
                        cell.x.saturating_sub(1),
                        cell.y,
                        cell.width + 1,
                        cell.height,
                    )
                } else {
                    cell
                };
                rects.push(adjusted);
                idx += 1;
            }
        }
    }

    rects
}
