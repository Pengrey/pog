use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{Finding, Severity};

#[derive(Clone, Copy, PartialEq)]
pub enum SeverityFilter {
    All,
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl SeverityFilter {
    pub fn all_options() -> Vec<SeverityFilter> {
        vec![
            SeverityFilter::All,
            SeverityFilter::Critical,
            SeverityFilter::High,
            SeverityFilter::Medium,
            SeverityFilter::Low,
            SeverityFilter::Info,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SeverityFilter::All => "All",
            SeverityFilter::Critical => "Critical",
            SeverityFilter::High => "High",
            SeverityFilter::Medium => "Medium",
            SeverityFilter::Low => "Low",
            SeverityFilter::Info => "Info",
        }
    }

    pub fn matches(&self, severity: Severity) -> bool {
        match self {
            SeverityFilter::All => true,
            SeverityFilter::Critical => severity == Severity::Critical,
            SeverityFilter::High => severity == Severity::High,
            SeverityFilter::Medium => severity == Severity::Medium,
            SeverityFilter::Low => severity == Severity::Low,
            SeverityFilter::Info => severity == Severity::Info,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            SeverityFilter::All => Color::White,
            SeverityFilter::Critical => Color::Red,
            SeverityFilter::High => Color::LightRed,
            SeverityFilter::Medium => Color::Yellow,
            SeverityFilter::Low => Color::Green,
            SeverityFilter::Info => Color::Blue,
        }
    }
}

pub struct SearchTab {
    search_input: String,
    search_focused: bool,
    severity_filter: SeverityFilter,
    dropdown_open: bool,
    dropdown_selected: usize,
    items: Vec<Finding>,
    filtered_items: Vec<Finding>,
    list_state: ListState,
    search_area: Option<Rect>,
    dropdown_button_area: Option<Rect>,
    dropdown_menu_area: Option<Rect>,
    list_area: Option<Rect>,
}

impl SearchTab {
    pub fn new(items: Vec<Finding>) -> Self {
        let filtered_items = items.clone();
        let mut list_state = ListState::default();
        if !filtered_items.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            search_input: String::new(),
            search_focused: false,
            severity_filter: SeverityFilter::All,
            dropdown_open: false,
            dropdown_selected: 0,
            items,
            filtered_items,
            list_state,
            search_area: None,
            dropdown_button_area: None,
            dropdown_menu_area: None,
            list_area: None,
        }
    }

    pub fn is_focused(&self) -> bool {
        self.search_focused
    }

    pub fn is_dropdown_open(&self) -> bool {
        self.dropdown_open
    }

    pub fn focus_search(&mut self) {
        self.search_focused = true;
        self.dropdown_open = false;
    }

    pub fn unfocus(&mut self) {
        self.search_focused = false;
        self.dropdown_open = false;
    }

    pub fn toggle_dropdown(&mut self) {
        self.dropdown_open = !self.dropdown_open;
        self.search_focused = false;
        if self.dropdown_open {
            self.dropdown_selected = SeverityFilter::all_options()
                .iter()
                .position(|&f| f == self.severity_filter)
                .unwrap_or(0);
        }
    }

    pub fn dropdown_next(&mut self) {
        let options = SeverityFilter::all_options();
        self.dropdown_selected = (self.dropdown_selected + 1) % options.len();
    }

    pub fn dropdown_previous(&mut self) {
        let options = SeverityFilter::all_options();
        self.dropdown_selected = if self.dropdown_selected == 0 {
            options.len() - 1
        } else {
            self.dropdown_selected - 1
        };
    }

    pub fn dropdown_select(&mut self) {
        let options = SeverityFilter::all_options();
        if let Some(&filter) = options.get(self.dropdown_selected) {
            self.severity_filter = filter;
            self.dropdown_open = false;
            self.filter_items();
        }
    }

    pub fn filter_items(&mut self) {
        let search_lower = self.search_input.to_lowercase();
        self.filtered_items = self.items
            .iter()
            .filter(|item| {
                let matches_search = search_lower.is_empty()
                    || item.title.to_lowercase().contains(&search_lower)
                    || item.description.to_lowercase().contains(&search_lower)
                    || item.location.to_lowercase().contains(&search_lower);
                let matches_severity = self.severity_filter.matches(item.severity);
                matches_search && matches_severity
            })
            .cloned()
            .collect();

        if self.filtered_items.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    pub fn list_next(&mut self) {
        if self.filtered_items.is_empty() { return; }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % self.filtered_items.len(),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn list_previous(&mut self) {
        if self.filtered_items.is_empty() { return; }
        let i = match self.list_state.selected() {
            Some(i) => if i == 0 { self.filtered_items.len() - 1 } else { i - 1 },
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn get_selected(&self) -> Option<&Finding> {
        self.list_state.selected().and_then(|i| self.filtered_items.get(i))
    }

    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        if self.dropdown_open {
            match key {
                KeyCode::Esc => {
                    self.dropdown_open = false;
                    true
                }
                KeyCode::Enter => {
                    self.dropdown_select();
                    true
                }
                KeyCode::Down => {
                    self.dropdown_next();
                    true
                }
                KeyCode::Up => {
                    self.dropdown_previous();
                    true
                }
                _ => false,
            }
        } else if self.search_focused {
            match key {
                KeyCode::Esc | KeyCode::Enter => {
                    self.search_focused = false;
                    true
                }
                KeyCode::Char(c) => {
                    self.search_input.push(c);
                    self.filter_items();
                    true
                }
                KeyCode::Backspace => {
                    self.search_input.pop();
                    self.filter_items();
                    true
                }
                KeyCode::Down => {
                    self.list_next();
                    true
                }
                KeyCode::Up => {
                    self.list_previous();
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn handle_click(&mut self, col: u16, row: u16) {
        if self.dropdown_open {
            if let Some(area) = self.dropdown_menu_area {
                if Self::in_area(col, row, area) {
                    let menu_start_y = area.y + 1;
                    if row >= menu_start_y {
                        let clicked_index = (row - menu_start_y) as usize;
                        let options = SeverityFilter::all_options();
                        if clicked_index < options.len() {
                            self.dropdown_selected = clicked_index;
                            self.dropdown_select();
                        }
                    }
                    return;
                }
            }
            self.dropdown_open = false;
        }

        if let Some(area) = self.dropdown_button_area {
            if Self::in_area(col, row, area) {
                self.toggle_dropdown();
                return;
            }
        }

        if let Some(area) = self.search_area {
            if Self::in_area(col, row, area) {
                self.search_focused = true;
                self.dropdown_open = false;
                return;
            }
        }

        if let Some(area) = self.list_area {
            if Self::in_area(col, row, area) {
                let list_start_y = area.y + 1;
                if row >= list_start_y {
                    let clicked_index = (row - list_start_y) as usize;
                    if clicked_index < self.filtered_items.len() {
                        self.list_state.select(Some(clicked_index));
                    }
                }
                return;
            }
        }

        self.search_focused = false;
    }

    fn in_area(col: u16, row: u16, area: Rect) -> bool {
        col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.render_left_panel(f, main_chunks[0]);
        self.render_details(f, main_chunks[1]);

        if self.dropdown_open {
            self.render_dropdown_menu(f);
        }
    }

    fn render_left_panel(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        self.render_search_box(f, chunks[0]);
        self.render_dropdown_button(f, chunks[1]);
        self.render_list(f, chunks[2]);
    }

    fn render_search_box(&mut self, f: &mut Frame, area: Rect) {
        self.search_area = Some(area);

        let border_style = if self.search_focused {
            Style::default().fg(Color::Blue)
        } else {
            Style::default().fg(Color::White)
        };

        let title = if self.search_focused { " Search (typing...) " } else { " Search (s or click) " };
        let input_text = if self.search_focused {
            format!("{}▌", self.search_input)
        } else {
            self.search_input.clone()
        };

        let input = Paragraph::new(input_text)
            .block(Block::default().borders(Borders::ALL).border_style(border_style).title(title))
            .style(Style::default().fg(Color::Blue));
        f.render_widget(input, area);
    }

    fn render_dropdown_button(&mut self, f: &mut Frame, area: Rect) {
        self.dropdown_button_area = Some(area);

        let arrow = if self.dropdown_open { "▲" } else { "▼" };
        let text = format!(" {} {}", self.severity_filter.as_str(), arrow);

        let border_style = if self.dropdown_open {
            Style::default().fg(Color::Blue)
        } else {
            Style::default().fg(Color::White)
        };

        let button = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(" Severity Filter (f or click) ")
            )
            .style(Style::default().fg(self.severity_filter.color()).add_modifier(Modifier::BOLD));
        f.render_widget(button, area);
    }

    fn render_dropdown_menu(&mut self, f: &mut Frame) {
        if let Some(button_area) = self.dropdown_button_area {
            let options = SeverityFilter::all_options();
            let menu_height = options.len() as u16 + 2;

            let menu_area = Rect {
                x: button_area.x,
                y: button_area.y + button_area.height,
                width: button_area.width,
                height: menu_height,
            };
            self.dropdown_menu_area = Some(menu_area);

            f.render_widget(Clear, menu_area);

            let items: Vec<ListItem> = options
                .iter()
                .enumerate()
                .map(|(i, filter)| {
                    let style = if i == self.dropdown_selected {
                        Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(filter.color())
                    };
                    ListItem::new(format!(" {} ", filter.as_str())).style(style)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL));

            f.render_widget(list, menu_area);
        }
    }

    fn render_list(&mut self, f: &mut Frame, area: Rect) {
        self.list_area = Some(area);

        let items: Vec<ListItem> = self.filtered_items
            .iter()
            .map(|item| ListItem::new(item.title.as_str()))
            .collect();

        let title = format!(" Findings ({}) ", self.filtered_items.len());

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_details(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Details ");

        if let Some(finding) = self.get_selected() {
            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Length(2),
                    Constraint::Length(2),
                    Constraint::Length(2),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(inner);

            let title = Paragraph::new(Line::from(vec![
                Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&finding.title),
            ]));
            f.render_widget(title, chunks[0]);

            let severity = Paragraph::new(Line::from(vec![
                Span::styled("Severity: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(finding.severity.as_str(), Style::default().fg(finding.severity.color())),
            ]));
            f.render_widget(severity, chunks[1]);

            let status = Paragraph::new(Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(finding.status.as_str(), Style::default().fg(finding.status.color())),
            ]));
            f.render_widget(status, chunks[2]);

            let location = Paragraph::new(Line::from(vec![
                Span::styled("Location: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&finding.location, Style::default().fg(Color::Cyan)),
            ]));
            f.render_widget(location, chunks[3]);

            let description = Paragraph::new(Line::from(vec![
                Span::styled("Description:\n", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&finding.description),
            ]))
            .wrap(Wrap { trim: true });
            f.render_widget(description, chunks[5]);
        } else {
            let empty = Paragraph::new("No finding selected")
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(empty, area);
        }
    }
}