use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::Tab;

pub struct PlaceholderTab {
    click_count: u32,
    last_click: Option<(u16, u16)>,
    area: Option<Rect>,
}

impl PlaceholderTab {
    pub fn new() -> Self {
        Self {
            click_count: 0,
            last_click: None,
            area: None,
        }
    }

    fn in_area(&self, col: u16, row: u16) -> bool {
        self.area.is_some_and(|a| {
            col >= a.x && col < a.x + a.width && row >= a.y && row < a.y + a.height
        })
    }
}

impl Tab for PlaceholderTab {
    fn title(&self) -> &'static str { "Placeholder" }

    fn handle_click(&mut self, col: u16, row: u16) {
        if self.in_area(col, row) {
            self.click_count += 1;
            self.last_click = Some((col, row));
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        self.area = Some(area);

        let click_info = if let Some((x, y)) = self.last_click {
            format!("Last click: ({}, {}) | Total clicks: {}", x, y, self.click_count)
        } else {
            "Click anywhere!".to_string()
        };

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "🚧 Coming Soon 🚧",
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("This tab is a placeholder for future content."),
            Line::from(""),
            Line::from(Span::styled(click_info, Style::default().fg(Color::Cyan))),
        ];

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(" Placeholder "))
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }
}
