use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

// ---------------------------------------------------------------------------
// Hit-testing
// ---------------------------------------------------------------------------

/// Return `true` when (`col`, `row`) falls inside `area`.
pub fn in_area(col: u16, row: u16, area: Rect) -> bool {
    col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height
}

// ---------------------------------------------------------------------------
// List navigation helpers
// ---------------------------------------------------------------------------

/// Advance the selection in a list, wrapping around.
pub fn list_next(state: &mut ListState, len: usize) {
    if len == 0 { return; }
    let i = match state.selected() {
        Some(i) => (i + 1) % len,
        None => 0,
    };
    state.select(Some(i));
}

/// Move the selection backwards in a list, wrapping around.
pub fn list_previous(state: &mut ListState, len: usize) {
    if len == 0 { return; }
    let i = match state.selected() {
        Some(i) => if i == 0 { len - 1 } else { i - 1 },
        None => 0,
    };
    state.select(Some(i));
}

// ---------------------------------------------------------------------------
// Search box
// ---------------------------------------------------------------------------

/// Shared state for a search-box widget.
pub struct SearchBox {
    pub input: String,
    pub focused: bool,
    pub area: Option<Rect>,
}

impl Default for SearchBox {
    fn default() -> Self {
        Self { input: String::new(), focused: false, area: None }
    }
}

impl SearchBox {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render the search box into `area`, saving the area for later hit-testing.
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        self.area = Some(area);

        let border_style = if self.focused {
            Style::default().fg(Color::Blue)
        } else {
            Style::default().fg(Color::White)
        };

        let title = if self.focused { " Search (typing...) " } else { " Search (s or click) " };
        let input_text = if self.focused {
            format!("{}▌", self.input)
        } else {
            self.input.clone()
        };

        let widget = Paragraph::new(input_text)
            .block(Block::default().borders(Borders::ALL).border_style(border_style).title(title))
            .style(Style::default().fg(Color::Blue));
        f.render_widget(widget, area);
    }

    /// Returns the lowercased search query.
    pub fn query(&self) -> String {
        self.input.to_lowercase()
    }
}

// ---------------------------------------------------------------------------
// Dropdown
// ---------------------------------------------------------------------------

/// A labelled option for a dropdown — every option has a display name and a
/// colour used to tint the label.
pub struct DropdownOption {
    pub label: String,
    pub color: Color,
}

/// Generic dropdown state.  It does not own the option list — callers pass the
/// options slice into each method so the same state can be used with different
/// backing stores.
pub struct Dropdown {
    pub open: bool,
    pub selected: usize,
    pub button_area: Option<Rect>,
    pub menu_area: Option<Rect>,
}

impl Default for Dropdown {
    fn default() -> Self {
        Self { open: false, selected: 0, button_area: None, menu_area: None }
    }
}

impl Dropdown {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle(&mut self, current_index: usize) {
        if self.open {
            self.open = false;
        } else {
            self.open = true;
            self.selected = current_index;
        }
    }

    pub fn close(&mut self) { self.open = false; }

    pub fn next(&mut self, count: usize) {
        if count > 0 { self.selected = (self.selected + 1) % count; }
    }

    pub fn previous(&mut self, count: usize) {
        if count > 0 {
            self.selected = if self.selected == 0 { count - 1 } else { self.selected - 1 };
        }
    }

    /// Render the filter button (the clickable bar that opens/closes the
    /// dropdown).
    pub fn render_button(
        &mut self,
        f: &mut Frame,
        area: Rect,
        title: &str,
        label: &str,
        label_color: Color,
    ) {
        self.button_area = Some(area);

        let arrow = if self.open { "▲" } else { "▼" };
        let text = format!(" {} {}", label, arrow);

        let border_style = if self.open {
            Style::default().fg(Color::Blue)
        } else {
            Style::default().fg(Color::White)
        };

        let button = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(title)
            )
            .style(Style::default().fg(label_color).add_modifier(Modifier::BOLD));
        f.render_widget(button, area);
    }

    /// Render the floating menu below the button.
    pub fn render_menu(&mut self, f: &mut Frame, options: &[DropdownOption]) {
        if let Some(button_area) = self.button_area {
            let menu_height = options.len() as u16 + 2;

            let menu_area = Rect {
                x: button_area.x,
                y: button_area.y + button_area.height,
                width: button_area.width,
                height: menu_height,
            };
            self.menu_area = Some(menu_area);

            f.render_widget(Clear, menu_area);

            let items: Vec<ListItem> = options.iter().enumerate().map(|(i, opt)| {
                let style = if i == self.selected {
                    Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(opt.color)
                };
                ListItem::new(format!(" {} ", opt.label)).style(style)
            }).collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(list, menu_area);
        }
    }

    /// Handle a click that lands inside the open menu.  Returns `Some(index)`
    /// if a menu item was clicked, `None` otherwise.
    pub fn click_menu(&self, col: u16, row: u16, option_count: usize) -> Option<usize> {
        if let Some(area) = self.menu_area {
            if in_area(col, row, area) {
                let start_y = area.y + 1;
                if row >= start_y {
                    let idx = (row - start_y) as usize;
                    if idx < option_count {
                        return Some(idx);
                    }
                }
            }
        }
        None
    }
}
