use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

use crate::app::GraphData;

pub struct GraphTab {
    data: GraphData,
}

impl GraphTab {
    pub fn new(data: GraphData) -> Self {
        Self { data }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", self.data.title));

        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.data.bars.is_empty() {
            let msg = Paragraph::new("No data to display")
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(msg, inner);
            return;
        }

        let max_value = self.data.bars.iter().map(|b| b.value).max().unwrap_or(1);
        let label_width = self.data.bars.iter().map(|b| b.label.len()).max().unwrap_or(0) as u16 + 2;

        let bar_constraints: Vec<Constraint> = self.data.bars.iter().map(|_| Constraint::Length(2)).collect();

        let bar_areas = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(bar_constraints)
            .split(inner);

        for (i, bar) in self.data.bars.iter().enumerate() {
            let row_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(label_width),
                    Constraint::Length(2), // Spacing between label and bar
                    Constraint::Min(10),
                    Constraint::Length(6),
                ])
                .split(bar_areas[i]);

            let label = Paragraph::new(bar.label.as_str())
                .style(Style::default().fg(bar.color).add_modifier(Modifier::BOLD))
                .alignment(ratatui::layout::Alignment::Right);
            f.render_widget(label, row_chunks[0]);

            let ratio = if max_value > 0 { bar.value as f64 / max_value as f64 } else { 0.0 };
            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(bar.color))
                .ratio(ratio)
                .label("");
            f.render_widget(gauge, row_chunks[2]);

            let value_text = Paragraph::new(format!(" {}", bar.value))
                .style(Style::default().fg(Color::White));
            f.render_widget(value_text, row_chunks[3]);
        }
    }
}