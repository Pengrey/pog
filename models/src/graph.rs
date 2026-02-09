use ratatui::style::Color;

use crate::Severity;

/// A single bar in a severity distribution graph.
#[derive(Clone, Debug)]
pub struct SeverityBar {
    pub label: String,
    pub value: u64,
    pub color: Color,
}

impl SeverityBar {
    pub fn new(label: impl Into<String>, value: u64, color: Color) -> Self {
        Self { label: label.into(), value, color }
    }

    /// Create a bar from a [`Severity`] variant, using its label and color.
    pub fn from_severity(severity: Severity, value: u64) -> Self {
        Self::new(severity.as_str(), value, severity.color())
    }
}

/// Data backing a severity distribution bar chart.
#[derive(Clone, Debug)]
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

    /// Sample graph data for demonstration / testing purposes.
    pub fn sample_severity() -> Self {
        Self::new("Severity Distribution")
            .with_bar(SeverityBar::from_severity(Severity::Critical, 3))
            .with_bar(SeverityBar::from_severity(Severity::High, 7))
            .with_bar(SeverityBar::from_severity(Severity::Medium, 12))
            .with_bar(SeverityBar::from_severity(Severity::Low, 5))
            .with_bar(SeverityBar::from_severity(Severity::Info, 2))
    }
}
