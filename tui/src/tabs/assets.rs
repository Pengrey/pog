use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use models::Asset;

use super::Tab;

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
}

// ---------------------------------------------------------------------------
// Assets tab state
// ---------------------------------------------------------------------------

pub struct AssetsTab {
    search_input: String,
    search_focused: bool,
    criticality_filter: CriticalityFilter,
    dropdown_open: bool,
    dropdown_selected: usize,
    items: Vec<Asset>,
    filtered_items: Vec<Asset>,
    list_state: ListState,
    search_area: Option<Rect>,
    criticality_button_area: Option<Rect>,
    dropdown_menu_area: Option<Rect>,
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
            search_input: String::new(),
            search_focused: false,
            criticality_filter: CriticalityFilter::All,
            dropdown_open: false,
            dropdown_selected: 0,
            items,
            filtered_items,
            list_state,
            search_area: None,
            criticality_button_area: None,
            dropdown_menu_area: None,
            list_area: None,
        }
    }

    fn filter_items(&mut self) {
        let search_lower = self.search_input.to_lowercase();
        self.filtered_items = self.items
            .iter()
            .filter(|asset| {
                let matches_search = search_lower.is_empty()
                    || asset.name.to_lowercase().contains(&search_lower)
                    || asset.description.to_lowercase().contains(&search_lower)
                    || asset.contact.to_lowercase().contains(&search_lower)
                    || asset.dns_or_ip.to_lowercase().contains(&search_lower);
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
        if self.dropdown_open {
            self.dropdown_open = false;
        } else {
            self.dropdown_open = true;
            self.search_focused = false;
            self.dropdown_selected = CriticalityFilter::OPTIONS
                .iter()
                .position(|f| *f == self.criticality_filter)
                .unwrap_or(0);
        }
    }

    fn dropdown_next(&mut self) {
        let count = CriticalityFilter::OPTIONS.len();
        self.dropdown_selected = (self.dropdown_selected + 1) % count;
    }

    fn dropdown_previous(&mut self) {
        let count = CriticalityFilter::OPTIONS.len();
        self.dropdown_selected = if self.dropdown_selected == 0 {
            count - 1
        } else {
            self.dropdown_selected - 1
        };
    }

    fn dropdown_select(&mut self) {
        if let Some(&filter) = CriticalityFilter::OPTIONS.get(self.dropdown_selected) {
            self.criticality_filter = filter;
        }
        self.dropdown_open = false;
        self.filter_items();
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

    fn get_selected(&self) -> Option<&Asset> {
        self.list_state.selected().and_then(|i| self.filtered_items.get(i))
    }

    fn in_area(col: u16, row: u16, area: Rect) -> bool {
        col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height
    }
}

// ---------------------------------------------------------------------------
// Tab trait implementation
// ---------------------------------------------------------------------------

impl Tab for AssetsTab {
    fn title(&self) -> &'static str { "Assets" }

    fn on_blur(&mut self) {
        self.search_focused = false;
        self.dropdown_open = false;
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        // --- dropdown open ---
        if self.dropdown_open {
            return match key {
                KeyCode::Esc => { self.dropdown_open = false; true }
                KeyCode::Enter => { self.dropdown_select(); true }
                KeyCode::Down => { self.dropdown_next(); true }
                KeyCode::Up => { self.dropdown_previous(); true }
                _ => false,
            };
        }

        // --- search focused ---
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

        // --- normal mode ---
        match key {
            KeyCode::Char('s') => { self.search_focused = true; self.dropdown_open = false; true }
            KeyCode::Char('f') => { self.toggle_dropdown(); true }
            KeyCode::Down | KeyCode::Char('j') => { self.list_next(); true }
            KeyCode::Up | KeyCode::Char('k') => { self.list_previous(); true }
            _ => false,
        }
    }

    fn handle_click(&mut self, col: u16, row: u16) {
        // Click inside open dropdown
        if self.dropdown_open {
            if let Some(area) = self.dropdown_menu_area
                && Self::in_area(col, row, area)
            {
                let menu_start_y = area.y + 1;
                if row >= menu_start_y {
                    let clicked_index = (row - menu_start_y) as usize;
                    if clicked_index < CriticalityFilter::OPTIONS.len() {
                        self.dropdown_selected = clicked_index;
                        self.dropdown_select();
                    }
                }
                return;
            }
            self.dropdown_open = false;
        }

        // Click on criticality button
        if let Some(area) = self.criticality_button_area
            && Self::in_area(col, row, area)
        {
            self.toggle_dropdown();
            return;
        }

        // Click on search box
        if let Some(area) = self.search_area
            && Self::in_area(col, row, area)
        {
            self.search_focused = true;
            self.dropdown_open = false;
            return;
        }

        // Click on list
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
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        self.render_left_panel(f, main_chunks[0]);
        self.render_details(f, main_chunks[1]);

        if self.dropdown_open {
            self.render_dropdown_menu(f);
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
                Constraint::Length(3), // search
                Constraint::Length(3), // criticality filter
                Constraint::Min(0),   // list
            ])
            .split(area);

        self.render_search_box(f, rows[0]);
        self.render_criticality_button(f, rows[1]);
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

    fn render_criticality_button(&mut self, f: &mut Frame, area: Rect) {
        self.criticality_button_area = Some(area);

        let arrow = if self.dropdown_open { "▲" } else { "▼" };
        let text = format!(" {} {}", self.criticality_filter.as_str(), arrow);

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
                    .title(" Criticality Filter (f or click) ")
            )
            .style(Style::default().fg(self.criticality_filter.color()).add_modifier(Modifier::BOLD));
        f.render_widget(button, area);
    }

    fn render_dropdown_menu(&mut self, f: &mut Frame) {
        if let Some(button_area) = self.criticality_button_area {
            let option_count = CriticalityFilter::OPTIONS.len();
            let menu_height = option_count as u16 + 2;

            let menu_area = Rect {
                x: button_area.x,
                y: button_area.y + button_area.height,
                width: button_area.width,
                height: menu_height,
            };
            self.dropdown_menu_area = Some(menu_area);

            f.render_widget(Clear, menu_area);

            let items: Vec<ListItem> = CriticalityFilter::OPTIONS.iter().enumerate().map(|(i, filter)| {
                let style = if i == self.dropdown_selected {
                    Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(filter.color())
                };
                ListItem::new(format!(" {} ", filter.as_str())).style(style)
            }).collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL));

            f.render_widget(list, menu_area);
        }
    }

    fn render_list(&mut self, f: &mut Frame, area: Rect) {
        self.list_area = Some(area);

        let items: Vec<ListItem> = self.filtered_items
            .iter()
            .map(|asset| {
                let crit_color = match asset.criticality.to_lowercase().as_str() {
                    "critical" => Color::Red,
                    "high" => Color::LightRed,
                    "medium" => Color::Yellow,
                    "low" => Color::Green,
                    _ => Color::Gray,
                };
                ListItem::new(Line::from(vec![
                    Span::raw(&asset.name),
                    Span::raw(" "),
                    Span::styled(
                        format!("[{}]", asset.criticality),
                        Style::default().fg(crit_color),
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
                    Constraint::Length(2), // name
                    Constraint::Length(2), // criticality
                    Constraint::Length(2), // dns/ip
                    Constraint::Length(2), // contact
                    Constraint::Length(1), // spacer
                    Constraint::Min(0),   // description
                ])
                .split(inner);

            let name = Paragraph::new(Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&asset.name, Style::default().fg(Color::Cyan)),
            ]));
            f.render_widget(name, chunks[0]);

            let crit_color = match asset.criticality.to_lowercase().as_str() {
                "critical" => Color::Red,
                "high" => Color::LightRed,
                "medium" => Color::Yellow,
                "low" => Color::Green,
                _ => Color::Gray,
            };
            let criticality = Paragraph::new(Line::from(vec![
                Span::styled("Criticality: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&asset.criticality, Style::default().fg(crit_color)),
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
