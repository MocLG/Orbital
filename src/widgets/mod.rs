pub mod docker;
pub mod git;
pub mod network;
pub mod ports;
pub mod processes;
pub mod spectre;
pub mod system;
pub mod disk;
pub mod vault;

use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

pub enum WidgetAction {
    None,
    SuspendAndEdit(String),
}

pub trait WidgetModule {
    /// Human-friendly name for the widget
    fn name(&self) -> &str;

    /// Initialize / first data fetch
    fn init(&mut self);

    /// Background state refresh (called every tick)
    fn update_state(&mut self);

    /// Render into the given area
    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool);

    /// Handle a keypress while focused — returns a WidgetAction
    fn handle_input(&mut self, event: KeyEvent) -> WidgetAction;

    /// Whether this widget has meaningful data to show.
    /// Returning false hides it from the grid entirely.
    fn is_visible(&self) -> bool {
        true
    }

    /// Optional status line hint for the bottom bar
    fn status_hint(&self) -> String {
        String::new()
    }
}
