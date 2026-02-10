use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use models::{Finding, Severity};

use super::Tab;
use crate::widgets::{self, Dropdown, DropdownOption, SearchBox};

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

    fn dropdown_options() -> Vec<DropdownOption> {
        Self::OPTIONS.iter().map(|f| DropdownOption {
            label: f.as_str().to_string(),
            color: f.color(),
        }).collect()
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

// ---------------------------------------------------------------------------
// Which dropdown is active
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum ActiveDropdown {
    None,
    Severity,
    Asset,
}

// ---------------------------------------------------------------------------
// Search tab state
// ---------------------------------------------------------------------------

pub struct SearchTab {
    search: SearchBox,
    severity_filter: SeverityFilter,
    asset_filter: AssetFilter,
    asset_options: Vec<AssetFilter>,
    active_dropdown: ActiveDropdown,
    severity_dropdown: Dropdown,
    asset_dropdown: Dropdown,
    items: Vec<Finding>,
    filtered_items: Vec<Finding>,
    list_state: ListState,
    list_area: Option<Rect>,
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
            search: SearchBox::new(),
            severity_filter: SeverityFilter::All,
            asset_filter: AssetFilter::All,
            asset_options,
            active_dropdown: ActiveDropdown::None,
            severity_dropdown: Dropdown::new(),
            asset_dropdown: Dropdown::new(),
            items,
            filtered_items,
            list_state,
            list_area: None,
        }
    }

    // --- dropdown helpers ---------------------------------------------------

    fn toggle_severity_dropdown(&mut self) {
        if self.active_dropdown == ActiveDropdown::Severity {
            self.active_dropdown = ActiveDropdown::None;
            self.severity_dropdown.close();
        } else {
            self.active_dropdown = ActiveDropdown::Severity;
            self.search.focused = false;
            self.asset_dropdown.close();
            let idx = SeverityFilter::OPTIONS
                .iter()
                .position(|f| *f == self.severity_filter)
                .unwrap_or(0);
            self.severity_dropdown.toggle(idx);
        }
    }

    fn toggle_asset_dropdown(&mut self) {
        if self.active_dropdown == ActiveDropdown::Asset {
            self.active_dropdown = ActiveDropdown::None;
            self.asset_dropdown.close();
        } else {
            self.active_dropdown = ActiveDropdown::Asset;
            self.search.focused = false;
            self.severity_dropdown.close();
            let idx = self.asset_options
                .iter()
                .position(|f| *f == self.asset_filter)
                .unwrap_or(0);
            self.asset_dropdown.toggle(idx);
        }
    }

    fn active_dropdown_mut(&mut self) -> Option<&mut Dropdown> {
        match self.active_dropdown {
            ActiveDropdown::Severity => Some(&mut self.severity_dropdown),
            ActiveDropdown::Asset => Some(&mut self.asset_dropdown),
            ActiveDropdown::None => None,
        }
    }

    fn dropdown_option_count(&self) -> usize {
        match self.active_dropdown {
            ActiveDropdown::Severity => SeverityFilter::OPTIONS.len(),
            ActiveDropdown::Asset => self.asset_options.len(),
            ActiveDropdown::None => 0,
        }
    }

    fn dropdown_select(&mut self) {
        match self.active_dropdown {
            ActiveDropdown::Severity => {
                if let Some(&filter) = SeverityFilter::OPTIONS.get(self.severity_dropdown.selected) {
                    self.severity_filter = filter;
                }
                self.severity_dropdown.close();
            }
            ActiveDropdown::Asset => {
                if let Some(filter) = self.asset_options.get(self.asset_dropdown.selected) {
                    self.asset_filter = filter.clone();
                }
                self.asset_dropdown.close();
            }
            ActiveDropdown::None => {}
        }
        self.active_dropdown = ActiveDropdown::None;
        self.filter_items();
    }

    fn close_dropdown(&mut self) {
        match self.active_dropdown {
            ActiveDropdown::Severity => self.severity_dropdown.close(),
            ActiveDropdown::Asset => self.asset_dropdown.close(),
            ActiveDropdown::None => {}
        }
        self.active_dropdown = ActiveDropdown::None;
    }

    // --- filtering ----------------------------------------------------------

    fn filter_items(&mut self) {
        let query = self.search.query();
        self.filtered_items = self.items
            .iter()
            .filter(|item| {
                let matches_search = query.is_empty()
                    || item.title.to_lowercase().contains(&query)
                    || item.description.to_lowercase().contains(&query)
                    || item.location.to_lowercase().contains(&query);
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

    fn get_selected(&self) -> Option<&Finding> {
        self.list_state.selected().and_then(|i| self.filtered_items.get(i))
    }
}

// ---------------------------------------------------------------------------
// Tab trait implementation
// ---------------------------------------------------------------------------

impl Tab for SearchTab {
    fn title(&self) -> &'static str { "Search" }

    fn on_blur(&mut self) {
        self.search.focused = false;
        self.close_dropdown();
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        // --- dropdown open ---
        if self.active_dropdown != ActiveDropdown::None {
            let count = self.dropdown_option_count();
            return match key {
                KeyCode::Esc => { self.close_dropdown(); true }
                KeyCode::Enter => { self.dropdown_select(); true }
                KeyCode::Down => { if let Some(d) = self.active_dropdown_mut() { d.next(count); } true }
                KeyCode::Up => { if let Some(d) = self.active_dropdown_mut() { d.previous(count); } true }
                _ => false,
            };
        }

        // --- search focused ---
        if self.search.focused {
            return match key {
                KeyCode::Esc | KeyCode::Enter => { self.search.focused = false; true }
                KeyCode::Char(c) => { self.search.input.push(c); self.filter_items(); true }
                KeyCode::Backspace => { self.search.input.pop(); self.filter_items(); true }
                KeyCode::Down => { widgets::list_next(&mut self.list_state, self.filtered_items.len()); true }
                KeyCode::Up => { widgets::list_previous(&mut self.list_state, self.filtered_items.len()); true }
                _ => false,
            };
        }

        // --- normal mode ---
        match key {
            KeyCode::Char('s') => { self.search.focused = true; self.close_dropdown(); true }
            KeyCode::Char('f') => { self.toggle_severity_dropdown(); true }
            KeyCode::Char('a') => { self.toggle_asset_dropdown(); true }
            KeyCode::Down => { widgets::list_next(&mut self.list_state, self.filtered_items.len()); true }
            KeyCode::Up => { widgets::list_previous(&mut self.list_state, self.filtered_items.len()); true }
            _ => false,
        }
    }

    fn handle_click(&mut self, col: u16, row: u16) {
        if self.active_dropdown != ActiveDropdown::None {
            let active = self.active_dropdown;
            let dd = match active {
                ActiveDropdown::Severity => &self.severity_dropdown,
                ActiveDropdown::Asset => &self.asset_dropdown,
                ActiveDropdown::None => unreachable!(),
            };
            let count = self.dropdown_option_count();
            if let Some(idx) = dd.click_menu(col, row, count) {
                match active {
                    ActiveDropdown::Severity => self.severity_dropdown.selected = idx,
                    ActiveDropdown::Asset => self.asset_dropdown.selected = idx,
                    ActiveDropdown::None => unreachable!(),
                }
                self.dropdown_select();
                return;
            }
            self.close_dropdown();
        }

        if let Some(area) = self.severity_dropdown.button_area {
            if widgets::in_area(col, row, area) {
                self.toggle_severity_dropdown();
                return;
            }
        }

        if let Some(area) = self.asset_dropdown.button_area {
            if widgets::in_area(col, row, area) {
                self.toggle_asset_dropdown();
                return;
            }
        }

        if let Some(area) = self.search.area {
            if widgets::in_area(col, row, area) {
                self.search.focused = true;
                self.close_dropdown();
                return;
            }
        }

        if let Some(area) = self.list_area {
            if widgets::in_area(col, row, area) {
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

        self.search.focused = false;
    }

    fn handle_scroll_down(&mut self) {
        widgets::list_next(&mut self.list_state, self.filtered_items.len());
    }

    fn handle_scroll_up(&mut self) {
        widgets::list_previous(&mut self.list_state, self.filtered_items.len());
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.render_left_panel(f, main_chunks[0]);
        self.render_details(f, main_chunks[1]);

        if self.active_dropdown != ActiveDropdown::None {
            self.render_active_dropdown_menu(f);
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

        self.search.render(f, rows[0]);

        // Severity and asset filter buttons side by side
        let filter_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);

        self.severity_dropdown.render_button(
            f, filter_cols[0],
            " Severity Filter (f or click) ",
            self.severity_filter.as_str(),
            self.severity_filter.color(),
        );
        self.asset_dropdown.render_button(
            f, filter_cols[1],
            " Asset Filter (a or click) ",
            self.asset_filter.as_str(),
            self.asset_filter.color(),
        );

        self.render_list(f, rows[2]);
    }

    fn render_active_dropdown_menu(&mut self, f: &mut Frame) {
        match self.active_dropdown {
            ActiveDropdown::Severity => {
                let opts = SeverityFilter::dropdown_options();
                self.severity_dropdown.render_menu(f, &opts);
            }
            ActiveDropdown::Asset => {
                let opts: Vec<DropdownOption> = self.asset_options.iter().map(|a| DropdownOption {
                    label: a.as_str().to_string(),
                    color: a.color(),
                }).collect();
                self.asset_dropdown.render_menu(f, &opts);
            }
            ActiveDropdown::None => {}
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
