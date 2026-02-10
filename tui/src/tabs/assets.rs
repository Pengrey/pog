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

pub struct AssetsTab {
    items: Vec<Asset>,
    list_state: ListState,
    list_area: Option<Rect>,
}

impl AssetsTab {
    pub fn new(items: Vec<Asset>) -> Self {
        let mut list_state = ListState::default();
        if !items.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            items,
            list_state,
            list_area: None,
        }
    }

    fn list_next(&mut self) {
        if self.items.is_empty() { return; }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % self.items.len(),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn list_previous(&mut self) {
        if self.items.is_empty() { return; }
        let i = match self.list_state.selected() {
            Some(i) => if i == 0 { self.items.len() - 1 } else { i - 1 },
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn get_selected(&self) -> Option<&Asset> {
        self.list_state.selected().and_then(|i| self.items.get(i))
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

    fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Down | KeyCode::Char('j') => { self.list_next(); true }
            KeyCode::Up | KeyCode::Char('k') => { self.list_previous(); true }
            _ => false,
        }
    }

    fn handle_click(&mut self, col: u16, row: u16) {
        if let Some(area) = self.list_area {
            if Self::in_area(col, row, area) {
                let list_start_y = area.y + 1;
                if row >= list_start_y {
                    let clicked_index = (row - list_start_y) as usize;
                    if clicked_index < self.items.len() {
                        self.list_state.select(Some(clicked_index));
                    }
                }
            }
        }
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

        self.render_list(f, main_chunks[0]);
        self.render_details(f, main_chunks[1]);
    }
}

// ---------------------------------------------------------------------------
// Private rendering helpers
// ---------------------------------------------------------------------------

impl AssetsTab {
    fn render_list(&mut self, f: &mut Frame, area: Rect) {
        self.list_area = Some(area);

        let items: Vec<ListItem> = self.items
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

        let title = format!(" Assets ({}) ", self.items.len());

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
