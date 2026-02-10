use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use models::Asset;

use super::Tab;
use crate::widgets::{self, Dropdown, DropdownOption, SearchBox};

// ---------------------------------------------------------------------------
// Criticality filter
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum CriticalityFilter {
    All,
    Critical,
    High,
    Medium,
    Low,
}

impl CriticalityFilter {
    pub const OPTIONS: &[CriticalityFilter] = &[
        CriticalityFilter::All,
        CriticalityFilter::Critical,
        CriticalityFilter::High,
        CriticalityFilter::Medium,
        CriticalityFilter::Low,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            CriticalityFilter::All => "All",
            CriticalityFilter::Critical => "Critical",
            CriticalityFilter::High => "High",
            CriticalityFilter::Medium => "Medium",
            CriticalityFilter::Low => "Low",
        }
    }

    pub fn matches(&self, criticality: &str) -> bool {
        match self {
            CriticalityFilter::All => true,
            other => other.as_str().eq_ignore_ascii_case(criticality),
        }
    }

    pub fn color(&self) -> Color {
        match self {
            CriticalityFilter::All => Color::White,
            CriticalityFilter::Critical => Color::Red,
            CriticalityFilter::High => Color::LightRed,
            CriticalityFilter::Medium => Color::Yellow,
            CriticalityFilter::Low => Color::Green,
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
// Assets tab state
// ---------------------------------------------------------------------------

pub struct AssetsTab {
    search: SearchBox,
    criticality_filter: CriticalityFilter,
    dropdown: Dropdown,
    items: Vec<Asset>,
    filtered_items: Vec<Asset>,
    list_state: ListState,
    list_area: Option<Rect>,
}

impl AssetsTab {
    pub fn new(items: Vec<Asset>) -> Self {
        let filtered_items = items.clone();
        let mut list_state = ListState::default();
        if !filtered_items.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            search: SearchBox::new(),
            criticality_filter: CriticalityFilter::All,
            dropdown: Dropdown::new(),
            items,
            filtered_items,
            list_state,
            list_area: None,
        }
    }

    fn filter_items(&mut self) {
        let query = self.search.query();
        self.filtered_items = self.items
            .iter()
            .filter(|asset| {
                let matches_search = query.is_empty()
                    || asset.name.to_lowercase().contains(&query)
                    || asset.description.to_lowercase().contains(&query)
                    || asset.contact.to_lowercase().contains(&query)
                    || asset.dns_or_ip.to_lowercase().contains(&query);
                let matches_crit = self.criticality_filter.matches(&asset.criticality);
                matches_search && matches_crit
            })
            .cloned()
            .collect();

        if self.filtered_items.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn toggle_dropdown(&mut self) {
        let idx = CriticalityFilter::OPTIONS
            .iter()
            .position(|f| *f == self.criticality_filter)
            .unwrap_or(0);
        self.dropdown.toggle(idx);
        if self.dropdown.open { self.search.focused = false; }
    }

    fn dropdown_select(&mut self) {
        if let Some(&filter) = CriticalityFilter::OPTIONS.get(self.dropdown.selected) {
            self.criticality_filter = filter;
        }
        self.dropdown.close();
        self.filter_items();
    }

    fn get_selected(&self) -> Option<&Asset> {
        self.list_state.selected().and_then(|i| self.filtered_items.get(i))
    }
}

// ---------------------------------------------------------------------------
// Tab trait implementation
// ---------------------------------------------------------------------------

impl Tab for AssetsTab {
    fn title(&self) -> &'static str { "Assets" }

    fn on_blur(&mut self) {
        self.search.focused = false;
        self.dropdown.close();
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        if self.dropdown.open {
            return match key {
                KeyCode::Esc => { self.dropdown.close(); true }
                KeyCode::Enter => { self.dropdown_select(); true }
                KeyCode::Down => { self.dropdown.next(CriticalityFilter::OPTIONS.len()); true }
                KeyCode::Up => { self.dropdown.previous(CriticalityFilter::OPTIONS.len()); true }
                _ => false,
            };
        }

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

        match key {
            KeyCode::Char('s') => { self.search.focused = true; self.dropdown.close(); true }
            KeyCode::Char('f') => { self.toggle_dropdown(); true }
            KeyCode::Down | KeyCode::Char('j') => { widgets::list_next(&mut self.list_state, self.filtered_items.len()); true }
            KeyCode::Up | KeyCode::Char('k') => { widgets::list_previous(&mut self.list_state, self.filtered_items.len()); true }
            _ => false,
        }
    }

    fn handle_click(&mut self, col: u16, row: u16) {
        if self.dropdown.open {
            if let Some(idx) = self.dropdown.click_menu(col, row, CriticalityFilter::OPTIONS.len()) {
                self.dropdown.selected = idx;
                self.dropdown_select();
                return;
            }
            self.dropdown.close();
        }

        if let Some(area) = self.dropdown.button_area {
            if widgets::in_area(col, row, area) {
                self.toggle_dropdown();
                return;
            }
        }

        if let Some(area) = self.search.area {
            if widgets::in_area(col, row, area) {
                self.search.focused = true;
                self.dropdown.close();
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
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        self.render_left_panel(f, main_chunks[0]);
        self.render_details(f, main_chunks[1]);

        if self.dropdown.open {
            let opts = CriticalityFilter::dropdown_options();
            self.dropdown.render_menu(f, &opts);
        }
    }
}

// ---------------------------------------------------------------------------
// Private rendering helpers
// ---------------------------------------------------------------------------

impl AssetsTab {
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
        self.dropdown.render_button(
            f, rows[1],
            " Criticality Filter (f or click) ",
            self.criticality_filter.as_str(),
            self.criticality_filter.color(),
        );
        self.render_list(f, rows[2]);
    }

    fn render_list(&mut self, f: &mut Frame, area: Rect) {
        self.list_area = Some(area);

        let items: Vec<ListItem> = self.filtered_items
            .iter()
            .map(|asset| {
                ListItem::new(Line::from(vec![
                    Span::raw(&asset.name),
                    Span::raw(" "),
                    Span::styled(
                        format!("[{}]", asset.criticality),
                        Style::default().fg(asset.criticality_color()),
                    ),
                ]))
            })
            .collect();

        let title = format!(" Assets ({}) ", self.filtered_items.len());

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_details(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Asset Details ");

        if let Some(asset) = self.get_selected() {
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

            let name = Paragraph::new(Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&asset.name, Style::default().fg(Color::Cyan)),
            ]));
            f.render_widget(name, chunks[0]);

            let criticality = Paragraph::new(Line::from(vec![
                Span::styled("Criticality: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&asset.criticality, Style::default().fg(asset.criticality_color())),
            ]));
            f.render_widget(criticality, chunks[1]);

            let dns = Paragraph::new(Line::from(vec![
                Span::styled("DNS/IP: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&asset.dns_or_ip),
            ]));
            f.render_widget(dns, chunks[2]);

            let contact = Paragraph::new(Line::from(vec![
                Span::styled("Contact: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&asset.contact),
            ]));
            f.render_widget(contact, chunks[3]);

            let description = Paragraph::new(Line::from(vec![
                Span::styled("Description:\n", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&asset.description),
            ]))
            .wrap(Wrap { trim: true });
            f.render_widget(description, chunks[5]);
        } else {
            let empty = Paragraph::new("No asset selected")
                .block(block)
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(empty, area);
        }
    }
}
