use ratatui::{layout::Rect, style::Color, Frame};
use crossterm::event::KeyCode;

use crate::tabs::{graph::GraphTab, search::SearchTab, placeholder::PlaceholderTab};

#[derive(Clone)]
pub struct SeverityBar {
    pub label: String,
    pub value: u64,
    pub color: Color,
}

impl SeverityBar {
    pub fn new(label: impl Into<String>, value: u64, color: Color) -> Self {
        Self { label: label.into(), value, color }
    }

    pub fn critical(value: u64) -> Self { Self::new("Critical", value, Color::Red) }
    pub fn high(value: u64) -> Self { Self::new("High", value, Color::LightRed) }
    pub fn medium(value: u64) -> Self { Self::new("Medium", value, Color::Yellow) }
    pub fn low(value: u64) -> Self { Self::new("Low", value, Color::Green) }
    pub fn info(value: u64) -> Self { Self::new("Info", value, Color::Blue) }
}

#[derive(Clone)]
pub struct GraphData {
    pub title: String,
    pub bars: Vec<SeverityBar>,
}

impl GraphData {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), bars: Vec::new() }
    }

    pub fn with_bar(mut self, bar: SeverityBar) -> Self {
        self.bars.push(bar);
        self
    }

    pub fn with_bars(mut self, bars: Vec<SeverityBar>) -> Self {
        self.bars = bars;
        self
    }

    pub fn default_severity() -> Self {
        Self::new("Severity Distribution")
            .with_bar(SeverityBar::critical(3))
            .with_bar(SeverityBar::high(7))
            .with_bar(SeverityBar::medium(12))
            .with_bar(SeverityBar::low(5))
            .with_bar(SeverityBar::info(2))
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "Critical",
            Severity::High => "High",
            Severity::Medium => "Medium",
            Severity::Low => "Low",
            Severity::Info => "Info",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Severity::Critical => Color::Red,
            Severity::High => Color::LightRed,
            Severity::Medium => Color::Yellow,
            Severity::Low => Color::Green,
            Severity::Info => Color::Blue,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Status {
    Open,
    InProgress,
    Resolved,
    FalsePositive,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Open => "Open",
            Status::InProgress => "In Progress",
            Status::Resolved => "Resolved",
            Status::FalsePositive => "False Positive",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Status::Open => Color::Red,
            Status::InProgress => Color::Yellow,
            Status::Resolved => Color::Green,
            Status::FalsePositive => Color::Gray,
        }
    }
}

#[derive(Clone)]
pub struct Finding {
    pub title: String,
    pub severity: Severity,
    pub location: String,
    pub description: String,
    pub status: Status,
}

impl Finding {
    pub fn new(
        title: impl Into<String>,
        severity: Severity,
        location: impl Into<String>,
        description: impl Into<String>,
        status: Status,
    ) -> Self {
        Self {
            title: title.into(),
            severity,
            location: location.into(),
            description: description.into(),
            status,
        }
    }

    pub fn default_findings() -> Vec<Finding> {
        vec![
            Finding::new("SQL Injection", Severity::Critical, "https://example.com/api/users?id=1", "User input is directly concatenated into SQL query without sanitization.", Status::Open),
            Finding::new("Cross-Site Scripting (XSS)", Severity::High, "https://example.com/search", "Reflected XSS vulnerability in search parameter.", Status::InProgress),
            Finding::new("Buffer Overflow", Severity::Critical, "https://example.com/upload", "Stack buffer overflow in file upload handler.", Status::Open),
            Finding::new("Authentication Bypass", Severity::Critical, "https://example.com/admin", "Admin panel accessible without authentication.", Status::Resolved),
            Finding::new("Remote Code Execution", Severity::Critical, "https://example.com/eval", "User input passed to eval() function.", Status::Open),
            Finding::new("Privilege Escalation", Severity::High, "https://example.com/api/role", "Users can modify their own role parameter.", Status::InProgress),
            Finding::new("Information Disclosure", Severity::Medium, "https://example.com/.git", "Git repository exposed to public.", Status::Open),
            Finding::new("Denial of Service", Severity::Medium, "https://example.com/api/export", "No rate limiting on resource-intensive endpoint.", Status::FalsePositive),
            Finding::new("Insecure Deserialization", Severity::High, "https://example.com/api/session", "Untrusted data deserialized without validation.", Status::Open),
            Finding::new("Path Traversal", Severity::Medium, "https://example.com/files", "File path parameter allows directory traversal.", Status::Open),
            Finding::new("CSRF Token Missing", Severity::Medium, "https://example.com/settings", "Form submission lacks CSRF protection.", Status::Open),
            Finding::new("Weak Password Policy", Severity::Low, "https://example.com/register", "No minimum password length requirement.", Status::Resolved),
            Finding::new("HTTP Only Flag Missing", Severity::Info, "https://example.com", "Session cookie missing HttpOnly flag.", Status::Open),
        ]
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum TabKind {
    Graph,
    Search,
    Placeholder,
}

pub struct App {
    current_tab: TabKind,
    pub graph_tab: GraphTab,
    pub search_tab: SearchTab,
    pub placeholder_tab: PlaceholderTab,
}

impl App {
    pub fn new(graph_data: GraphData, findings: Vec<Finding>) -> Self {
        Self {
            current_tab: TabKind::Graph,
            graph_tab: GraphTab::new(graph_data),
            search_tab: SearchTab::new(findings),
            placeholder_tab: PlaceholderTab::new(),
        }
    }

    pub fn tab_titles(&self) -> Vec<&'static str> {
        vec!["Graph", "Search", "Placeholder"]
    }

    pub fn current_tab_index(&self) -> usize {
        match self.current_tab {
            TabKind::Graph => 0,
            TabKind::Search => 1,
            TabKind::Placeholder => 2,
        }
    }

    pub fn select_tab(&mut self, index: usize) {
        self.search_tab.unfocus();
        self.current_tab = match index {
            0 => TabKind::Graph,
            1 => TabKind::Search,
            _ => TabKind::Placeholder,
        };
    }

    pub fn next_tab(&mut self) {
        self.search_tab.unfocus();
        self.current_tab = match self.current_tab {
            TabKind::Graph => TabKind::Search,
            TabKind::Search => TabKind::Placeholder,
            TabKind::Placeholder => TabKind::Graph,
        };
    }

    pub fn render_current_tab(&mut self, f: &mut Frame, area: Rect) {
        match self.current_tab {
            TabKind::Graph => self.graph_tab.render(f, area),
            TabKind::Search => self.search_tab.render(f, area),
            TabKind::Placeholder => self.placeholder_tab.render(f, area),
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        if self.current_tab == TabKind::Search {
            if self.search_tab.is_focused() || self.search_tab.is_dropdown_open() {
                return self.search_tab.handle_key(key);
            }
        }

        match key {
            KeyCode::Char('t') | KeyCode::Tab => {
                self.next_tab();
                true
            }
            KeyCode::Char('s') if self.current_tab == TabKind::Search => {
                self.search_tab.focus_search();
                true
            }
            KeyCode::Char('f') if self.current_tab == TabKind::Search => {
                self.search_tab.toggle_dropdown();
                true
            }
            KeyCode::Down if self.current_tab == TabKind::Search => {
                self.search_tab.list_next();
                true
            }
            KeyCode::Up if self.current_tab == TabKind::Search => {
                self.search_tab.list_previous();
                true
            }
            _ => false,
        }
    }

    pub fn handle_click(&mut self, col: u16, row: u16) {
        match self.current_tab {
            TabKind::Search => self.search_tab.handle_click(col, row),
            TabKind::Placeholder => self.placeholder_tab.handle_click(col, row),
            _ => {}
        }
    }

    pub fn handle_scroll_down(&mut self) {
        if self.current_tab == TabKind::Search {
            self.search_tab.list_next();
        }
    }

    pub fn handle_scroll_up(&mut self) {
        if self.current_tab == TabKind::Search {
            self.search_tab.list_previous();
        }
    }
}