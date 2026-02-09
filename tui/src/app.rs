use crossterm::event::KeyCode;
use models::{Finding, GraphData};
use ratatui::{layout::Rect, Frame};

use crate::tabs::Tab;
use crate::tabs::graph::GraphTab;
use crate::tabs::placeholder::PlaceholderTab;
use crate::tabs::search::SearchTab;

/// Top-level application state — owns all tabs and routes input/rendering.
pub struct App {
    tabs: Vec<Box<dyn Tab>>,
    current: usize,
}

impl App {
    pub fn new(graph_data: GraphData, findings: Vec<Finding>) -> Self {
        let tabs: Vec<Box<dyn Tab>> = vec![
            Box::new(GraphTab::new(graph_data, findings.clone())),
            Box::new(SearchTab::new(findings)),
            Box::new(PlaceholderTab::new()),
        ];
        Self { tabs, current: 0 }
    }

    pub fn tab_titles(&self) -> Vec<&'static str> {
        self.tabs.iter().map(|t| t.title()).collect()
    }

    pub fn current_tab_index(&self) -> usize {
        self.current
    }

    pub fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs[self.current].on_blur();
            self.current = index;
        }
    }

    pub fn next_tab(&mut self) {
        self.tabs[self.current].on_blur();
        self.current = (self.current + 1) % self.tabs.len();
    }

    pub fn render_current_tab(&mut self, f: &mut Frame, area: Rect) {
        self.tabs[self.current].render(f, area);
    }

    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        // Let the active tab try to consume the key first.
        if self.tabs[self.current].handle_key(key) {
            return true;
        }

        // Global key bindings.
        match key {
            KeyCode::Char('t') | KeyCode::Tab => {
                self.next_tab();
                true
            }
            _ => false,
        }
    }

    pub fn handle_click(&mut self, col: u16, row: u16) {
        self.tabs[self.current].handle_click(col, row);
    }

    pub fn handle_scroll_down(&mut self) {
        self.tabs[self.current].handle_scroll_down();
    }

    pub fn handle_scroll_up(&mut self) {
        self.tabs[self.current].handle_scroll_up();
    }
}
