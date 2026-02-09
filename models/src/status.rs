use ratatui::style::Color;

/// Workflow status for a security finding.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Status {
    Open,
    InProgress,
    Resolved,
    FalsePositive,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Open => "Open",
            Status::InProgress => "In Progress",
            Status::Resolved => "Resolved",
            Status::FalsePositive => "False Positive",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Status::Open => Color::Red,
            Status::InProgress => Color::Yellow,
            Status::Resolved => Color::Green,
            Status::FalsePositive => Color::Gray,
        }
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Status {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(' ', "").as_str() {
            "open" => Ok(Status::Open),
            "inprogress" => Ok(Status::InProgress),
            "resolved" => Ok(Status::Resolved),
            "falsepositive" => Ok(Status::FalsePositive),
            other => Err(format!("unknown status: {other}")),
        }
    }
}
