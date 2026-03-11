use crate::theme::Theme;
use crate::widgets::{WidgetAction, WidgetModule};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};

const REFRESH_INTERVAL: Duration = Duration::from_secs(30);

/// Priority-ordered list of AI CLI tools to scan for.
const AI_TOOLS: &[(&str, &str, &str)] = &[
    // (binary, display name, launch command)
    ("claude", "Claude Code", "claude"),
    ("codex", "Codex", "codex"),
    ("opencode", "OpenCode", "opencode"),
    ("copilot", "GitHub Copilot CLI", "copilot"),
    ("gh", "GitHub Copilot", "gh copilot"),
    ("gemini", "Gemini CLI", "gemini"),
];

struct DetectedTool {
    display_name: String,
    command: String,
}

pub struct AiIntelWidget {
    tool: Option<DetectedTool>,
    error_msg: Option<String>,
    last_refresh: Instant,
}

impl AiIntelWidget {
    pub fn new() -> Self {
        let tool = detect_ai_tool();
        Self {
            tool,
            error_msg: None,
            last_refresh: Instant::now(),
        }
    }
}

fn detect_ai_tool() -> Option<DetectedTool> {
    for &(binary, display_name, command) in AI_TOOLS {
        if which_exists(binary) {
            return Some(DetectedTool {
                display_name: display_name.to_string(),
                command: command.to_string(),
            });
        }
    }
    None
}

fn which_exists(binary: &str) -> bool {
    std::process::Command::new("which")
        .arg(binary)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

impl WidgetModule for AiIntelWidget {
    fn name(&self) -> &str {
        "AI Intel"
    }

    fn init(&mut self) {}

    fn update_state(&mut self) {
        if self.last_refresh.elapsed() >= REFRESH_INTERVAL {
            self.tool = detect_ai_tool();
            self.last_refresh = Instant::now();
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool) {
        let (border_type, border_style, title_style) = if is_focused {
            (
                BorderType::Double,
                Theme::border_focused(),
                Style::default()
                    .fg(Theme::VIOLET)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                BorderType::Thick,
                Theme::border_unfocused(),
                Style::default().fg(Theme::VIOLET),
            )
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style)
            .title("[ ◈ AI INTELLIGENCE ]")
            .title_style(title_style)
            .style(Style::default().bg(Theme::BG));

        let mut lines: Vec<Line> = Vec::new();

        if let Some(ref tool) = self.tool {
            lines.push(Line::from(vec![
                Span::styled(
                    " ▸ Active: ",
                    Style::default().fg(Theme::DIM),
                ),
                Span::styled(
                    &tool.display_name,
                    Style::default()
                        .fg(Theme::VIOLET)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(""));

            if let Some(ref err) = self.error_msg {
                lines.push(Line::from(Span::styled(
                    format!("  ✗ {err}"),
                    Style::default().fg(Theme::RED),
                )));
            } else if is_focused {
                lines.push(Line::from(vec![
                    Span::styled(
                        "  Press ",
                        Style::default().fg(Theme::DIM),
                    ),
                    Span::styled(
                        "'i'",
                        Style::default()
                            .fg(Theme::AMBER)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " to initiate session",
                        Style::default().fg(Theme::DIM),
                    ),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    "  ◈ STANDBY",
                    Style::default()
                        .fg(Theme::VIOLET)
                        .add_modifier(Modifier::DIM),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                " No AI tools detected",
                Style::default().fg(Theme::DIM),
            )));
        }

        let content = Paragraph::new(lines).block(block);
        frame.render_widget(content, area);
    }

    fn handle_input(&mut self, event: KeyEvent) -> WidgetAction {
        if event.code == KeyCode::Char('i') {
            if let Some(ref tool) = self.tool {
                self.error_msg = None;
                return WidgetAction::SuspendAndRun(tool.command.clone());
            }
        }
        WidgetAction::None
    }

    fn is_visible(&self) -> bool {
        self.tool.is_some()
    }

    fn status_hint(&self) -> String {
        match &self.tool {
            Some(t) => format!("AI: {} │ 'i' launch", t.display_name),
            None => String::new(),
        }
    }
}
