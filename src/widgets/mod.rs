pub mod docker;
pub mod git;
pub mod network;
pub mod ports;
pub mod processes;
pub mod system;
pub mod disk;

use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

pub trait WidgetModule {
    /// Human-friendly name for the widget
    fn name(&self) -> &str;

    /// Initialize / first data fetch
    fn init(&mut self);

    /// Background state refresh (called every tick)
    fn update_state(&mut self);

    /// Render into the given area
    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool);

    /// Handle a keypress while focused — returns true if consumed
    fn handle_input(&mut self, event: KeyEvent) -> bool;

    /// Optional status line hint for the bottom bar
    fn status_hint(&self) -> String {
        String::new()
    }
}
