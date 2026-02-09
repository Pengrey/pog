use std::collections::BTreeMap;

use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, Gauge, GraphType, List, ListItem, Paragraph},
    Frame,
};

use models::{Finding, GraphData, Severity};

use super::Tab;

// ---------------------------------------------------------------------------
// Severity toggle filter (checkboxes, not a dropdown)
// ---------------------------------------------------------------------------

struct SeverityToggle {
    severity: Severity,
    enabled: bool,
}

// ---------------------------------------------------------------------------
// Timeline data — weekly buckets for a line / area graph
// ---------------------------------------------------------------------------

/// One point on the x-axis (a week).
struct WeekBucket {
    /// Short label, e.g. "Sep 1" or "Jan 20".
    label: String,
    /// Count of findings that fall in this week, per severity.
    severity_counts: [u32; 5],
}

impl WeekBucket {
    fn total(&self, toggles: &[SeverityToggle]) -> u32 {
        Severity::ALL
            .iter()
            .enumerate()
            .filter(|(_, s)| toggles.iter().any(|t| t.severity == **s && t.enabled))
            .map(|(i, _)| self.severity_counts[i])
            .sum()
    }
}

fn severity_index(s: Severity) -> usize {
    Severity::ALL.iter().position(|&v| v == s).unwrap_or(0)
}

/// Parse "YYYY/MM/DD" → (year, month, day).
fn parse_ymd(date: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<&str> = date.split('/').collect();
    if parts.len() < 3 { return None; }
    Some((parts[0].parse().ok()?, parts[1].parse().ok()?, parts[2].parse().ok()?))
}

/// Convert (year, month, day) → ordinal day count since an arbitrary epoch
/// (good enough for grouping into 7-day buckets).
fn day_ordinal(y: i32, m: u32, d: u32) -> i32 {
    let m = m as i32;
    let d = d as i32;
    // Rata Die–style day number (simplified, doesn't need to be exact)
    let a = (14 - m) / 12;
    let yy = y + 4800 - a;
    let mm = m + 12 * a - 3;
    d + (153 * mm + 2) / 5 + 365 * yy + yy / 4 - yy / 100 + yy / 400 - 32045
}

fn month_abbrev(m: u32) -> &'static str {
    match m {
        1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
        5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
        9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
        _ => "???",
    }
}

fn build_weekly_timeline(findings: &[Finding]) -> Vec<WeekBucket> {
    // Parse all dates into ordinal days.
    let mut entries: Vec<(i32, u32, u32, usize)> = Vec::new(); // (y, m, d, sev_idx)
    for f in findings {
        if let Some((y, m, d)) = parse_ymd(&f.date) {
            entries.push((y, m, d, severity_index(f.severity)));
        }
    }
    if entries.is_empty() { return Vec::new(); }

    let ords: Vec<i32> = entries.iter().map(|&(y, m, d, _)| day_ordinal(y, m, d)).collect();
    let min_ord = *ords.iter().min().unwrap();
    let max_ord = *ords.iter().max().unwrap();

    // Bucket width = 7 days.
    let bucket_count = ((max_ord - min_ord) / 7 + 1) as usize;

    // Group into buckets.
    let mut buckets_map: BTreeMap<usize, [u32; 5]> = BTreeMap::new();
    for (i, &ord) in ords.iter().enumerate() {
        let idx = ((ord - min_ord) / 7) as usize;
        buckets_map.entry(idx).or_insert([0; 5])[entries[i].3] += 1;
    }

    // Build continuous list from 0..bucket_count, filling gaps with zeros.
    // We need labels: use the date of the first day in each bucket.
    // Reverse-compute from ordinal is complex, so we pre-build a lookup.
    let mut date_of_bucket: Vec<(u32, u32)> = Vec::with_capacity(bucket_count);
    {
        let first = entries.iter().min_by_key(|e| day_ordinal(e.0, e.1, e.2)).unwrap();
        for b in 0..bucket_count {
            let delta = (b as i32) * 7;
            let (rm, rd) = approx_month_day(first.0, first.1, first.2, delta);
            date_of_bucket.push((rm, rd));
        }
    }

    (0..bucket_count)
        .map(|b| {
            let counts = buckets_map.get(&b).copied().unwrap_or([0; 5]);
            let (m, d) = date_of_bucket[b];
            WeekBucket {
                label: format!("{} {}", month_abbrev(m), d),
                severity_counts: counts,
            }
        })
        .collect()
}

/// Approximate month/day after adding `delta` days to (y, m, d).
fn approx_month_day(y: i32, m: u32, d: u32, delta: i32) -> (u32, u32) {
    let days_in = |mm: u32, yy: i32| -> u32 {
        match mm {
            1|3|5|7|8|10|12 => 31,
            4|6|9|11 => 30,
            2 => if yy % 4 == 0 && (yy % 100 != 0 || yy % 400 == 0) { 29 } else { 28 },
            _ => 30,
        }
    };

    let mut yy = y;
    let mut mm = m;
    let mut dd = d as i32 + delta;

    while dd > days_in(mm, yy) as i32 {
        dd -= days_in(mm, yy) as i32;
        mm += 1;
        if mm > 12 { mm = 1; yy += 1; }
    }
    while dd < 1 {
        mm = if mm == 1 { 12 } else { mm - 1 };
        if mm == 12 { yy -= 1; }
        dd += days_in(mm, yy) as i32;
    }

    (mm, dd as u32)
}

// ---------------------------------------------------------------------------
// GraphTab
// ---------------------------------------------------------------------------

pub struct GraphTab {
    data: GraphData,
    findings: Vec<Finding>,
    toggles: Vec<SeverityToggle>,
    toggle_cursor: usize,
    filter_area: Option<Rect>,
}

impl GraphTab {
    pub fn new(data: GraphData, findings: Vec<Finding>) -> Self {
        let toggles = Severity::ALL
            .iter()
            .map(|&s| SeverityToggle { severity: s, enabled: true })
            .collect();
        Self {
            data,
            findings,
            toggles,
            toggle_cursor: 0,
            filter_area: None,
        }
    }

    fn toggle_current(&mut self) {
        self.toggles[self.toggle_cursor].enabled = !self.toggles[self.toggle_cursor].enabled;
    }
}

impl Tab for GraphTab {
    fn title(&self) -> &'static str { "Graph" }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Down | KeyCode::Char('j') => {
                self.toggle_cursor = (self.toggle_cursor + 1) % self.toggles.len();
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.toggle_cursor = if self.toggle_cursor == 0 {
                    self.toggles.len() - 1
                } else {
                    self.toggle_cursor - 1
                };
                true
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.toggle_current();
                true
            }
            _ => false,
        }
    }

    fn handle_click(&mut self, col: u16, row: u16) {
        if let Some(area) = self.filter_area
            && col >= area.x && col < area.x + area.width
            && row >= area.y && row < area.y + area.height
        {
            let item_start = area.y + 1;
            if row >= item_start {
                let idx = (row - item_start) as usize;
                if idx < self.toggles.len() {
                    self.toggle_cursor = idx;
                    self.toggle_current();
                }
            }
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        // Top: severity distribution bars.  Bottom: timeline + filter.
        let bar_count = self.data.bars.len() as u16;
        let severity_height = bar_count * 2 + 3;

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(severity_height),
                Constraint::Min(10),
            ])
            .split(area);

        self.render_severity_bars(f, rows[0]);

        // Bottom row: timeline chart (left) + severity filter (right).
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(30),
                Constraint::Length(22),
            ])
            .split(rows[1]);

        self.render_timeline(f, cols[0]);
        self.render_filter(f, cols[1]);
    }
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

impl GraphTab {
    fn render_severity_bars(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", self.data.title));

        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.data.bars.is_empty() {
            let msg = Paragraph::new("No data to display")
                .alignment(Alignment::Center);
            f.render_widget(msg, inner);
            return;
        }

        let max_value = self.data.bars.iter().map(|b| b.value).max().unwrap_or(1);
        let label_width = self.data.bars.iter().map(|b| b.label.len()).max().unwrap_or(0) as u16 + 2;

        let bar_constraints: Vec<Constraint> = self.data.bars
            .iter()
            .map(|_| Constraint::Length(2))
            .collect();

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
                    Constraint::Length(2),
                    Constraint::Min(10),
                    Constraint::Length(6),
                ])
                .split(bar_areas[i]);

            let label = Paragraph::new(bar.label.as_str())
                .style(Style::default().fg(bar.color).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Right);
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

    // ── Line graph ───────────────────────────────────────────────────

    fn render_timeline(&self, f: &mut Frame, area: Rect) {
        let buckets = build_weekly_timeline(&self.findings);

        if buckets.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Findings over time ");
            let inner = block.inner(area);
            f.render_widget(block, area);
            let msg = Paragraph::new("No findings with dates")
                .alignment(Alignment::Center);
            f.render_widget(msg, inner);
            return;
        }

        let values: Vec<u32> = buckets.iter().map(|b| b.total(&self.toggles)).collect();
        let max_val = values.iter().copied().max().unwrap_or(1).max(1) as f64;

        // Build data points for the chart: (x, y) where x = bucket index, y = count.
        let data_points: Vec<(f64, f64)> = values
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as f64, v as f64))
            .collect();

        // Build x-axis labels: show month name at each month boundary.
        let n = buckets.len();
        // We'll build month boundary indices for smart label placement.
        let mut month_indices: Vec<(usize, String)> = Vec::new();
        {
            let mut prev_month = String::new();
            for (i, b) in buckets.iter().enumerate() {
                let month = b.label.split_whitespace().next().unwrap_or("").to_string();
                if month != prev_month {
                    month_indices.push((i, month.clone()));
                    prev_month = month;
                }
            }
        }

        // Build x-axis labels at evenly spaced positions showing month names.
        let x_labels: Vec<ratatui::text::Span> = month_indices
            .iter()
            .map(|(_, name)| {
                ratatui::text::Span::styled(
                    name.clone(),
                    Style::default().fg(Color::DarkGray),
                )
            })
            .collect();

        // Build y-axis labels.
        let y_step = (max_val / 4.0).ceil().max(1.0);
        let y_max = (y_step * 4.0).max(max_val);
        let y_labels: Vec<ratatui::text::Span> = (0..=4)
            .map(|i| {
                let v = (y_step * i as f64) as u32;
                ratatui::text::Span::styled(
                    format!("{v}"),
                    Style::default().fg(Color::DarkGray),
                )
            })
            .collect();

        let dataset = Dataset::default()
            .name("Findings")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Blue))
            .data(&data_points);

        let x_axis = Axis::default()
            .style(Style::default().fg(Color::DarkGray))
            .bounds([0.0, (n - 1) as f64])
            .labels(x_labels);

        let y_axis = Axis::default()
            .style(Style::default().fg(Color::DarkGray))
            .bounds([0.0, y_max])
            .labels(y_labels);

        let chart = Chart::new(vec![dataset])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Findings over time "),
            )
            .x_axis(x_axis)
            .y_axis(y_axis)
            .legend_position(None);

        f.render_widget(chart, area);
    }

    // ── Severity filter panel ───────────────────────────────────────────

    fn render_filter(&mut self, f: &mut Frame, area: Rect) {
        self.filter_area = Some(area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Filter (↑↓ Space) ");

        let items: Vec<ListItem> = self.toggles
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let check = if t.enabled { "◉" } else { "○" };
                let marker = if i == self.toggle_cursor { "▸ " } else { "  " };
                let text = format!("{marker}{check} {}", t.severity.as_str());
                let style = if i == self.toggle_cursor {
                    Style::default().fg(t.severity.color()).add_modifier(Modifier::BOLD)
                } else if t.enabled {
                    Style::default().fg(t.severity.color())
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                ListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }
}

