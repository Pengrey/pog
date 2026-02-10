use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use models::{Finding, Severity};

use super::Tab;

// ---------------------------------------------------------------------------
// Severity filter (wraps the domain `Severity` with an extra "All" option)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum SeverityFilter {
    All,
    Only(Severity),
}

impl SeverityFilter {
    pub const OPTIONS: &[SeverityFilter] = &[
        SeverityFilter::All,
        SeverityFilter::Only(Severity::Critical),
        SeverityFilter::Only(Severity::High),
        SeverityFilter::Only(Severity::Medium),
        SeverityFilter::Only(Severity::Low),
        SeverityFilter::Only(Severity::Info),
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            SeverityFilter::All => "All",
            SeverityFilter::Only(s) => s.as_str(),
        }
    }

    pub fn matches(&self, severity: Severity) -> bool {
        match self {
            SeverityFilter::All => true,
            SeverityFilter::Only(s) => *s == severity,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            SeverityFilter::All => Color::White,
            SeverityFilter::Only(s) => s.color(),
        }
    }
}

// ---------------------------------------------------------------------------
// Asset filter
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
pub enum AssetFilter {
    All,
    Only(String),
}

impl AssetFilter {
    pub fn as_str(&self) -> &str {
        match self {
            AssetFilter::All => "All",
            AssetFilter::Only(s) => s.as_str(),
        }
    }

    pub fn matches(&self, asset: &str) -> bool {
        match self {
            AssetFilter::All => true,
            AssetFilter::Only(s) => s == asset,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            AssetFilter::All => Color::White,
            AssetFilter::Only(_) => Color::Cyan,
        }
    }
}


pub struct SearchTab {
    search_input: String,
    search_focused: bool,
    severity_filter: SeverityFilter,
    asset_filter: AssetFilter,
    asset_options: Vec<AssetFilter>,
    active_dropdown: ActiveDropdown,
    dropdown_selected: usize,
    items: Vec<Finding>,
    filtered_items: Vec<Finding>,
    list_state: ListState,
    search_area: Option<Rect>,
    severity_button_area: Option<Rect>,
    asset_button_area: Option<Rect>,
    dropdown_menu_area: Option<Rect>,
    list_area: Option<Rect>,
}

#[derive(Clone, Copy, PartialEq)]
enum ActiveDropdown {
    None,
    Severity,
    Asset,
}

impl SearchTab {
    pub fn new(items: Vec<Finding>) -> Self {
        let filtered_items = items.clone();
        let mut list_state = ListState::default();
        if !filtered_items.is_empty() {
            list_state.select(Some(0));
        }

        // Build unique sorted asset list
        let mut assets: Vec<String> = items.iter().map(|f| f.asset.clone()).collect();
        assets.sort();
        assets.dedup();
        let mut asset_options: Vec<AssetFilter> = vec![AssetFilter::All];
        asset_options.extend(assets.into_iter().map(AssetFilter::Only));

        Self {
            search_input: String::new(),
            search_focused: false,
            severity_filter: SeverityFilter::All,
            asset_filter: AssetFilter::All,
            asset_options,
            active_dropdown: ActiveDropdown::None,
            dropdown_selected: 0,
            items,
            filtered_items,
            list_state,
            search_area: None,
            severity_button_area: None,
            asset_button_area: None,
            dropdown_menu_area: None,
            list_area: None,
        }
    }

    fn toggle_severity_dropdown(&mut self) {
        if self.active_dropdown == ActiveDropdown::Severity {
            self.active_dropdown = ActiveDropdown::None;
        } else {
            self.active_dropdown = ActiveDropdown::Severity;
            self.search_focused = false;
            self.dropdown_selected = SeverityFilter::OPTIONS
                .iter()
                .position(|f| *f == self.severity_filter)
                .unwrap_or(0);
        }
    }

    fn toggle_asset_dropdown(&mut self) {
        if self.active_dropdown == ActiveDropdown::Asset {
            self.active_dropdown = ActiveDropdown::None;
        } else {
            self.active_dropdown = ActiveDropdown::Asset;
            self.search_focused = false;
            self.dropdown_selected = self.asset_options
                .iter()
                .position(|f| *f == self.asset_filter)
                .unwrap_or(0);
        }
    }

    fn dropdown_option_count(&self) -> usize {
        match self.active_dropdown {
            ActiveDropdown::Severity => SeverityFilter::OPTIONS.len(),
            ActiveDropdown::Asset => self.asset_options.len(),
            ActiveDropdown::None => 0,
        }
    }

    fn dropdown_next(&mut self) {
        let count = self.dropdown_option_count();
        if count > 0 {
            self.dropdown_selected = (self.dropdown_selected + 1) % count;
        }
    }

    fn dropdown_previous(&mut self) {
        let count = self.dropdown_option_count();
        if count > 0 {
            self.dropdown_selected = if self.dropdown_selected == 0 {
                count - 1
            } else {
                self.dropdown_selected - 1
            };
        }
    }

    fn dropdown_select(&mut self) {
        match self.active_dropdown {
            ActiveDropdown::Severity => {
                if let Some(&filter) = SeverityFilter::OPTIONS.get(self.dropdown_selected) {
                    self.severity_filter = filter;
                }
            }
            ActiveDropdown::Asset => {
                if let Some(filter) = self.asset_options.get(self.dropdown_selected) {
                    self.asset_filter = filter.clone();
                }
            }
            ActiveDropdown::None => {}
        }
        self.active_dropdown = ActiveDropdown::None;
        self.filter_items();
    }

    fn filter_items(&mut self) {
        let search_lower = self.search_input.to_lowercase();
        self.filtered_items = self.items
            .iter()
            .filter(|item| {
                let matches_search = search_lower.is_empty()
                    || item.title.to_lowercase().contains(&search_lower)
                    || item.description.to_lowercase().contains(&search_lower)
                    || item.location.to_lowercase().contains(&search_lower);
                let matches_severity = self.severity_filter.matches(item.severity);
                let matches_asset = self.asset_filter.matches(&item.asset);
                matches_search && matches_severity && matches_asset
            })
            .cloned()
            .collect();

        if self.filtered_items.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn list_next(&mut self) {
        if self.filtered_items.is_empty() { return; }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % self.filtered_items.len(),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn list_previous(&mut self) {
        if self.filtered_items.is_empty() { return; }
        let i = match self.list_state.selected() {
            Some(i) => if i == 0 { self.filtered_items.len() - 1 } else { i - 1 },
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn get_selected(&self) -> Option<&Finding> {
        self.list_state.selected().and_then(|i| self.filtered_items.get(i))
    }

    fn in_area(col: u16, row: u16, area: Rect) -> bool {
        col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height
    }
}

// ---------------------------------------------------------------------------
// Tab trait implementation
// ---------------------------------------------------------------------------

impl Tab for SearchTab {
    fn title(&self) -> &'static str { "Search" }

    fn on_blur(&mut self) {
        self.search_focused = false;
        self.active_dropdown = ActiveDropdown::None;
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        // --- dropdown is open: route keys to dropdown ---
        if self.active_dropdown != ActiveDropdown::None {
            return match key {
                KeyCode::Esc => { self.active_dropdown = ActiveDropdown::None; true }
                KeyCode::Enter => { self.dropdown_select(); true }
                KeyCode::Down => { self.dropdown_next(); true }
                KeyCode::Up => { self.dropdown_previous(); true }
                _ => false,
            };
        }

        // --- search box focused: route keys to text input ---
        if self.search_focused {
            return match key {
                KeyCode::Esc | KeyCode::Enter => { self.search_focused = false; true }
                KeyCode::Char(c) => { self.search_input.push(c); self.filter_items(); true }
                KeyCode::Backspace => { self.search_input.pop(); self.filter_items(); true }
                KeyCode::Down => { self.list_next(); true }
                KeyCode::Up => { self.list_previous(); true }
                _ => false,
            };
        }

        // --- normal mode: tab-level shortcuts ---
        match key {
            KeyCode::Char('s') => { self.search_focused = true; self.active_dropdown = ActiveDropdown::None; true }
            KeyCode::Char('f') => { self.toggle_severity_dropdown(); true }
            KeyCode::Char('a') => { self.toggle_asset_dropdown(); true }
            KeyCode::Down => { self.list_next(); true }
            KeyCode::Up => { self.list_previous(); true }
            _ => false,
        }
    }

    fn handle_click(&mut self, col: u16, row: u16) {
        if self.active_dropdown != ActiveDropdown::None {
            if let Some(area) = self.dropdown_menu_area
                && Self::in_area(col, row, area)
            {
                let menu_start_y = area.y + 1;
                if row >= menu_start_y {
                    let clicked_index = (row - menu_start_y) as usize;
                    if clicked_index < self.dropdown_option_count() {
                        self.dropdown_selected = clicked_index;
                        self.dropdown_select();
                    }
                }
                return;
            }
            self.active_dropdown = ActiveDropdown::None;
        }

        if let Some(area) = self.severity_button_area
            && Self::in_area(col, row, area)
        {
            self.toggle_severity_dropdown();
            return;
        }

        if let Some(area) = self.asset_button_area
            && Self::in_area(col, row, area)
        {
            self.toggle_asset_dropdown();
            return;
        }

        if let Some(area) = self.search_area
            && Self::in_area(col, row, area)
        {
            self.search_focused = true;
            self.active_dropdown = ActiveDropdown::None;
            return;
        }

        if let Some(area) = self.list_area
            && Self::in_area(col, row, area)
        {
            let list_start_y = area.y + 1;
            if row >= list_start_y {
                let clicked_index = (row - list_start_y) as usize;
                if clicked_index < self.filtered_items.len() {
                    self.list_state.select(Some(clicked_index));
                }
            }
            return;
        }

        self.search_focused = false;
    }

    fn handle_scroll_down(&mut self) {
        self.list_next();
    }

    fn handle_scroll_up(&mut self) {
        self.list_previous();
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.render_left_panel(f, main_chunks[0]);
        self.render_details(f, main_chunks[1]);

        if self.active_dropdown != ActiveDropdown::None {
            self.render_dropdown_menu(f);
        }
    }
}

// ---------------------------------------------------------------------------
// Private rendering helpers
// ---------------------------------------------------------------------------

impl SearchTab {
    fn render_left_panel(&mut self, f: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        self.render_search_box(f, rows[0]);

        // Severity and asset filters side by side
        let filter_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);

        self.render_severity_button(f, filter_cols[0]);
        self.render_asset_button(f, filter_cols[1]);
        self.render_list(f, rows[2]);
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

    fn render_severity_button(&mut self, f: &mut Frame, area: Rect) {
        self.severity_button_area = Some(area);

        let arrow = if self.active_dropdown == ActiveDropdown::Severity { "▲" } else { "▼" };
        let text = format!(" {} {}", self.severity_filter.as_str(), arrow);

        let border_style = if self.active_dropdown == ActiveDropdown::Severity {
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

    fn render_asset_button(&mut self, f: &mut Frame, area: Rect) {
        self.asset_button_area = Some(area);

        let arrow = if self.active_dropdown == ActiveDropdown::Asset { "▲" } else { "▼" };
        let text = format!(" {} {}", self.asset_filter.as_str(), arrow);

        let border_style = if self.active_dropdown == ActiveDropdown::Asset {
            Style::default().fg(Color::Blue)
        } else {
            Style::default().fg(Color::White)
        };

        let button = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(" Asset Filter (a or click) ")
            )
            .style(Style::default().fg(self.asset_filter.color()).add_modifier(Modifier::BOLD));
        f.render_widget(button, area);
    }

    fn render_dropdown_menu(&mut self, f: &mut Frame) {
        let button_area = match self.active_dropdown {
            ActiveDropdown::Severity => self.severity_button_area,
            ActiveDropdown::Asset => self.asset_button_area,
            ActiveDropdown::None => None,
        };

        if let Some(button_area) = button_area {
            let option_count = self.dropdown_option_count();
            let menu_height = option_count as u16 + 2;

            let menu_area = Rect {
                x: button_area.x,
                y: button_area.y + button_area.height,
                width: button_area.width,
                height: menu_height,
            };
            self.dropdown_menu_area = Some(menu_area);

            f.render_widget(Clear, menu_area);

            let items: Vec<ListItem> = match self.active_dropdown {
                ActiveDropdown::Severity => {
                    SeverityFilter::OPTIONS.iter().enumerate().map(|(i, filter)| {
                        let style = if i == self.dropdown_selected {
                            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(filter.color())
                        };
                        ListItem::new(format!(" {} ", filter.as_str())).style(style)
                    }).collect()
                }
                ActiveDropdown::Asset => {
                    self.asset_options.iter().enumerate().map(|(i, filter)| {
                        let style = if i == self.dropdown_selected {
                            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(filter.color())
                        };
                        ListItem::new(format!(" {} ", filter.as_str())).style(style)
                    }).collect()
                }
                ActiveDropdown::None => vec![],
            };

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
                    Constraint::Length(2), // title
                    Constraint::Length(2), // id
                    Constraint::Length(2), // severity
                    Constraint::Length(2), // asset
                    Constraint::Length(2), // date
                    Constraint::Length(2), // status
                    Constraint::Length(2), // location
                    Constraint::Length(1), // spacer
                    Constraint::Min(0),   // description
                ])
                .split(inner);

            let title = Paragraph::new(Line::from(vec![
                Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&finding.title),
            ]));
            f.render_widget(title, chunks[0]);

            let id_line = Paragraph::new(Line::from(vec![
                Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&finding.hex_id, Style::default().fg(Color::Yellow)),
            ]));
            f.render_widget(id_line, chunks[1]);

            let severity = Paragraph::new(Line::from(vec![
                Span::styled("Severity: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(finding.severity.as_str(), Style::default().fg(finding.severity.color())),
            ]));
            f.render_widget(severity, chunks[2]);

            let asset = Paragraph::new(Line::from(vec![
                Span::styled("Asset: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&finding.asset, Style::default().fg(Color::Cyan)),
            ]));
            f.render_widget(asset, chunks[3]);

            let date = Paragraph::new(Line::from(vec![
                Span::styled("Date: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(if finding.date.is_empty() { "—" } else { &finding.date }),
            ]));
            f.render_widget(date, chunks[4]);

            let status = Paragraph::new(Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(finding.status.as_str(), Style::default().fg(finding.status.color())),
            ]));
            f.render_widget(status, chunks[5]);

            let location = Paragraph::new(Line::from(vec![
                Span::styled("Location: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&finding.location, Style::default().fg(Color::Cyan)),
            ]));
            f.render_widget(location, chunks[6]);

            let description = Paragraph::new(Line::from(vec![
                Span::styled("Description:\n", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&finding.description),
            ]))
            .wrap(Wrap { trim: true });
            f.render_widget(description, chunks[8]);
        } else {
            let empty = Paragraph::new("No finding selected")
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(empty, area);
        }
    }
}