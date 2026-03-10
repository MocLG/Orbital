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
use sysinfo::System;

struct ProcInfo {
    pid: u32,
    name: String,
    cpu: f32,
    mem_mb: f64,
}

pub struct ProcessesWidget {
    sys: System,
    procs: Vec<ProcInfo>,
    state: ListState,
    last_action: Option<String>,
}

impl ProcessesWidget {
    pub fn new() -> Self {
        Self {
            sys: System::new(),
            procs: Vec::new(),
            state: ListState::default(),
            last_action: None,
        }
    }

    fn refresh(&mut self) {
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let mut procs: Vec<ProcInfo> = self
            .sys
            .processes()
            .values()
            .map(|p| ProcInfo {
                pid: p.pid().as_u32(),
                name: p.name().to_string_lossy().to_string(),
                cpu: p.cpu_usage(),
                mem_mb: p.memory() as f64 / 1_048_576.0,
            })
            .collect();

        procs.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
        procs.truncate(20);
        self.procs = procs;

        if self.state.selected().is_none() && !self.procs.is_empty() {
            self.state.select(Some(0));
        }
    }
}

impl WidgetModule for ProcessesWidget {
    fn name(&self) -> &str {
        "Processes"
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
            .title("[ PROCESSES ]")
            .title_style(title_style)
            .style(Style::default().bg(Theme::BG));

        let items: Vec<ListItem> = self
            .procs
            .iter()
            .map(|p| {
                let cpu_color = if p.cpu > 50.0 {
                    Theme::RED
                } else if p.cpu > 20.0 {
                    Theme::AMBER
                } else {
                    Theme::NEON_GREEN
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:>6} ", p.pid),
                        Style::default().fg(Theme::DIM),
                    ),
                    Span::styled(
                        format!("{:<20} ", truncate_str(&p.name, 20)),
                        Theme::text(),
                    ),
                    Span::styled(
                        format!("{:>5.1}% ", p.cpu),
                        Style::default()
                            .fg(cpu_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:.0}M", p.mem_mb),
                        Style::default().fg(Theme::BLUE),
                    ),
                ]))
            })
            .collect();

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
                if i + 1 < self.procs.len() {
                    self.state.select(Some(i + 1));
                }
                WidgetAction::None
            }
            KeyCode::Char('k') => {
                if let Some(idx) = self.state.selected() {
                    if let Some(proc) = self.procs.get(idx) {
                        if let Some(p) = self.sys.process(sysinfo::Pid::from_u32(proc.pid)) {
                            p.kill();
                            self.last_action =
                                Some(format!("Sent SIGTERM to {} ({})", proc.name, proc.pid));
                        }
                    }
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

    fn status_hint(&self) -> String {
        let base = "↑↓: select  k: kill  Enter: refresh";
        if let Some(ref action) = self.last_action {
            format!("{base}  │ {action}")
        } else {
            base.into()
        }
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max - 1])
    } else {
        s.to_string()
    }
}
