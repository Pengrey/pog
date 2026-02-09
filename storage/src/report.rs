use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use minijinja::Environment;
use printpdf::path::PaintMode;
use printpdf::*;

use models::{Finding, Severity};

use crate::error::{Result, StorageError};

// ═══════════════════════════════════════════════════════════════════════
// Public API
// ═══════════════════════════════════════════════════════════════════════

/// Generate a PDF report and write it to `output_path`.
///
/// The report is driven by a **MiniJinja template** that uses `#!`
/// directives.  Descriptions are parsed as **Markdown** — bold, italic,
/// inline code, fenced code blocks, headings, bullet lists, and
/// `[text](url)` links are all rendered natively.
pub fn generate_report(
    template_path: &Path,
    output_path: &Path,
    findings: &[Finding],
    asset: &str,
    from: &str,
    to: &str,
) -> Result<()> {
    // ── Read & render template ──────────────────────────────────────
    let source = fs::read_to_string(template_path).map_err(|e| {
        StorageError::ParseError(format!(
            "Cannot read template {}: {e}",
            template_path.display()
        ))
    })?;

    let mut env = Environment::new();
    env.add_template("report", &source).map_err(|e| {
        StorageError::ParseError(format!("Template syntax error: {e}"))
    })?;

    let critical = findings.iter().filter(|f| f.severity == Severity::Critical).count();
    let high     = findings.iter().filter(|f| f.severity == Severity::High).count();
    let medium   = findings.iter().filter(|f| f.severity == Severity::Medium).count();
    let low      = findings.iter().filter(|f| f.severity == Severity::Low).count();
    let info     = findings.iter().filter(|f| f.severity == Severity::Info).count();

    let findings_val: Vec<_> = findings
        .iter()
        .enumerate()
        .map(|(i, f)| {
            minijinja::context! {
                num         => i + 1,
                title       => f.title.as_str(),
                severity    => f.severity.as_str(),
                asset       => f.asset.as_str(),
                date        => f.date.as_str(),
                location    => f.location.as_str(),
                description => f.description.as_str(),
                status      => f.status.as_str(),
            }
        })
        .collect();

    let ctx = minijinja::context! {
        findings => findings_val,
        date     => current_date(),
        asset    => asset,
        from     => from,
        to       => to,
        total    => findings.len(),
        critical => critical,
        high     => high,
        medium   => medium,
        low      => low,
        info     => info,
    };

    let tmpl = env.get_template("report").map_err(|e| {
        StorageError::ParseError(format!("Template error: {e}"))
    })?;
    let rendered = tmpl.render(&ctx).map_err(|e| {
        StorageError::ParseError(format!("Template render error: {e}"))
    })?;

    let blocks = parse_blocks(&rendered);
    render_pdf(&blocks, output_path)
}

// ═══════════════════════════════════════════════════════════════════════
// Block model & parser
// ═══════════════════════════════════════════════════════════════════════

enum Block {
    Title(String),
    Subtitle(String),
    Section(String),
    /// (severity, heading) — each finding starts on a new page.
    Finding(String, String),
    /// (key, value)
    Meta(String, String),
    Table(Vec<Vec<String>>),
    /// Plain text rendered as markdown.
    Text(String),
    /// Auto-generated table of contents with page numbers.
    Index,
    Spacer(f32),
    PageBreak,
    HRule,
}

fn parse_blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut lines = text.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(directive) = trimmed.strip_prefix("#! ") {
            if let Some(rest) = directive.strip_prefix("title ") {
                blocks.push(Block::Title(rest.trim().into()));
            } else if let Some(rest) = directive.strip_prefix("subtitle ") {
                blocks.push(Block::Subtitle(rest.trim().into()));
            } else if let Some(rest) = directive.strip_prefix("section ") {
                blocks.push(Block::Section(rest.trim().into()));
            } else if let Some(rest) = directive.strip_prefix("finding ") {
                let mut parts = rest.trim().splitn(2, ' ');
                let severity = parts.next().unwrap_or("Info").to_string();
                let heading  = parts.next().unwrap_or("").to_string();
                blocks.push(Block::Finding(severity, heading));
            } else if let Some(rest) = directive.strip_prefix("meta ") {
                if let Some((k, v)) = rest.split_once(':') {
                    blocks.push(Block::Meta(k.trim().into(), v.trim().into()));
                }
            } else if directive.starts_with("table") {
                let mut rows = Vec::new();
                while let Some(peek) = lines.peek() {
                    let p = peek.trim();
                    if p.starts_with("#!") { break; }
                    if !p.is_empty() && p.contains('|') {
                        rows.push(p.split('|').map(|c| c.trim().to_string()).collect());
                    }
                    lines.next();
                }
                if !rows.is_empty() {
                    blocks.push(Block::Table(rows));
                }
            } else if directive.starts_with("index") {
                blocks.push(Block::Index);
            } else if let Some(rest) = directive.strip_prefix("spacer ") {
                blocks.push(Block::Spacer(rest.trim().parse().unwrap_or(10.0)));
            } else if directive.starts_with("comment") || directive.starts_with("image") {
                // no-op
            } else if directive.starts_with("pagebreak") {
                blocks.push(Block::PageBreak);
            } else if directive.starts_with("hr") {
                blocks.push(Block::HRule);
            }
        } else {
            // Accumulate consecutive non-empty, non-directive lines
            // but preserve individual newlines so markdown can see them.
            let mut para = trimmed.to_string();
            while let Some(peek) = lines.peek() {
                let p = peek.trim();
                if p.is_empty() || p.starts_with("#!") { break; }
                para.push('\n');
                para.push_str(p);
                lines.next();
            }
            blocks.push(Block::Text(para));
        }
    }
    blocks
}

// ═══════════════════════════════════════════════════════════════════════
// Inline Markdown model
// ═══════════════════════════════════════════════════════════════════════

/// A span of styled text within a line.
#[derive(Clone, Debug)]
enum MdSpan {
    Plain(String),
    Bold(String),
    Italic(String),
    BoldItalic(String),
    Code(String),
    /// (display text, url)
    Link(String, String),
}

/// A block-level markdown element.
enum MdBlock {
    Paragraph(Vec<MdSpan>),
    Heading(u8, Vec<MdSpan>),
    BulletItem(Vec<MdSpan>),
    CodeBlock(Vec<String>),
}

/// Parse a markdown string into block-level elements.
fn parse_markdown(text: &str) -> Vec<MdBlock> {
    let mut blocks = Vec::new();
    let mut lines = text.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        // Fenced code block.
        if trimmed.starts_with("```") {
            let mut code_lines = Vec::new();
            for next in lines.by_ref() {
                if next.trim().starts_with("```") { break; }
                code_lines.push(next.to_string());
            }
            blocks.push(MdBlock::CodeBlock(code_lines));
            continue;
        }

        // Heading.
        if let Some(rest) = trimmed.strip_prefix("### ") {
            blocks.push(MdBlock::Heading(3, parse_inline_spans(rest)));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            blocks.push(MdBlock::Heading(2, parse_inline_spans(rest)));
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            blocks.push(MdBlock::Heading(1, parse_inline_spans(rest)));
        }
        // Bullet list item.
        else if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            blocks.push(MdBlock::BulletItem(parse_inline_spans(rest)));
        }
        // Empty line — skip.
        else if trimmed.is_empty() {
            continue;
        }
        // Paragraph — accumulate with inline continuation.
        else {
            blocks.push(MdBlock::Paragraph(parse_inline_spans(trimmed)));
        }
    }
    blocks
}

/// Parse inline markdown: **bold**, *italic*, ***bold-italic***,
/// `code`, [text](url).
fn parse_inline_spans(text: &str) -> Vec<MdSpan> {
    let mut spans = Vec::new();
    let mut buf = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // ── [text](url) ──
        if chars[i] == '['
            && let Some((display, url, end)) = try_parse_link(&chars, i)
        {
            if !buf.is_empty() {
                spans.push(MdSpan::Plain(std::mem::take(&mut buf)));
            }
            spans.push(MdSpan::Link(display, url));
            i = end;
            continue;
        }

        // ── backtick inline code ──
        if chars[i] == '`'
            && let Some((code, end)) = extract_delimited(&chars, i, '`', 1)
        {
            if !buf.is_empty() {
                spans.push(MdSpan::Plain(std::mem::take(&mut buf)));
            }
            spans.push(MdSpan::Code(code));
            i = end;
            continue;
        }

        // ── *** bold italic *** ──
        if i + 2 < len && chars[i] == '*' && chars[i+1] == '*' && chars[i+2] == '*'
            && let Some((inner, end)) = extract_between(&chars, i, "***")
        {
            if !buf.is_empty() {
                spans.push(MdSpan::Plain(std::mem::take(&mut buf)));
            }
            spans.push(MdSpan::BoldItalic(inner));
            i = end;
            continue;
        }

        // ── ** bold ** ──
        if i + 1 < len && chars[i] == '*' && chars[i+1] == '*'
            && let Some((inner, end)) = extract_between(&chars, i, "**")
        {
            if !buf.is_empty() {
                spans.push(MdSpan::Plain(std::mem::take(&mut buf)));
            }
            spans.push(MdSpan::Bold(inner));
            i = end;
            continue;
        }

        // ── * italic * ──
        if chars[i] == '*'
            && let Some((inner, end)) = extract_between(&chars, i, "*")
        {
            if !buf.is_empty() {
                spans.push(MdSpan::Plain(std::mem::take(&mut buf)));
            }
            spans.push(MdSpan::Italic(inner));
            i = end;
            continue;
        }

        buf.push(chars[i]);
        i += 1;
    }

    if !buf.is_empty() {
        spans.push(MdSpan::Plain(buf));
    }
    spans
}

fn try_parse_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    // [display](url)
    let mut i = start + 1;
    let len = chars.len();
    let mut display = String::new();
    while i < len && chars[i] != ']' {
        display.push(chars[i]);
        i += 1;
    }
    if i >= len || chars[i] != ']' { return None; }
    i += 1; // skip ']'
    if i >= len || chars[i] != '(' { return None; }
    i += 1; // skip '('
    let mut url = String::new();
    while i < len && chars[i] != ')' {
        url.push(chars[i]);
        i += 1;
    }
    if i >= len { return None; }
    i += 1; // skip ')'
    Some((display, url, i))
}

fn extract_delimited(chars: &[char], start: usize, delim: char, skip: usize) -> Option<(String, usize)> {
    let mut i = start + skip;
    let len = chars.len();
    let mut inner = String::new();
    while i < len {
        if chars[i] == delim {
            return Some((inner, i + 1));
        }
        inner.push(chars[i]);
        i += 1;
    }
    None
}

fn extract_between(chars: &[char], start: usize, delim: &str) -> Option<(String, usize)> {
    let dlen = delim.len();
    let dchars: Vec<char> = delim.chars().collect();
    let mut i = start + dlen;
    let len = chars.len();
    let mut inner = String::new();
    while i + dlen <= len {
        if chars[i..i+dlen] == dchars[..] {
            if !inner.is_empty() {
                return Some((inner, i + dlen));
            }
            return None;
        }
        inner.push(chars[i]);
        i += 1;
    }
    None
}

// ═══════════════════════════════════════════════════════════════════════
// PDF renderer
// ═══════════════════════════════════════════════════════════════════════

const PAGE_W: f32 = 210.0;
const PAGE_H: f32 = 297.0;
const MARGIN: f32 = 25.0;
const CONTENT_W: f32 = PAGE_W - 2.0 * MARGIN;
const BOTTOM_MARGIN: f32 = 20.0;
/// Maximum Y before we must start a new page.
const MAX_Y: f32 = PAGE_H - MARGIN - BOTTOM_MARGIN;

const COLOR_BLUE:      (f32,f32,f32) = (0.12, 0.30, 0.78);
const COLOR_DARK_BLUE: (f32,f32,f32) = (0.08, 0.18, 0.50);
const COLOR_GRAY:      (f32,f32,f32) = (0.45, 0.45, 0.45);
const COLOR_LIGHT_GRAY:(f32,f32,f32) = (0.65, 0.65, 0.65);
const COLOR_BLACK:     (f32,f32,f32) = (0.12, 0.12, 0.12);
const COLOR_LINE:      (f32,f32,f32) = (0.78, 0.78, 0.78);
const COLOR_WHITE:     (f32,f32,f32) = (1.0, 1.0, 1.0);
const COLOR_ROW_BG:    (f32,f32,f32) = (0.94, 0.95, 0.97);
const COLOR_HDR_BG:    (f32,f32,f32) = (0.12, 0.30, 0.78);
const COLOR_CARD_BG:   (f32,f32,f32) = (0.97, 0.97, 0.98);
const COLOR_FOOTER:    (f32,f32,f32) = (0.55, 0.55, 0.55);
const COLOR_CODE_BG:   (f32,f32,f32) = (0.94, 0.94, 0.96);
const COLOR_LINK:      (f32,f32,f32) = (0.10, 0.35, 0.82);

fn severity_color(name: &str) -> (f32,f32,f32) {
    match name {
        "Critical" => (0.75, 0.05, 0.05),
        "High"     => (0.90, 0.40, 0.05),
        "Medium"   => (0.78, 0.62, 0.05),
        "Low"      => (0.12, 0.52, 0.22),
        "Info"     => (0.20, 0.42, 0.85),
        _          => COLOR_BLACK,
    }
}

fn severity_bg_color(name: &str) -> (f32,f32,f32) {
    match name {
        "Critical" => (1.0, 0.92, 0.92),
        "High"     => (1.0, 0.94, 0.88),
        "Medium"   => (1.0, 0.97, 0.88),
        "Low"      => (0.90, 0.98, 0.92),
        "Info"     => (0.90, 0.94, 1.0),
        _          => COLOR_CARD_BG,
    }
}

// ── TOC entry ───────────────────────────────────────────────────────

struct TocEntry {
    is_section: bool,
    label: String,
    severity: String,
    page_num: u32,
}

/// Two-pass render: first pass collects page numbers, second pass
/// renders with real TOC data.
fn render_pdf(blocks: &[Block], output_path: &Path) -> Result<()> {
    // ── Pass 1: simulate layout to learn page numbers ───────────────
    let toc = simulate_toc(blocks);

    // ── Pass 2: real render ─────────────────────────────────────────
    let (doc, page, layer) = PdfDocument::new(
        "Security Assessment Report",
        Mm(PAGE_W),
        Mm(PAGE_H),
        "Layer 1",
    );

    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| StorageError::ParseError(format!("Font error: {e}")))?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| StorageError::ParseError(format!("Font error: {e}")))?;
    let font_oblique = doc
        .add_builtin_font(BuiltinFont::HelveticaOblique)
        .map_err(|e| StorageError::ParseError(format!("Font error: {e}")))?;
    let font_bold_oblique = doc
        .add_builtin_font(BuiltinFont::HelveticaBoldOblique)
        .map_err(|e| StorageError::ParseError(format!("Font error: {e}")))?;
    let font_courier = doc
        .add_builtin_font(BuiltinFont::Courier)
        .map_err(|e| StorageError::ParseError(format!("Font error: {e}")))?;

    let mut r = Rpt {
        doc, font, font_bold, font_oblique, font_bold_oblique, font_courier,
        page, layer,
        y: MARGIN,
        page_num: 1,
        pages: vec![(page, layer)],
        finding_count: 0,
    };

    for block in blocks {
        match block {
            Block::Title(t)      => r.render_title(t),
            Block::Subtitle(t)   => r.render_subtitle(t),
            Block::Section(t)    => r.render_section(t),
            Block::Finding(s, h) => r.render_finding(s, h),
            Block::Meta(k, v)    => r.render_meta(k, v),
            Block::Table(rows)   => r.render_table(rows),
            Block::Text(t)       => r.render_markdown(t),
            Block::Index         => r.render_index(&toc),
            Block::Spacer(mm)    => { r.ensure_space(*mm); r.spacing(*mm); }
            Block::PageBreak     => r.new_page(),
            Block::HRule         => { r.ensure_space(6.0); r.hline(COLOR_LINE, 0.6); r.spacing(6.0); }
        }
    }

    // Add bookmarks for PDF sidebar navigation.
    r.add_bookmarks(blocks);

    // Page footers on every page.
    let total = r.page_num;
    r.render_footers(total);

    let file = File::create(output_path)?;
    r.doc
        .save(&mut BufWriter::new(file))
        .map_err(|e| StorageError::ParseError(format!("PDF write error: {e}")))?;

    Ok(())
}

/// Simulated layout pass — identical logic to real render but only
/// tracks the page counter to assign page numbers to sections and
/// findings.
fn simulate_toc(blocks: &[Block]) -> Vec<TocEntry> {
    let mut entries = Vec::new();
    let mut page: u32 = 1;
    let mut y: f32 = MARGIN;
    let mut finding_count: u32 = 0;

    let new_page = |p: &mut u32, y: &mut f32| { *p += 1; *y = MARGIN; };
    let ensure = |p: &mut u32, y: &mut f32, need: f32| {
        if *y + need > MAX_Y { new_page(p, y); }
    };

    for block in blocks {
        match block {
            Block::Title(_) => { y = 115.0; }
            Block::Subtitle(_) => { y += 12.0; }
            Block::Section(t) => {
                ensure(&mut page, &mut y, 18.0);
                entries.push(TocEntry {
                    is_section: true,
                    label: t.clone(),
                    severity: String::new(),
                    page_num: page,
                });
                y += 19.0;
            }
            Block::Finding(s, h) => {
                // Each finding starts on a new page (except possibly
                // the first one right after the section heading).
                finding_count += 1;
                if finding_count > 1 {
                    new_page(&mut page, &mut y);
                }
                ensure(&mut page, &mut y, 30.0);
                entries.push(TocEntry {
                    is_section: false,
                    label: h.clone(),
                    severity: s.clone(),
                    page_num: page,
                });
                y += 24.0;
            }
            Block::Meta(_, _) => { ensure(&mut page, &mut y, 6.0); y += 5.0; }
            Block::Table(rows) => {
                y += 9.5; // header
                for _ in rows.iter().skip(1) {
                    ensure(&mut page, &mut y, 7.5);
                    y += 7.0;
                }
                y += 5.0;
            }
            Block::Text(t) => {
                let md = parse_markdown(t);
                for mb in &md {
                    match mb {
                        MdBlock::Paragraph(spans) => {
                            let text = spans_to_plain(spans);
                            let lines = wrap_text(&text, 90);
                            for _ in &lines {
                                ensure(&mut page, &mut y, 5.0);
                                y += 4.5;
                            }
                            y += 3.0;
                        }
                        MdBlock::Heading(_, _) => {
                            ensure(&mut page, &mut y, 10.0);
                            y += 9.0;
                        }
                        MdBlock::BulletItem(spans) => {
                            let text = spans_to_plain(spans);
                            let lines = wrap_text(&text, 85);
                            for _ in &lines {
                                ensure(&mut page, &mut y, 5.0);
                                y += 4.5;
                            }
                            y += 1.5;
                        }
                        MdBlock::CodeBlock(code_lines) => {
                            y += 2.0;
                            for _ in code_lines {
                                ensure(&mut page, &mut y, 5.0);
                                y += 4.2;
                            }
                            y += 4.0;
                        }
                    }
                }
            }
            Block::Index => {
                // Rough height estimate for index — can't be exact since
                // entries aren't known yet but index is before the content.
                y += 6.0;
            }
            Block::Spacer(mm) => { ensure(&mut page, &mut y, *mm); y += mm; }
            Block::PageBreak => { new_page(&mut page, &mut y); }
            Block::HRule => { ensure(&mut page, &mut y, 6.0); y += 6.0; }
        }
    }
    entries
}

fn spans_to_plain(spans: &[MdSpan]) -> String {
    let mut s = String::new();
    for span in spans {
        match span {
            MdSpan::Plain(t)
            | MdSpan::Bold(t)
            | MdSpan::Italic(t)
            | MdSpan::BoldItalic(t)
            | MdSpan::Code(t) => s.push_str(t),
            MdSpan::Link(display, _) => s.push_str(display),
        }
    }
    s
}

// ─── Rpt state ──────────────────────────────────────────────────────

struct Rpt {
    doc: PdfDocumentReference,
    font: IndirectFontRef,
    font_bold: IndirectFontRef,
    font_oblique: IndirectFontRef,
    font_bold_oblique: IndirectFontRef,
    font_courier: IndirectFontRef,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    y: f32,
    page_num: u32,
    pages: Vec<(PdfPageIndex, PdfLayerIndex)>,
    finding_count: u32,
}

impl Rpt {
    fn layer(&self) -> PdfLayerReference {
        self.doc.get_page(self.page).get_layer(self.layer)
    }

    fn ay(&self) -> f32 { PAGE_H - self.y }

    fn new_page(&mut self) {
        let (p, l) = self.doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Layer 1");
        self.page = p;
        self.layer = l;
        self.y = MARGIN;
        self.page_num += 1;
        self.pages.push((p, l));
    }

    fn ensure_space(&mut self, needed: f32) {
        if self.y + needed > MAX_Y { self.new_page(); }
    }

    fn spacing(&mut self, mm: f32) { self.y += mm; }

    // ── Drawing primitives ──────────────────────────────────────────

    fn put_text(&self, s: &str, size: f32, x: f32, color: (f32,f32,f32), font: &IndirectFontRef) {
        let l = self.layer();
        l.set_fill_color(rgb(color));
        l.use_text(s, size, Mm(x), Mm(self.ay()), font);
    }

    fn hline(&self, color: (f32,f32,f32), thickness: f32) {
        self.hline_at(MARGIN, PAGE_W - MARGIN, self.y, color, thickness);
    }

    fn hline_at(&self, x1: f32, x2: f32, y_top: f32, color: (f32,f32,f32), thickness: f32) {
        let l = self.layer();
        l.set_outline_color(rgb(color));
        l.set_outline_thickness(thickness);
        let line = Line {
            points: vec![
                (Point::new(Mm(x1), Mm(PAGE_H - y_top)), false),
                (Point::new(Mm(x2), Mm(PAGE_H - y_top)), false),
            ],
            is_closed: false,
        };
        l.add_line(line);
    }

    fn filled_rect(&self, x: f32, y_top: f32, w: f32, h: f32, color: (f32,f32,f32)) {
        let l = self.layer();
        l.set_fill_color(rgb(color));
        let y_bot = PAGE_H - y_top - h;
        let rect = Rect::new(Mm(x), Mm(y_bot), Mm(x + w), Mm(y_bot + h))
            .with_mode(PaintMode::Fill);
        l.add_rect(rect);
    }

    /// Add a clickable URL annotation on the current page.
    fn add_link_rect(&self, x: f32, y_top: f32, w: f32, h: f32, url: &str) {
        let y_bot_mm = PAGE_H - y_top - h;
        let rect = Rect::new(
            Mm(x), Mm(y_bot_mm),
            Mm(x + w), Mm(y_bot_mm + h),
        );
        let annot = LinkAnnotation::new(
            rect,
            Some(BorderArray::Solid([0.0, 0.0, 0.0])), // no visible border
            Some(ColorArray::Transparent),
            Actions::uri(url.to_string()),
            Some(HighlightingMode::None),
        );
        self.layer().add_link_annotation(annot);
    }

    // ── Page footers ────────────────────────────────────────────────

    fn render_footers(&self, total_pages: u32) {
        for (pg, (page_idx, layer_idx)) in self.pages.iter().enumerate() {
            let page_ref = self.doc.get_page(*page_idx);
            let layer_ref = page_ref.get_layer(*layer_idx);

            layer_ref.set_outline_color(rgb(COLOR_LINE));
            layer_ref.set_outline_thickness(0.4);
            let fl = Line {
                points: vec![
                    (Point::new(Mm(MARGIN), Mm(BOTTOM_MARGIN + 2.0)), false),
                    (Point::new(Mm(PAGE_W - MARGIN), Mm(BOTTOM_MARGIN + 2.0)), false),
                ],
                is_closed: false,
            };
            layer_ref.add_line(fl);

            let label = format!("Page {} of {}", pg + 1, total_pages);
            layer_ref.set_fill_color(rgb(COLOR_FOOTER));
            layer_ref.use_text(&label, 7.5, Mm(PAGE_W - MARGIN - 22.0), Mm(BOTTOM_MARGIN - 1.0), &self.font);

            layer_ref.set_fill_color(rgb(COLOR_FOOTER));
            layer_ref.use_text("Generated by pog", 7.5, Mm(MARGIN), Mm(BOTTOM_MARGIN - 1.0), &self.font_oblique);
        }
    }

    /// Add PDF sidebar bookmarks for sections.
    fn add_bookmarks(&self, blocks: &[Block]) {
        // Bookmarks map page_index → name.  printpdf only supports
        // one bookmark per page so we assign the first heading that
        // landed on each page.
        let mut seen_pages = std::collections::HashSet::new();
        let mut page: u32 = 1;
        let mut y: f32 = MARGIN;
        let mut fc: u32 = 0;

        let np = |p: &mut u32, yy: &mut f32| { *p += 1; *yy = MARGIN; };
        let es = |p: &mut u32, yy: &mut f32, n: f32| {
            if *yy + n > MAX_Y { np(p, yy); }
        };

        for block in blocks {
            match block {
                Block::Title(_) => { y = 115.0; }
                Block::Subtitle(_) => { y += 12.0; }
                Block::Section(t) => {
                    es(&mut page, &mut y, 18.0);
                    let pi = (page - 1) as usize;
                    if pi < self.pages.len() && seen_pages.insert(pi) {
                        self.doc.add_bookmark(t.as_str(), self.pages[pi].0);
                    }
                    y += 19.0;
                }
                Block::Finding(_, h) => {
                    fc += 1;
                    if fc > 1 { np(&mut page, &mut y); }
                    es(&mut page, &mut y, 30.0);
                    let pi = (page - 1) as usize;
                    if pi < self.pages.len() && seen_pages.insert(pi) {
                        self.doc.add_bookmark(h.as_str(), self.pages[pi].0);
                    }
                    y += 24.0;
                }
                Block::Meta(_, _) => { es(&mut page, &mut y, 6.0); y += 5.0; }
                Block::Table(rows) => {
                    y += 9.5;
                    for _ in rows.iter().skip(1) { es(&mut page, &mut y, 7.5); y += 7.0; }
                    y += 5.0;
                }
                Block::Text(_) => { y += 8.0; } // rough approx for bookmark pass
                Block::Index => { y += 6.0; }
                Block::Spacer(mm) => { es(&mut page, &mut y, *mm); y += mm; }
                Block::PageBreak => { np(&mut page, &mut y); }
                Block::HRule => { es(&mut page, &mut y, 6.0); y += 6.0; }
            }
        }
    }

    // ── Block renderers ─────────────────────────────────────────────

    fn render_title(&mut self, text: &str) {
        let fb = self.font_bold.clone();
        self.filled_rect(0.0, 0.0, PAGE_W, 8.0, COLOR_BLUE);
        self.y = 85.0;

        let lines = wrap_text(text, 35);
        let block_h = lines.len() as f32 * 14.0 + 2.0;
        self.filled_rect(MARGIN, self.y - 2.0, 3.0, block_h, COLOR_BLUE);

        for part in &lines {
            self.put_text(part, 30.0, MARGIN + 8.0, COLOR_DARK_BLUE, &fb);
            self.y += 14.0;
        }
        self.y += 4.0;
    }

    fn render_subtitle(&mut self, text: &str) {
        let f = self.font.clone();
        self.put_text(text, 14.0, MARGIN + 8.0, COLOR_GRAY, &f);
        self.y += 12.0;
    }

    fn render_section(&mut self, text: &str) {
        self.ensure_space(18.0);
        let fb = self.font_bold.clone();
        self.filled_rect(MARGIN, self.y - 0.5, CONTENT_W, 0.8, COLOR_LINE);
        self.y += 5.0;
        self.put_text(text, 16.0, MARGIN, COLOR_DARK_BLUE, &fb);
        self.y += 8.0;
        self.hline_at(MARGIN, MARGIN + 50.0, self.y, COLOR_BLUE, 1.8);
        self.y += 6.0;
    }

    fn render_finding(&mut self, severity: &str, heading: &str) {
        // Each finding starts on a new page (except possibly the
        // very first one right after "Detailed Findings" heading).
        self.finding_count += 1;
        if self.finding_count > 1 {
            self.new_page();
        }

        self.ensure_space(30.0);
        let fb = self.font_bold.clone();
        let sev_color = severity_color(severity);
        let sev_bg    = severity_bg_color(severity);

        // Left accent bar + card background.
        self.filled_rect(MARGIN, self.y, 3.0, 18.0, sev_color);
        self.filled_rect(MARGIN + 3.0, self.y, CONTENT_W - 3.0, 18.0, sev_bg);

        // Severity badge pill.
        let badge_x = MARGIN + 7.0;
        let badge_y = self.y + 2.5;
        let badge_text = severity.to_uppercase();
        let badge_w = badge_text.len() as f32 * 2.2 + 8.0;
        self.filled_rect(badge_x, badge_y, badge_w, 5.5, sev_color);
        self.put_text(&badge_text, 7.5, badge_x + 2.5, COLOR_WHITE, &fb);
        self.y += 4.0;

        // Heading text.
        self.y += 7.0;
        let heading_trunc = truncate(heading, 75);
        self.put_text(&heading_trunc, 12.0, MARGIN + 7.0, COLOR_BLACK, &fb);
        self.y += 10.0;
    }

    fn render_meta(&mut self, key: &str, value: &str) {
        self.ensure_space(6.0);
        let f  = self.font.clone();
        let fb = self.font_bold.clone();

        self.put_text("\u{2022}", 8.0, MARGIN + 6.0, COLOR_LIGHT_GRAY, &f);
        self.put_text(&format!("{key}:"), 9.0, MARGIN + 10.0, COLOR_GRAY, &fb);

        let key_indent = MARGIN + 12.0 + key.len() as f32 * 1.9;
        let val = truncate(value, 80);
        self.put_text(&val, 9.0, key_indent, COLOR_BLACK, &f);
        self.y += 5.0;
    }

    fn render_table(&mut self, rows: &[Vec<String>]) {
        if rows.is_empty() { return; }
        let nc = rows[0].len();
        if nc == 0 { return; }
        let col_w = CONTENT_W / nc as f32;
        let f  = self.font.clone();
        let fb = self.font_bold.clone();

        // Header.
        self.ensure_space(10.5);
        self.filled_rect(MARGIN, self.y, CONTENT_W, 7.5, COLOR_HDR_BG);
        self.y += 5.0;
        for (i, cell) in rows[0].iter().enumerate() {
            self.put_text(cell, 8.5, MARGIN + i as f32 * col_w + 3.0, COLOR_WHITE, &fb);
        }
        self.y += 4.5;

        // Data rows.
        for (ri, row) in rows.iter().skip(1).enumerate() {
            self.ensure_space(7.5);
            if ri % 2 == 0 {
                self.filled_rect(MARGIN, self.y, CONTENT_W, 6.5, COLOR_ROW_BG);
            }
            self.y += 4.5;
            for (i, cell) in row.iter().enumerate() {
                let trunc = truncate(cell, (col_w * 0.45) as usize);
                let color = severity_color(cell);
                let font = if color != COLOR_BLACK { &fb } else { &f };
                self.put_text(&trunc, 8.0, MARGIN + i as f32 * col_w + 3.0, color, font);
            }
            self.y += 2.5;
        }

        self.hline(COLOR_LINE, 0.5);
        self.y += 5.0;
    }

    // ── Markdown renderer ───────────────────────────────────────────

    fn render_markdown(&mut self, text: &str) {
        let md_blocks = parse_markdown(text);
        for mb in &md_blocks {
            match mb {
                MdBlock::Paragraph(spans) => self.render_md_spans(spans, MARGIN + 2.0),
                MdBlock::Heading(level, spans) => self.render_md_heading(*level, spans),
                MdBlock::BulletItem(spans) => self.render_md_bullet(spans),
                MdBlock::CodeBlock(lines) => self.render_md_code_block(lines),
            }
        }
    }

    /// Render a sequence of inline spans as word-wrapped lines.
    fn render_md_spans(&mut self, spans: &[MdSpan], x_start: f32) {
        // Flatten spans into words with their style, then word-wrap.
        let styled_words = spans_to_styled_words(spans);
        let lines = wrap_styled_words(&styled_words, 90);

        for line_words in &lines {
            self.ensure_space(5.0);
            let mut x = x_start;
            for sw in line_words {
                let font = self.font_for_style(&sw.style);
                let size = 9.5;
                let color = match sw.style {
                    Style::Code => COLOR_DARK_BLUE,
                    Style::Link => COLOR_LINK,
                    _ => COLOR_BLACK,
                };

                // Code background.
                if sw.style == Style::Code {
                    let code_w = sw.text.len() as f32 * 1.95 + 2.0;
                    self.filled_rect(x - 0.5, self.y - 1.0, code_w, 4.5, COLOR_CODE_BG);
                }

                self.put_text(&sw.text, size, x, color, &font);

                // Clickable link annotation.
                if sw.style == Style::Link {
                    let link_w = sw.text.len() as f32 * 2.0 + 1.0;
                    self.hline_at(x, x + link_w, self.y + 1.2, COLOR_LINK, 0.3);
                    if let Some(url) = &sw.url {
                        self.add_link_rect(x, self.y - 2.5, link_w, 5.0, url);
                    }
                }

                x += sw.text.len() as f32 * 2.0 + 2.0;
            }
            self.y += 4.5;
        }
        self.y += 3.0;
    }

    fn render_md_heading(&mut self, level: u8, spans: &[MdSpan]) {
        self.ensure_space(10.0);
        let fb = self.font_bold.clone();
        let text = spans_to_plain(spans);
        let size = match level { 1 => 14.0, 2 => 12.0, _ => 11.0 };
        self.put_text(&text, size, MARGIN + 2.0, COLOR_DARK_BLUE, &fb);
        self.y += size * 0.6 + 2.0;
    }

    fn render_md_bullet(&mut self, spans: &[MdSpan]) {
        let f = self.font.clone();
        self.ensure_space(5.0);
        self.put_text("\u{2022}", 9.0, MARGIN + 6.0, COLOR_GRAY, &f);

        // Render spans indented.
        self.render_md_spans(spans, MARGIN + 10.0);
        // Remove the extra paragraph spacing, keep bullet tight.
        self.y -= 1.5;
    }

    fn render_md_code_block(&mut self, code_lines: &[String]) {
        let fc = self.font_courier.clone();
        self.spacing(2.0);

        for line in code_lines {
            self.ensure_space(5.0);
            // Background bar.
            self.filled_rect(MARGIN + 2.0, self.y - 1.0, CONTENT_W - 4.0, 4.5, COLOR_CODE_BG);
            // Left accent.
            self.filled_rect(MARGIN + 2.0, self.y - 1.0, 1.5, 4.5, COLOR_BLUE);

            let trunc = truncate(line, 95);
            self.put_text(&trunc, 8.0, MARGIN + 6.0, COLOR_BLACK, &fc);
            self.y += 4.2;
        }
        self.y += 4.0;
    }

    fn font_for_style(&self, style: &Style) -> IndirectFontRef {
        match style {
            Style::Plain | Style::Link => self.font.clone(),
            Style::Bold                => self.font_bold.clone(),
            Style::Italic              => self.font_oblique.clone(),
            Style::BoldItalic          => self.font_bold_oblique.clone(),
            Style::Code                => self.font_courier.clone(),
        }
    }

    // ── Table of contents with page numbers ─────────────────────────

    fn render_index(&mut self, toc: &[TocEntry]) {
        if toc.is_empty() { return; }
        let fb = self.font_bold.clone();
        let f  = self.font.clone();

        for entry in toc {
            self.ensure_space(7.0);

            if entry.is_section {
                let trunc = truncate(&entry.label, 70);
                self.put_text(&trunc, 10.0, MARGIN + 4.0, COLOR_DARK_BLUE, &fb);

                // Page number — right aligned.
                let pstr = format!("{}", entry.page_num);
                self.put_text(&pstr, 10.0, PAGE_W - MARGIN - 8.0, COLOR_DARK_BLUE, &fb);

                // Dot leader between title and page number.
                let title_end = MARGIN + 4.0 + trunc.len() as f32 * 2.2 + 4.0;
                let dots_end = PAGE_W - MARGIN - 12.0;
                if dots_end > title_end {
                    let dot_count = ((dots_end - title_end) / 1.8) as usize;
                    let dots: String = ".".repeat(dot_count);
                    self.put_text(&dots, 8.0, title_end, COLOR_LINE, &f);
                }
                self.y += 6.5;
            } else {
                // Finding entry — indented with severity dot.
                let sev_color = severity_color(&entry.severity);
                self.put_text("\u{25CF}", 6.0, MARGIN + 10.0, sev_color, &fb);

                let trunc = truncate(&entry.label, 60);
                self.put_text(&trunc, 9.0, MARGIN + 16.0, COLOR_BLACK, &f);

                // Severity label.
                self.put_text(&entry.severity, 8.0, PAGE_W - MARGIN - 32.0, sev_color, &fb);

                // Page number.
                let pstr = format!("{}", entry.page_num);
                self.put_text(&pstr, 9.0, PAGE_W - MARGIN - 8.0, COLOR_GRAY, &f);

                // Dot leader.
                let title_end = MARGIN + 16.0 + trunc.len() as f32 * 1.9 + 3.0;
                let dots_end = PAGE_W - MARGIN - 35.0;
                if dots_end > title_end {
                    let dot_count = ((dots_end - title_end) / 1.8) as usize;
                    let dots: String = ".".repeat(dot_count);
                    self.put_text(&dots, 7.0, title_end, COLOR_LINE, &f);
                }
                self.y += 5.5;
            }
        }
        self.y += 4.0;
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Styled word model for word-wrapping inline markdown
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq)]
enum Style { Plain, Bold, Italic, BoldItalic, Code, Link }

#[derive(Clone, Debug)]
struct StyledWord {
    text: String,
    style: Style,
    url: Option<String>,
}

fn spans_to_styled_words(spans: &[MdSpan]) -> Vec<StyledWord> {
    let mut words = Vec::new();
    for span in spans {
        let (style, text, url) = match span {
            MdSpan::Plain(t)      => (Style::Plain, t.as_str(), None),
            MdSpan::Bold(t)       => (Style::Bold, t.as_str(), None),
            MdSpan::Italic(t)     => (Style::Italic, t.as_str(), None),
            MdSpan::BoldItalic(t) => (Style::BoldItalic, t.as_str(), None),
            MdSpan::Code(t)       => {
                // Code is kept as one "word" (not split on spaces).
                words.push(StyledWord { text: t.clone(), style: Style::Code, url: None });
                continue;
            }
            MdSpan::Link(display, u) => (Style::Link, display.as_str(), Some(u.clone())),
        };
        for w in text.split_whitespace() {
            words.push(StyledWord { text: w.into(), style: style.clone(), url: url.clone() });
        }
    }
    words
}

fn wrap_styled_words(words: &[StyledWord], max_chars: usize) -> Vec<Vec<StyledWord>> {
    let mut lines: Vec<Vec<StyledWord>> = Vec::new();
    let mut current: Vec<StyledWord> = Vec::new();
    let mut line_len: usize = 0;

    for w in words {
        let wlen = w.text.len();
        if !current.is_empty() && line_len + 1 + wlen > max_chars {
            lines.push(std::mem::take(&mut current));
            line_len = 0;
        }
        if !current.is_empty() { line_len += 1; } // space
        line_len += wlen;
        current.push(w.clone());
    }
    if !current.is_empty() { lines.push(current); }
    lines
}

// ═══════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════

fn rgb(c: (f32,f32,f32)) -> Color {
    Color::Rgb(Rgb::new(c.0, c.1, c.2, None))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else { format!("{}...", &s[..max.saturating_sub(3)]) }
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let words: Vec<&str> = paragraph.split_whitespace().collect();
        if words.is_empty() { lines.push(String::new()); continue; }
        let mut current = String::new();
        for word in words {
            if current.is_empty() {
                current = word.to_string();
            } else if current.len() + 1 + word.len() > max_chars {
                lines.push(current);
                current = word.to_string();
            } else {
                current.push(' ');
                current.push_str(word);
            }
        }
        if !current.is_empty() { lines.push(current); }
    }
    lines
}

fn current_date() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = now / 86400;
    let mut y = 1970i32;
    let mut remaining = days as i32;
    loop {
        let yd = if is_leap(y) { 366 } else { 365 };
        if remaining < yd { break; }
        remaining -= yd;
        y += 1;
    }
    let mut m = 1u32;
    loop {
        let md = month_days(m, y);
        if remaining < md { break; }
        remaining -= md;
        m += 1;
    }
    let d = remaining + 1;
    format!("{y}/{m:02}/{d:02}")
}

fn is_leap(y: i32) -> bool { y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) }

fn month_days(m: u32, y: i32) -> i32 {
    match m {
        1|3|5|7|8|10|12 => 31,
        4|6|9|11 => 30,
        2 => if is_leap(y) { 29 } else { 28 },
        _ => 30,
    }
}
