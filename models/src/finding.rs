use crate::{Severity, Status};

/// A single security finding/vulnerability.
#[derive(Clone, Debug)]
pub struct Finding {
    /// Database row id (`None` for findings not yet persisted).
    pub id: Option<i64>,
    /// Hex identifier assigned per asset, e.g. `0x001`.
    pub hex_id: String,
    /// Human-readable identifier (folder name / slug), e.g. `sql-injection`.
    pub slug: String,
    pub title: String,
    pub severity: Severity,
    /// The asset that was tested (lowercase, underscores for spaces).
    pub asset: String,
    /// Date the finding was recorded, in `YYYY/MM/DD` format.
    pub date: String,
    pub location: String,
    pub report_content: String,
    pub status: Status,
    /// Relative paths to images inside the POGDIR finding directory.
    pub images: Vec<String>,
}

impl Finding {
    pub fn new(
        title: impl Into<String>,
        severity: Severity,
        asset: impl Into<String>,
        date: impl Into<String>,
        location: impl Into<String>,
        report_content: impl Into<String>,
        status: Status,
    ) -> Self {
        let title = title.into();
        let slug = title.to_lowercase().replace(' ', "-");
        Self {
            id: None,
            hex_id: String::new(),
            slug,
            title,
            severity,
            asset: asset.into(),
            date: date.into(),
            location: location.into(),
            report_content: report_content.into(),
            status,
            images: Vec::new(),
        }
    }

    /// Convenience builder to attach image paths.
    pub fn with_images(mut self, images: Vec<String>) -> Self {
        self.images = images;
        self
    }
}
