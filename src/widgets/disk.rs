use crate::theme::Theme;
use crate::widgets::{WidgetModule, WidgetAction};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};
use sysinfo::Disks;
use std::collections::HashSet;

struct DiskInfo {
    device: String,
    mount: String,
    total: u64,
    used: u64,
}

pub struct DiskWidget {
    disks: Vec<DiskInfo>,
    scroll: usize,
}

impl DiskWidget {
    pub fn new() -> Self {
        Self {
            disks: Vec::new(),
            scroll: 0,
        }
    }

    fn refresh(&mut self) {
        let disk_list = Disks::new_with_refreshed_list();
        let mut seen = HashSet::new();
        let mut entries: Vec<DiskInfo> = Vec::new();

        for d in disk_list.iter() {
            let total = d.total_space();
            if total == 0 {
                continue;
            }

            let device = d.name().to_string_lossy().to_string();
            if device.is_empty() {
                continue;
            }

            let mount = d.mount_point().to_string_lossy().to_string();

            // Filter out virtual/temporary/loop filesystems
            if mount.starts_with("/snap")
                || mount.starts_with("/run")
                || mount.starts_with("/sys")
                || mount.starts_with("/proc")
                || mount.starts_with("/dev")
            {
                continue;
            }
            if device.starts_with("/dev/loop") {
                continue;
            }
            let fs = d.file_system().to_string_lossy().to_string();
            if fs == "tmpfs" || fs == "devtmpfs" || fs == "squashfs" || fs == "overlay" {
                continue;
            }

            let available = d.available_space();
            let used = total.saturating_sub(available);

            // Deduplicate by device name — prefer "/" mount, otherwise first seen
            if seen.contains(&device) {
                if mount == "/" {
                    // Replace existing entry with root mount
                    if let Some(existing) = entries.iter_mut().find(|e| e.device == device) {
                        existing.mount = mount;
                        existing.total = total;
                        existing.used = used;
                    }
                }
                continue;
            }

            seen.insert(device.clone());
            entries.push(DiskInfo {
                device,
                mount,
                total,
                used,
            });
        }
        self.disks = entries;
    }
}

impl WidgetModule for DiskWidget {
    fn name(&self) -> &str {
        "Disks"
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
            .title("[ DISKS ]")
            .title_style(title_style)
            .style(Style::default().bg(Theme::BG));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.disks.is_empty() {
            let msg = Paragraph::new("No disks found").style(Theme::label());
            frame.render_widget(msg, inner);
            return;
        }

        // Each disk needs ~3 lines (label + gauge + space)
        let per_disk = 3u16;
        let visible = (inner.height / per_disk) as usize;
        let end = (self.scroll + visible).min(self.disks.len());
        let shown = &self.disks[self.scroll..end];

        let constraints: Vec<Constraint> = shown
            .iter()
            .map(|_| Constraint::Length(per_disk))
            .chain(std::iter::once(Constraint::Min(0)))
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        for (i, disk) in shown.iter().enumerate() {
            let pct = if disk.total > 0 {
                disk.used as f64 / disk.total as f64
            } else {
                0.0
            };
            let color = if pct > 0.9 {
                Theme::RED
            } else if pct > 0.7 {
                Theme::AMBER
            } else {
                Theme::NEON_GREEN
            };

            let used_gb = disk.used as f64 / 1_073_741_824.0;
            let total_gb = disk.total as f64 / 1_073_741_824.0;

            let label_line = Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{} ", disk.device),
                    Style::default()
                        .fg(Theme::CYAN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("on {}", disk.mount),
                    Style::default().fg(Theme::DIM),
                ),
            ]));
            let disk_area = chunks[i];
            let sub = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)])
                .split(disk_area);

            frame.render_widget(label_line, sub[0]);

            let gauge = Gauge::default()
                .label(Span::styled(
                    format!("{:.1}/{:.1} GB ({:.0}%)", used_gb, total_gb, pct * 100.0),
                    Style::default()
                        .fg(Theme::SOFT_WHITE)
                        .add_modifier(Modifier::BOLD),
                ))
                .gauge_style(Style::default().fg(color).bg(Theme::SURFACE))
                .ratio(pct.clamp(0.0, 1.0));
            frame.render_widget(gauge, sub[1]);
        }
    }

    fn handle_input(&mut self, event: KeyEvent) -> WidgetAction {
        match event.code {
            KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                WidgetAction::None
            }
            KeyCode::Down => {
                if self.scroll + 1 < self.disks.len() {
                    self.scroll += 1;
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
        "↑↓: scroll  Enter: refresh".into()
    }
}
