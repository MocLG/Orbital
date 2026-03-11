use crate::discovery;
use crate::event::{AppEvent, EventHandler};
use crate::theme::Theme;
use crate::widgets::{WidgetAction, WidgetModule};

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Sparkline},
    Frame, Terminal,
};
use std::io::Stdout;

const HEARTBEAT_LEN: usize = 40;

pub struct App {
    pub widgets: Vec<Box<dyn WidgetModule>>,
    pub focused: usize,
    pub running: bool,
    pub show_help: bool,
    tick_count: u64,
    heartbeat: Vec<u64>,
    heartbeat_phase: f64,
}

impl App {
    pub fn new() -> Self {
        let mut heartbeat = vec![0u64; HEARTBEAT_LEN];
        for i in 0..HEARTBEAT_LEN {
            let phase = (i as f64) * std::f64::consts::PI * 2.0 / HEARTBEAT_LEN as f64;
            heartbeat[i] = ((phase.sin() + 1.0) * 4.0) as u64;
        }
        Self {
            widgets: Vec::new(),
            focused: 0,
            running: true,
            show_help: false,
            tick_count: 0,
            heartbeat,
            heartbeat_phase: 0.0,
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
        self.boot_sequence(terminal)?;

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
                                match w.handle_input(key) {
                                    WidgetAction::SuspendAndEdit(path) => {
                                        // Leave TUI, open editor, resume
                                        crossterm::terminal::disable_raw_mode()?;
                                        crossterm::execute!(
                                            std::io::stdout(),
                                            crossterm::terminal::LeaveAlternateScreen
                                        )?;
                                        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
                                        let _ = std::process::Command::new(&editor)
                                            .arg(&path)
                                            .status();
                                        crossterm::execute!(
                                            std::io::stdout(),
                                            crossterm::terminal::EnterAlternateScreen
                                        )?;
                                        crossterm::terminal::enable_raw_mode()?;
                                        terminal.clear()?;
                                    }
                                    WidgetAction::None => {}
                                }
                            }
                        }
                    }
                }
                Some(AppEvent::Tick) => {
                    self.tick_count += 1;
                    // Advance heartbeat oscilloscope
                    self.heartbeat_phase += 0.3;
                    for i in 0..HEARTBEAT_LEN {
                        let phase = self.heartbeat_phase
                            + (i as f64) * std::f64::consts::PI * 2.0 / HEARTBEAT_LEN as f64;
                        self.heartbeat[i] = ((phase.sin() + 1.0) * 4.0) as u64;
                    }
                    // Update widgets every tick — each widget throttles internally
                    for w in self.widgets.iter_mut() {
                        w.update_state();
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
                Constraint::Length(2), // footer
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
            .border_type(BorderType::Thick)
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
                apply_scanlines(frame, rect);
            }
        }
    }

    fn render_empty_state(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
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

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        // Row 1: heartbeat sparkline
        let hb_label_width = 6u16; // "PULSE "
        let hb_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(hb_label_width),
                Constraint::Min(10),
            ])
            .split(rows[0]);

        let label = Paragraph::new(Span::styled(
            "PULSE ",
            Style::default()
                .fg(Theme::MAGENTA)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(Theme::SURFACE));
        frame.render_widget(label, hb_chunks[0]);

        let spark = Sparkline::default()
            .data(&self.heartbeat)
            .style(Style::default().fg(Theme::CYAN).bg(Theme::SURFACE));
        frame.render_widget(spark, hb_chunks[1]);

        // Row 2: key hints + focused widget
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

        frame.render_widget(footer, rows[1]);
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
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Theme::CYAN))
            .title("[ HELP ]")
            .title_style(Theme::title_focused())
            .style(Style::default().bg(Theme::BG));

        let help = Paragraph::new(help_lines).block(block);
        frame.render_widget(help, rect);
    }

    fn boot_sequence(
        &self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let hex = "0123456789ABCDEF";
        let hex_bytes = hex.as_bytes();

        for frame_idx in 0..10u32 {
            terminal.draw(|frame| {
                let size = frame.area();
                let bg = Block::default().style(Style::default().bg(Theme::BG));
                frame.render_widget(bg, size);

                let mut lines: Vec<Line> = Vec::new();
                for row in 0..size.height.saturating_sub(2) {
                    let mut spans = Vec::new();
                    for col in (0..size.width).step_by(3) {
                        let seed = (row as u32)
                            .wrapping_mul(31)
                            .wrapping_add(col as u32)
                            .wrapping_mul(7)
                            .wrapping_add(frame_idx.wrapping_mul(13));
                        let h1 = hex_bytes[(seed & 0xF) as usize] as char;
                        let h2 = hex_bytes[((seed >> 4) & 0xF) as usize] as char;
                        let bright = ((seed >> 8) & 0x3) == 0;
                        let fg = if bright { Theme::CYAN } else { Theme::DIM };
                        spans.push(Span::styled(
                            format!("{h1}{h2} "),
                            Style::default().fg(fg),
                        ));
                        let _ = col; // suppress unused
                    }
                    lines.push(Line::from(spans));
                }

                // Overlay status text
                let status = if frame_idx < 6 {
                    "DECRYPTING..."
                } else {
                    "ORBITAL ONLINE"
                };
                let msg_y = size.height / 2;
                if (msg_y as usize) < lines.len() {
                    lines[msg_y as usize] = Line::from(vec![
                        Span::styled(
                            format!("{:^width$}", status, width = size.width as usize),
                            Style::default()
                                .fg(if frame_idx < 6 { Theme::MAGENTA } else { Theme::NEON_GREEN })
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]);
                }

                let para = Paragraph::new(lines);
                frame.render_widget(para, size);
            })?;
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(())
    }
}

fn apply_scanlines(frame: &mut Frame, area: Rect) {
    let buf = frame.buffer_mut();
    for y in (area.top() + 1..area.bottom().saturating_sub(1)).step_by(2) {
        for x in (area.left() + 1)..area.right().saturating_sub(1) {
            if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
                cell.set_bg(Theme::SCANLINE_BG);
            }
        }
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
        .spacing(0)
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
            .spacing(0)
            .split(row_area);

        for (c, &cell) in col_areas.iter().enumerate() {
            if idx < count {
                // Overlap shared borders by expanding into the neighbor's 1px border.
                // Clamp to the parent area so we never draw outside bounds.
                let x = if c > 0 { cell.x.saturating_sub(1).max(area.x) } else { cell.x };
                let y = if r > 0 { cell.y.saturating_sub(1).max(area.y) } else { cell.y };
                let w = cell.width + if c > 0 { 1 } else { 0 };
                let h = cell.height + if r > 0 { 1 } else { 0 };
                // Clamp right/bottom edge to parent area
                let w = w.min(area.x + area.width - x);
                let h = h.min(area.y + area.height - y);
                rects.push(Rect::new(x, y, w, h));
                idx += 1;
            }
        }
    }

    rects
}
