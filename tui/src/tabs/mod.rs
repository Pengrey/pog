pub mod assets;
pub mod graph;
pub mod search;

use crossterm::event::KeyCode;
use ratatui::{layout::Rect, Frame};

/// Common interface for all TUI tabs.
pub trait Tab {
    /// Human-readable title shown in the tab bar.
    fn title(&self) -> &'static str;

    /// Render this tab into the given area.
    fn render(&mut self, f: &mut Frame, area: Rect);

    /// Handle a key press. Return `true` if the key was consumed.
    fn handle_key(&mut self, _key: KeyCode) -> bool { false }

    /// Handle a mouse click at (`col`, `row`). Default is no-op.
    fn handle_click(&mut self, _col: u16, _row: u16) {}

    /// Handle scroll-down. Default is no-op.
    fn handle_scroll_down(&mut self) {}

    /// Handle scroll-up. Default is no-op.
    fn handle_scroll_up(&mut self) {}

    /// Called when the tab loses focus (e.g. user switches away).
    fn on_blur(&mut self) {}
}
