use ratatui::style::Color;

/// A tracked asset with metadata.
#[derive(Clone, Debug)]
pub struct Asset {
    /// Database row id (`None` for assets not yet persisted).
    pub id: Option<i64>,
    /// Unique name – lowercase, underscores for spaces.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Contact person or team responsible for the asset.
    pub contact: String,
    /// Criticality level of the asset (e.g. "Critical", "High", "Medium", "Low").
    pub criticality: String,
    /// DNS name or IP address.
    pub dns_or_ip: String,
}

impl Asset {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            description: "-".into(),
            contact: "-".into(),
            criticality: "-".into(),
            dns_or_ip: "-".into(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_contact(mut self, contact: impl Into<String>) -> Self {
        self.contact = contact.into();
        self
    }

    pub fn with_criticality(mut self, crit: impl Into<String>) -> Self {
        self.criticality = crit.into();
        self
    }

    pub fn with_dns_or_ip(mut self, dns: impl Into<String>) -> Self {
        self.dns_or_ip = dns.into();
        self
    }

    /// Map the criticality string to a TUI color.
    pub fn criticality_color(&self) -> Color {
        match self.criticality.to_lowercase().as_str() {
            "critical" => Color::Red,
            "high" => Color::LightRed,
            "medium" => Color::Yellow,
            "low" => Color::Green,
            _ => Color::Gray,
        }
    }
}
