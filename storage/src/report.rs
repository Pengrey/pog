// storage/src/report.rs — LaTeX-based PDF report generation via tectonic.
//
// The public entry point is [`generate_report`].  It renders a MiniJinja
// template (`.tmpl`), parses the result into `Block`s, converts them to a
// LaTeX document, and compiles it to PDF using the tectonic crate (an
// embedded TeX engine — no external dependencies required).

use crate::error::{Result, StorageError};
use crate::pogdir::PogDir;
use models::{Finding, Severity};
use std::fs;
use std::path::Path;

// ───────────────────────── public API ─────────────────────────

/// Generate a PDF report from a `.tmpl` template.
///
/// `findings` are numbered sequentially starting at 1 and exposed to the
/// template together with aggregate counters and the supplied metadata.
pub fn generate_report(
    findings: &[Finding],
    template_path: &str,
    output_path: &str,
    asset: &str,
    from: &str,
    to: &str,
    pog: &PogDir,
) -> Result<()> {
    let raw = fs::read_to_string(template_path)?;

    // ── set up work directory for images ──
    let work_dir = tempfile::tempdir()?;
    prepare_finding_images(findings, pog, work_dir.path())?;

    let template_dir = Path::new(template_path)
        .parent()
        .unwrap_or(Path::new("."));

    // ── build MiniJinja context ──
    let mut env = minijinja::Environment::new();

    // Register a `latex` filter so templates can safely embed variables
    // inside `#! latex` blocks:  {{ asset|latex }}
    env.add_filter("latex", |value: String| -> String {
        latex_escape(&value)
    });

    env.add_template("report", &raw)
        .map_err(|e| StorageError::TemplateError(e.to_string()))?;
    let tmpl = env
        .get_template("report")
        .map_err(|e| StorageError::TemplateError(e.to_string()))?;

    let finding_values: Vec<minijinja::Value> = findings
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let mut map = std::collections::BTreeMap::new();
            map.insert("num".to_string(), minijinja::Value::from(i as i64 + 1));
            map.insert("title".to_string(), minijinja::Value::from(f.title.as_str()));
            map.insert("severity".to_string(), minijinja::Value::from(f.severity.as_str()));
            map.insert("asset".to_string(), minijinja::Value::from(f.asset.as_str()));
            map.insert("date".to_string(), minijinja::Value::from(f.date.as_str()));
            map.insert("location".to_string(), minijinja::Value::from(f.location.as_str()));
            // Pre-process description to rewrite image paths for the work directory
            let desc = rewrite_description_images(&f.description, &f.images, &f.slug);
            map.insert("description".to_string(), minijinja::Value::from(desc));
            map.insert("status".to_string(), minijinja::Value::from(f.status.as_str()));
            // Include images list (resolved to work-directory names)
            let img_values: Vec<minijinja::Value> = f.images.iter()
                .filter_map(|img| {
                    Path::new(img).file_name()
                        .and_then(|n| n.to_str())
                        .map(|basename| {
                            minijinja::Value::from(format!("{}-{}", f.slug, basename))
                        })
                })
                .collect();
            map.insert("images".to_string(), minijinja::Value::from(img_values));
            minijinja::Value::from(map)
        })
        .collect();

    let count = |sev: Severity| -> i64 {
        findings.iter().filter(|f| f.severity == sev).count() as i64
    };

    let ctx = minijinja::context! {
        findings => finding_values,
        date => current_date(),
        asset => asset,
        from => from,
        to => to,
        total => findings.len() as i64,
        critical => count(Severity::Critical),
        high => count(Severity::High),
        medium => count(Severity::Medium),
        low => count(Severity::Low),
        info => count(Severity::Info),
    };

    let rendered = tmpl
        .render(&ctx)
        .map_err(|e| StorageError::TemplateError(e.to_string()))?;

    // ── parse blocks and render via LaTeX ──
    let blocks = parse_blocks(&rendered);
    copy_template_assets(template_dir, work_dir.path())?;
    let latex_src = blocks_to_latex(&blocks, asset);
    render_pdf(&latex_src, output_path, work_dir.path())?;

    Ok(())
}

// ───────────────────────── image helpers ─────────────────────────

/// Copy all images from the POGDIR finding directories into the work directory
/// so they can be found by tectonic during LaTeX compilation.
fn prepare_finding_images(
    findings: &[Finding],
    pog: &PogDir,
    work_dir: &Path,
) -> Result<()> {
    for f in findings {
        for img_path in &f.images {
            let basename = Path::new(img_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("image");
            let src = pog.finding_dir(&f.asset, &f.hex_id, &f.slug).join(img_path);
            let dest_name = format!("{}-{}", f.slug, basename);
            let dest = work_dir.join(&dest_name);
            if src.exists() {
                fs::copy(&src, &dest)?;
            }
        }
    }
    Ok(())
}

/// Rewrite markdown image references in a finding description so that the
/// basenames match the files copied into the work directory by
/// [`prepare_finding_images`].
fn rewrite_description_images(desc: &str, images: &[String], slug: &str) -> String {
    if images.is_empty() {
        return desc.to_string();
    }

    let mut result = String::with_capacity(desc.len());
    let mut remaining = desc;

    while let Some(pos) = remaining.find("![") {
        result.push_str(&remaining[..pos]);
        remaining = &remaining[pos..];

        if let Some((alt, path, consumed)) = parse_md_image(remaining) {
            let basename = path
                .rsplit('/')
                .next()
                .and_then(|s| s.rsplit('\\').next())
                .unwrap_or(&path);

            let matched = images.iter().any(|img| {
                let img_base = img.rsplit('/').next().unwrap_or(img);
                img_base == basename
            });

            if matched {
                let new_name = format!("{slug}-{basename}");
                result.push_str(&format!("![{alt}]({new_name})"));
                remaining = &remaining[consumed..];
                continue;
            }
        }

        // Could not parse or no match – emit "![" verbatim and continue
        result.push_str("![");
        remaining = &remaining[2..];
    }

    result.push_str(remaining);
    result
}

/// Try to parse `![alt](path)` from the start of `s`.
/// Returns `(alt, path, total_bytes_consumed)`.
fn parse_md_image(s: &str) -> Option<(String, String, usize)> {
    if !s.starts_with("![") {
        return None;
    }
    let after = &s[2..];
    let close_bracket = after.find(']')?;
    let alt = after[..close_bracket].to_string();
    let rest = &after[close_bracket + 1..];
    if !rest.starts_with('(') {
        return None;
    }
    let close_paren = rest[1..].find(')')?;
    let path = rest[1..1 + close_paren].to_string();
    let consumed = 2 + close_bracket + 1 + 1 + close_paren + 1;
    Some((alt, path, consumed))
}

/// Copy all files from the template directory into the work directory,
/// preserving relative paths so that template assets (images, styles, etc.)
/// are available during LaTeX compilation.  This makes templates fully
/// self-contained — they can reference their own images via raw LaTeX
/// without the program needing to know about them.
fn copy_template_assets(template_dir: &Path, work_dir: &Path) -> Result<()> {
    fn walk(root: &Path, dir: &Path, dest: &Path) -> std::io::Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let target = dest.join(rel);
            if path.is_dir() {
                fs::create_dir_all(&target)?;
                walk(root, &path, dest)?;
            } else {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                let _ = fs::copy(&path, &target);
            }
        }
        Ok(())
    }
    Ok(walk(template_dir, template_dir, work_dir)?)
}

// ───────────────────────── block model ─────────────────────────

/// Intermediate representation of a report element, parsed from `#!`
/// directives and plain text in the rendered template.
#[derive(Debug, PartialEq)]
enum Block {
    Title(String),
    Subtitle(String),
    Section(String),
    /// Finding card: (severity label, heading text).
    Finding(String, String),
    Meta(String, String),
    /// Table rows – first row is the header.
    Table(Vec<Vec<String>>),
    /// Free-form markdown content.
    Text(String),
    /// Raw LaTeX passthrough — inserted verbatim into the document.
    Latex(String),
    /// Auto-generated table of contents.
    Index,
    /// Vertical spacer (millimetres).
    Spacer(f32),
    PageBreak,
    HRule,
}

// ───────────────────────── block parser ─────────────────────────

/// Parse the rendered template text into a sequence of [`Block`]s.
fn parse_blocks(text: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut text_buf = String::new();
    let mut in_latex = false;
    let mut latex_buf = String::new();

    let flush_text = |buf: &mut String, out: &mut Vec<Block>| {
        let trimmed = buf.trim().to_string();
        if !trimmed.is_empty() {
            out.push(Block::Text(trimmed));
        }
        buf.clear();
    };

    let flush_table = |rows: &mut Vec<Vec<String>>, out: &mut Vec<Block>| {
        if !rows.is_empty() {
            out.push(Block::Table(std::mem::take(rows)));
        }
    };

    for line in text.lines() {
        let trimmed = line.trim();

        // ── raw LaTeX block: collect lines verbatim until #! endlatex ──
        if in_latex {
            if trimmed == "#! endlatex" {
                if !latex_buf.is_empty() {
                    blocks.push(Block::Latex(latex_buf.trim_end().to_string()));
                }
                latex_buf.clear();
                in_latex = false;
            } else {
                if !latex_buf.is_empty() {
                    latex_buf.push('\n');
                }
                latex_buf.push_str(line);
            }
            continue;
        }

        // Blank lines between text paragraphs get preserved inside the
        // text buffer as empty lines.
        if trimmed.is_empty() {
            if !text_buf.is_empty() {
                text_buf.push('\n');
            }
            continue;
        }

        // ── directive lines ──
        if let Some(rest) = trimmed.strip_prefix("#!") {
            flush_text(&mut text_buf, &mut blocks);

            let rest = rest.trim();
            if let Some(arg) = rest.strip_prefix("title ") {
                flush_table(&mut table_rows, &mut blocks);
                blocks.push(Block::Title(arg.trim().to_string()));
            } else if let Some(arg) = rest.strip_prefix("subtitle ") {
                flush_table(&mut table_rows, &mut blocks);
                blocks.push(Block::Subtitle(arg.trim().to_string()));
            } else if let Some(arg) = rest.strip_prefix("section ") {
                flush_table(&mut table_rows, &mut blocks);
                blocks.push(Block::Section(arg.trim().to_string()));
            } else if let Some(arg) = rest.strip_prefix("finding ") {
                flush_table(&mut table_rows, &mut blocks);
                let arg = arg.trim();
                if let Some(pos) = arg.find(' ') {
                    let sev = arg[..pos].to_string();
                    let heading = arg[pos + 1..].trim().to_string();
                    blocks.push(Block::Finding(sev, heading));
                }
            } else if let Some(arg) = rest.strip_prefix("meta ") {
                flush_table(&mut table_rows, &mut blocks);
                if let Some(pos) = arg.find(':') {
                    let key = arg[..pos].trim().to_string();
                    let val = arg[pos + 1..].trim().to_string();
                    blocks.push(Block::Meta(key, val));
                }
            } else if rest == "index" {
                flush_table(&mut table_rows, &mut blocks);
                blocks.push(Block::Index);
            } else if let Some(arg) = rest.strip_prefix("spacer ") {
                flush_table(&mut table_rows, &mut blocks);
                if let Ok(mm) = arg.trim().parse::<f32>() {
                    blocks.push(Block::Spacer(mm));
                }
            } else if rest == "pagebreak" {
                flush_table(&mut table_rows, &mut blocks);
                blocks.push(Block::PageBreak);
            } else if rest == "hr" {
                flush_table(&mut table_rows, &mut blocks);
                blocks.push(Block::HRule);
            } else if rest == "latex" {
                flush_table(&mut table_rows, &mut blocks);
                in_latex = true;
                latex_buf.clear();
            } else if let Some(arg) = rest.strip_prefix("latex ") {
                flush_table(&mut table_rows, &mut blocks);
                blocks.push(Block::Latex(arg.to_string()));
            }
            // #! comment lines are silently ignored
            continue;
        }

        // ── pipe-delimited table row ──
        if trimmed.contains('|') && !trimmed.starts_with('-') {
            // Strip leading/trailing empty cells produced by lines
            // like `| A | B | C |` (the split yields ["", "A", "B", "C", ""]).
            let mut cols: Vec<String> = trimmed.split('|').map(|c| c.trim().to_string()).collect();
            if cols.first().map_or(false, |c| c.is_empty()) {
                cols.remove(0);
            }
            if cols.last().map_or(false, |c| c.is_empty()) {
                cols.pop();
            }

            // Skip markdown separator lines like `|---|---|---|`.
            // Every cell consists solely of dashes and optional colons.
            let is_separator = !cols.is_empty() && cols.iter().all(|c| {
                !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':')
            });
            if is_separator {
                if table_rows.is_empty() {
                    flush_text(&mut text_buf, &mut blocks);
                }
                continue;
            }

            flush_text(&mut text_buf, &mut blocks);
            table_rows.push(cols);
            continue;
        }

        // ── plain text ──
        flush_table(&mut table_rows, &mut blocks);
        if !text_buf.is_empty() {
            text_buf.push('\n');
        }
        text_buf.push_str(trimmed);
    }

    flush_text(&mut text_buf, &mut blocks);
    flush_table(&mut table_rows, &mut blocks);

    blocks
}

// ───────────────────────── markdown model ─────────────────────────

/// A block-level markdown element.
#[derive(Debug, PartialEq)]
enum MdBlock {
    Paragraph(Vec<MdSpan>),
    Heading(u8, Vec<MdSpan>),
    BulletItem(Vec<MdSpan>),
    CodeBlock(String),
}

/// An inline markdown span.
#[derive(Debug, PartialEq, Clone)]
enum MdSpan {
    Plain(String),
    Bold(String),
    Italic(String),
    BoldItalic(String),
    Code(String),
    Link(String, String), // (display, url)
    Image(String, String), // (alt text, file path)
}

// ───────────────────────── markdown parser ─────────────────────────

/// Parse multi-line markdown text into block-level elements.
fn parse_markdown(text: &str) -> Vec<MdBlock> {
    let mut out = Vec::new();
    let mut in_code = false;
    let mut code_buf = String::new();
    let mut para_buf = String::new();

    let flush_para = |buf: &mut String, out: &mut Vec<MdBlock>| {
        let t = buf.trim().to_string();
        if !t.is_empty() {
            out.push(MdBlock::Paragraph(parse_inline_spans(&t)));
        }
        buf.clear();
    };

    for line in text.lines() {
        let trimmed = line.trim();

        // ── fenced code blocks ──
        if trimmed.starts_with("```") {
            if in_code {
                out.push(MdBlock::CodeBlock(code_buf.trim_end().to_string()));
                code_buf.clear();
                in_code = false;
            } else {
                flush_para(&mut para_buf, &mut out);
                in_code = true;
            }
            continue;
        }
        if in_code {
            if !code_buf.is_empty() {
                code_buf.push('\n');
            }
            code_buf.push_str(line);
            continue;
        }

        // ── blank line → flush paragraph ──
        if trimmed.is_empty() {
            flush_para(&mut para_buf, &mut out);
            continue;
        }

        // ── headings ──
        if let Some(rest) = trimmed.strip_prefix("### ") {
            flush_para(&mut para_buf, &mut out);
            out.push(MdBlock::Heading(3, parse_inline_spans(rest)));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            flush_para(&mut para_buf, &mut out);
            out.push(MdBlock::Heading(2, parse_inline_spans(rest)));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# ") {
            flush_para(&mut para_buf, &mut out);
            out.push(MdBlock::Heading(1, parse_inline_spans(rest)));
            continue;
        }

        // ── bullet list items ──
        if let Some(rest) = trimmed.strip_prefix("- ") {
            flush_para(&mut para_buf, &mut out);
            out.push(MdBlock::BulletItem(parse_inline_spans(rest)));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("* ") {
            flush_para(&mut para_buf, &mut out);
            out.push(MdBlock::BulletItem(parse_inline_spans(rest)));
            continue;
        }

        // ── regular paragraph text ──
        if !para_buf.is_empty() {
            para_buf.push(' ');
        }
        para_buf.push_str(trimmed);
    }

    flush_para(&mut para_buf, &mut out);
    if in_code && !code_buf.is_empty() {
        out.push(MdBlock::CodeBlock(code_buf.trim_end().to_string()));
    }

    out
}

// ───────────────────────── inline span parser ─────────────────────────

/// Parse inline markdown spans from a single logical line.
fn parse_inline_spans(text: &str) -> Vec<MdSpan> {
    let mut spans = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut plain = String::new();

    let flush_plain = |p: &mut String, out: &mut Vec<MdSpan>| {
        if !p.is_empty() {
            out.push(MdSpan::Plain(std::mem::take(p)));
        }
    };

    while i < len {
        // ── image: ![alt](path) ──
        if chars[i] == '!'
            && i + 1 < len
            && chars[i + 1] == '['
            && let Some((alt, path, end)) = try_parse_link(&chars, i + 1) {
                flush_plain(&mut plain, &mut spans);
                spans.push(MdSpan::Image(alt, path));
                i = end;
                continue;
            }

        // ── link: [text](url) ──
        if chars[i] == '['
            && let Some((display, url, end)) = try_parse_link(&chars, i) {
                flush_plain(&mut plain, &mut spans);
                spans.push(MdSpan::Link(display, url));
                i = end;
                continue;
            }

        // ── backtick inline code ──
        if chars[i] == '`'
            && let Some((content, end)) = extract_delimited(&chars, i, '`') {
                flush_plain(&mut plain, &mut spans);
                spans.push(MdSpan::Code(content));
                i = end;
                continue;
            }

        // ── bold italic *** ──
        if i + 2 < len && chars[i] == '*' && chars[i + 1] == '*' && chars[i + 2] == '*'
            && let Some((content, end)) = extract_between(&chars, i + 3, "***") {
                flush_plain(&mut plain, &mut spans);
                spans.push(MdSpan::BoldItalic(content));
                i = end;
                continue;
            }

        // ── bold ** ──
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*'
            && let Some((content, end)) = extract_between(&chars, i + 2, "**") {
                flush_plain(&mut plain, &mut spans);
                spans.push(MdSpan::Bold(content));
                i = end;
                continue;
            }

        // ── italic * ──
        if chars[i] == '*'
            && let Some((content, end)) = extract_between(&chars, i + 1, "*") {
                flush_plain(&mut plain, &mut spans);
                spans.push(MdSpan::Italic(content));
                i = end;
                continue;
            }

        plain.push(chars[i]);
        i += 1;
    }

    flush_plain(&mut plain, &mut spans);
    spans
}

/// Try to parse `[display](url)` starting at position `start`.
fn try_parse_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    // `start` must point at '['
    let mut i = start + 1;
    let mut display = String::new();
    while i < chars.len() && chars[i] != ']' {
        display.push(chars[i]);
        i += 1;
    }
    if i >= chars.len() {
        return None;
    }
    i += 1; // skip ']'
    if i >= chars.len() || chars[i] != '(' {
        return None;
    }
    i += 1; // skip '('
    let mut url = String::new();
    while i < chars.len() && chars[i] != ')' {
        url.push(chars[i]);
        i += 1;
    }
    if i >= chars.len() {
        return None;
    }
    i += 1; // skip ')'
    Some((display, url, i))
}

/// Extract text delimited by a single `delim` character (e.g. backtick).
fn extract_delimited(chars: &[char], start: usize, delim: char) -> Option<(String, usize)> {
    let mut i = start + 1;
    let mut content = String::new();
    while i < chars.len() {
        if chars[i] == delim {
            if !content.is_empty() {
                return Some((content, i + 1));
            }
            return None;
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

/// Extract text ending with the multi-char `end_marker` (e.g. `"**"`).
fn extract_between(chars: &[char], start: usize, end_marker: &str) -> Option<(String, usize)> {
    let marker: Vec<char> = end_marker.chars().collect();
    let mlen = marker.len();
    let mut i = start;
    let mut content = String::new();
    while i + mlen <= chars.len() {
        if chars[i..i + mlen] == marker[..] {
            if !content.is_empty() {
                return Some((content, i + mlen));
            }
            return None;
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

/// Flatten spans to a plain string (stripping formatting).
#[allow(dead_code)]
fn spans_to_plain(spans: &[MdSpan]) -> String {
    let mut out = String::new();
    for s in spans {
        match s {
            MdSpan::Plain(t)
            | MdSpan::Bold(t)
            | MdSpan::Italic(t)
            | MdSpan::BoldItalic(t)
            | MdSpan::Code(t) => out.push_str(t),
            MdSpan::Link(display, _) => out.push_str(display),
            MdSpan::Image(alt, _) => out.push_str(alt),
        }
    }
    out
}

// ───────────────────────── LaTeX helpers ─────────────────────────

/// Escape characters that are special in LaTeX.
fn latex_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str(r"\textbackslash{}"),
            '&' => out.push_str(r"\&"),
            '%' => out.push_str(r"\%"),
            '$' => out.push_str(r"\$"),
            '#' => out.push_str(r"\#"),
            '_' => out.push_str(r"\_"),
            '{' => out.push_str(r"\{"),
            '}' => out.push_str(r"\}"),
            '~' => out.push_str(r"\textasciitilde{}"),
            '^' => out.push_str(r"\textasciicircum{}"),
            // Unicode dashes
            '\u{2013}' => out.push_str("--"),          // en-dash –
            '\u{2014}' => out.push_str("---"),         // em-dash —
            // Unicode quotes
            '\u{2018}' => out.push_str("`"),            // left single quote ‘
            '\u{2019}' => out.push_str("'"),            // right single quote ’
            '\u{201C}' => out.push_str("``"),           // left double quote “
            '\u{201D}' => out.push_str("''"),           // right double quote ”
            // Currency & symbols
            '\u{20AC}' => out.push_str("\\texteuro{}"),  // euro sign €
            '\u{00A3}' => out.push_str("\\textsterling{}"), // pound sign
            '\u{00A9}' => out.push_str("\\textcopyright{}"), // copyright
            '\u{00AE}' => out.push_str("\\textregistered{}"), // registered
            '\u{2122}' => out.push_str("\\texttrademark{}"), // trademark
            '\u{00B0}' => out.push_str("\\textdegree{}"), // degree
            // Math operators
            '\u{00D7}' => out.push_str("$\\times$"),    // multiplication sign ×
            '\u{00F7}' => out.push_str("$\\div$"),      // division sign
            '\u{2264}' => out.push_str("$\\leq$"),      // less-than or equal
            '\u{2265}' => out.push_str("$\\geq$"),      // greater-than or equal
            '\u{2248}' => out.push_str("$\\approx$"),   // approximately
            '\u{2260}' => out.push_str("$\\neq$"),      // not equal
            // Arrows
            '\u{2192}' => out.push_str("$\\rightarrow$"), // right arrow
            '\u{2190}' => out.push_str("$\\leftarrow$"),  // left arrow
            // Misc
            '\u{2022}' => out.push_str("\\textbullet{}"), // bullet
            '\u{2026}' => out.push_str("\\ldots{}"),     // ellipsis
            '\u{00AB}' => out.push_str("\\guillemotleft{}"), // «
            '\u{00BB}' => out.push_str("\\guillemotright{}"), // »
            _ => out.push(ch),
        }
    }
    out
}

/// Convert a severity label to a LaTeX xcolor name.
fn severity_latex_color(sev: &str) -> &str {
    match sev.to_lowercase().as_str() {
        "critical" => "SevCritical",
        "high" => "SevHigh",
        "medium" => "SevMedium",
        "low" => "SevLow",
        "info" => "SevInfo",
        _ => "black",
    }
}

/// Render a slice of [`MdSpan`]s to LaTeX inline markup.
fn spans_to_latex(spans: &[MdSpan]) -> String {
    let mut out = String::new();
    for s in spans {
        match s {
            MdSpan::Plain(t) => out.push_str(&latex_escape(t)),
            MdSpan::Bold(t) => {
                out.push_str(r"\textbf{");
                out.push_str(&latex_escape(t));
                out.push('}');
            }
            MdSpan::Italic(t) => {
                out.push_str(r"\textit{");
                out.push_str(&latex_escape(t));
                out.push('}');
            }
            MdSpan::BoldItalic(t) => {
                out.push_str(r"\textbf{\textit{");
                out.push_str(&latex_escape(t));
                out.push_str("}}");
            }
            MdSpan::Code(t) => {
                out.push_str(r"\code{");
                out.push_str(&latex_escape(t));
                out.push('}');
            }
            MdSpan::Link(display, url) => {
                out.push_str(r"\href{");
                out.push_str(&latex_escape(url));
                out.push_str("}{");
                out.push_str(&latex_escape(display));
                out.push('}');
            }
            MdSpan::Image(alt, path) => {
                out.push_str("\n\n\\begin{center}\n");
                out.push_str(&format!("\\IfFileExists{{{}}}{{", path));
                out.push_str(&format!("\\includegraphics[width=0.9\\linewidth]{{{}}}\\\\[2mm]\n", path));
                if !alt.is_empty() {
                    out.push_str(&format!("{{\\small\\color{{CorpGray}}\\textit{{{}}}}}\n", latex_escape(alt)));
                }
                out.push_str("}{}");
                out.push_str("\\end{center}\n\n");
            }
        }
    }
    out
}

/// Render markdown text to LaTeX markup (block-level).
fn md_to_latex(text: &str) -> String {
    let md_blocks = parse_markdown(text);
    let mut out = String::new();

    let mut in_itemize = false;

    for mb in &md_blocks {
        match mb {
            MdBlock::Paragraph(spans) => {
                if in_itemize {
                    out.push_str("\\end{itemize}\n");
                    in_itemize = false;
                }
                out.push_str(&spans_to_latex(spans));
                out.push_str("\n\n");
            }
            MdBlock::Heading(level, spans) => {
                if in_itemize {
                    out.push_str("\\end{itemize}\n");
                    in_itemize = false;
                }
                let cmd = match level {
                    1 => "subsection*",
                    2 => "subsubsection*",
                    _ => "paragraph*",
                };
                out.push_str(&format!("\\{}{{{}}}\n\n", cmd, spans_to_latex(spans)));
            }
            MdBlock::BulletItem(spans) => {
                if !in_itemize {
                    out.push_str("\\begin{itemize}\n");
                    in_itemize = true;
                }
                out.push_str(&format!("  \\item {}\n", spans_to_latex(spans)));
            }
            MdBlock::CodeBlock(code) => {
                if in_itemize {
                    out.push_str("\\end{itemize}\n");
                    in_itemize = false;
                }
                out.push_str("\\begin{lstlisting}\n");
                out.push_str(code);
                out.push_str("\n\\end{lstlisting}\n\n");
            }
        }
    }

    if in_itemize {
        out.push_str("\\end{itemize}\n");
    }

    out
}

// ───────────────────────── blocks → LaTeX document ─────────────────────────

/// Convert the parsed blocks into a complete LaTeX document string.
fn blocks_to_latex(blocks: &[Block], asset: &str) -> String {
    let mut body = String::new();
    let mut after_section = false;

    for block in blocks {
        match block {
            Block::Title(t) => {
                body.push_str(&format!(
                    "\\thispagestyle{{empty}}\n\
                     \\vspace*{{40mm}}\n\
                     \\begin{{center}}\n\
                     {{\\color{{CorpDark}}\\rule{{0.6\\textwidth}}{{2pt}}}}\\\\[6mm]\n\
                     {{\\Huge\\bfseries\\color{{CorpDark}} {}}}\\\\[6mm]\n\
                     {{\\color{{CorpDark}}\\rule{{0.6\\textwidth}}{{2pt}}}}\n\
                     \\end{{center}}\n\
                     \\vspace{{10mm}}\n\n",
                    latex_escape(t),
                ));
            }
            Block::Subtitle(t) => {
                body.push_str(&format!(
                    "\\begin{{center}}\n\
                     {{\\Large\\color{{CorpGray}} {}}}\n\
                     \\end{{center}}\n\
                     \\vspace{{4mm}}\n\n",
                    latex_escape(t),
                ));
            }
            Block::Section(t) => {
                body.push_str(&format!(
                    "\\section{{{}}}\n\n",
                    latex_escape(t),
                ));
                after_section = true;
            }
            Block::Finding(sev, heading) => {
                let color = severity_latex_color(sev);
                if !after_section {
                    body.push_str("\\clearpage\n");
                }
                after_section = false;
                body.push_str(&format!(
                    "\\noindent\\colorbox{{{}!10}}{{\\parbox{{\\dimexpr\\textwidth-2\\fboxsep}}{{%\n\
                       \\large\\bfseries\\color{{CorpDark}} {}\n\
                       \\hfill {{\\normalsize\\colorbox{{{}}}{{\\color{{white}}\\textbf{{\\,{}\\,}}}}}}\n\
                     }}}}\n\
                     \\vspace{{0.5mm}}\n\
                     {{\\noindent\\color{{{}}}\\rule{{\\textwidth}}{{1.5pt}}}}\n\
                     \\nopagebreak\n\
                     \\vspace{{1mm}}\n\n",
                    color,
                    latex_escape(heading),
                    color,
                    latex_escape(sev),
                    color,
                ));
            }
            Block::Meta(key, val) => {
                after_section = false;
                body.push_str(&format!(
                    "\\noindent{{\\color{{CorpGray}}\\textbf{{{}:}}}} {}\\par\\vspace{{-0.3\\parskip}}\n",
                    latex_escape(key),
                    latex_escape(val),
                ));
            }
            Block::Table(rows) => {
                if rows.is_empty() {
                    continue;
                }
                let ncols = rows[0].len();
                // Use first column as fixed width, rest expand.
                let col_spec = if ncols <= 2 {
                    "l X".to_string()
                } else {
                    let mut s = String::from("l ");
                    for _ in 1..ncols {
                        s.push_str("X ");
                    }
                    s
                };
                // Increase row height for better readability
                body.push_str("{\\renewcommand{\\arraystretch}{1.35}\n");
                body.push_str(&format!(
                    "\\noindent\n\\begin{{tabularx}}{{\\textwidth}}{{{}}}\n\\toprule\n",
                    col_spec.trim(),
                ));
                // header row
                if let Some(header) = rows.first() {
                    let cells: Vec<String> =
                        header.iter().map(|c| format!("\\textbf{{\\color{{CorpDark}}{}}}", latex_escape(c))).collect();
                    body.push_str("\\rowcolor{CorpRule!30}\n");
                    body.push_str(&cells.join(" & "));
                    body.push_str(" \\\\\n\\midrule\n");
                }
                // data rows (with alternating background)
                for (idx, row) in rows.iter().skip(1).enumerate() {
                    if idx % 2 == 1 {
                        body.push_str("\\rowcolor{CodeBg}\n");
                    }
                    let cells: Vec<String> =
                        row.iter().map(|c| latex_escape(c)).collect();
                    body.push_str(&cells.join(" & "));
                    body.push_str(" \\\\\n");
                }
                body.push_str("\\bottomrule\n\\end{tabularx}\n}\n\\vspace{4mm}\n\n");
            }
            Block::Latex(raw) => {
                after_section = false;
                body.push_str(raw);
                body.push_str("\n\n");
            }
            Block::Text(t) => {
                after_section = false;
                body.push_str(&md_to_latex(t));
            }
            Block::Index => {
                body.push_str("\\tableofcontents\n\\vspace{6mm}\n\n");
            }
            Block::Spacer(mm) => {
                body.push_str(&format!("\\vspace{{{}mm}}\n\n", mm));
            }
            Block::PageBreak => {
                body.push_str("\\clearpage\n\n");
            }
            Block::HRule => {
                body.push_str("\\noindent{\\color{CorpRule}\\rule{\\textwidth}{0.4pt}}\n\\vspace{2mm}\n\n");
            }
        }
    }

    format!(
        "{PREAMBLE}\n\\begin{{document}}\n\n{body}\\end{{document}}\n",
        PREAMBLE = latex_preamble(asset),
        body = body,
    )
}

/// The LaTeX preamble: document class, packages, colour definitions, and
/// style settings that produce a professional-looking security report.
fn latex_preamble(asset: &str) -> String {
    let escaped_asset = latex_escape(asset);
    r#"\documentclass[11pt,a4paper]{article}

% ── geometry ──
\usepackage[top=25mm,bottom=30mm,left=25mm,right=25mm]{geometry}

% ── encoding & fonts ──
\usepackage[utf8]{inputenc}
\usepackage[T1]{fontenc}
\usepackage[scaled=0.92]{helvet}
\usepackage{courier}
\usepackage{microtype}
\renewcommand{\familydefault}{\sfdefault}

% ── packages ──
\usepackage{xcolor}
\usepackage{hyperref}
\usepackage{booktabs}
\usepackage{tabularx}
\usepackage{listings}
\usepackage{parskip}
\usepackage{fancyhdr}
\usepackage{graphicx}
\usepackage{etoolbox}
\usepackage{colortbl}
\usepackage{textcomp}

% ── corporate colours ──
\definecolor{CorpDark}{HTML}{1E293B}
\definecolor{CorpAccent}{HTML}{334155}
\definecolor{CorpRule}{HTML}{CBD5E1}
\definecolor{CorpGray}{HTML}{64748B}
\definecolor{CodeBg}{HTML}{F1F5F9}

% ── severity colours ──
\definecolor{SevCritical}{HTML}{991B1B}
\definecolor{SevHigh}{HTML}{C2410C}
\definecolor{SevMedium}{HTML}{B45309}
\definecolor{SevLow}{HTML}{15803D}
\definecolor{SevInfo}{HTML}{1D4ED8}

% ── hyperlinks ──
\hypersetup{
  colorlinks=true,
  linkcolor=CorpDark,
  urlcolor=SevInfo,
  bookmarks=true,
  bookmarksnumbered=true,
}

% ── listings (code blocks) ──
\lstset{
  basicstyle=\small\ttfamily,
  backgroundcolor=\color{CodeBg},
  frame=single,
  rulecolor=\color{CorpRule},
  framerule=0.4pt,
  breaklines=true,
  breakatwhitespace=false,
  postbreak=\mbox{\textcolor{CorpGray}{$\hookrightarrow$}\space},
  xleftmargin=6mm,
  xrightmargin=6mm,
  aboveskip=8pt,
  belowskip=8pt,
}

% ── section styling ──
\makeatletter
\renewcommand{\section}{%
  \@startsection{section}{1}{0pt}{-2ex plus -1ex minus -0.2ex}{1.2ex plus 0.2ex}{%
    \large\bfseries\color{CorpDark}}}
\makeatother

% ── TOC styling ──
\setcounter{tocdepth}{1}
\setcounter{secnumdepth}{2}
\makeatletter
\renewcommand{\l@section}[2]{%
  \addpenalty{-\@highpenalty}%
  \vskip 8pt plus 2pt
  \setlength\@tempdima{2em}%
  \begingroup
    \parindent\z@ \rightskip\@tocrmarg
    \parfillskip -\rightskip
    \leavevmode\large\bfseries\color{CorpDark}
    #1\nobreak
    \leaders\hbox{$\m@th\mkern 4mu\cdot\mkern 4mu$}\hfill
    \nobreak\hb@xt@\@pnumwidth{\hss #2}%
    \par
  \endgroup
  \penalty\@highpenalty}
\renewcommand{\l@subsection}[2]{%
  \vskip 2pt
  \setlength\@tempdima{3em}%
  \begingroup
    \parindent 1.5em \rightskip\@tocrmarg
    \parfillskip -\rightskip
    \leavevmode\normalsize\color{CorpAccent}
    #1\nobreak
    \leaders\hbox{$\m@th\mkern 4mu\cdot\mkern 4mu$}\hfill
    \nobreak\hb@xt@\@pnumwidth{\hss #2}%
    \par
  \endgroup}
\makeatother

% ── breakable inline code ──
\makeatletter
\newcommand{\code}[1]{{%
  \ttfamily\hyphenpenalty=10000\exhyphenpenalty=10000
  \@code@loop#1\@nil
}}
\def\@code@loop{\@ifnextchar\@nil{\@gobble}{\@code@char}}
\def\@code@char#1{#1\discretionary{}{}{}\@code@loop}
\makeatother

% ── headers / footers ──
\pagestyle{fancy}
\fancyhf{}
\renewcommand{\headrulewidth}{0.4pt}
\renewcommand{\headrule}{\hbox to\headwidth{\color{CorpRule}\leaders\hrule height \headrulewidth\hfill}}
\fancyhead[L]{\small\color{CorpGray}\textit{Security Assessment Report -- %%ASSET%%}}
\fancyhead[R]{\small\color{CorpGray}\thepage}
\fancyfoot[C]{}
\renewcommand{\footrulewidth}{0pt}
"#
    .replace("%%ASSET%%", &escaped_asset)
}

// ───────────────────────── PDF compilation ─────────────────────────

/// Compile the LaTeX source to PDF using the embedded tectonic engine
/// and write the result to `output_path`.  No external TeX installation
/// is required.
fn render_pdf(latex_src: &str, output_path: &str, work_dir: &Path) -> Result<()> {
    use tectonic::config::PersistentConfig;
    use tectonic::driver::{OutputFormat, ProcessingSessionBuilder};
    use tectonic::status::NoopStatusBackend;

    let mut status = NoopStatusBackend::default();

    let config = PersistentConfig::open(false).map_err(|e| {
        StorageError::PdfError(format!("tectonic configuration error: {e}"))
    })?;

    let bundle = config.default_bundle(false, &mut status).map_err(|e| {
        StorageError::PdfError(format!("tectonic bundle error: {e}"))
    })?;

    let format_cache_path = config.format_cache_path().map_err(|e| {
        StorageError::PdfError(format!("tectonic format cache error: {e}"))
    })?;

    let mut sb = ProcessingSessionBuilder::default();
    sb.bundle(bundle)
        .primary_input_buffer(latex_src.as_bytes())
        .tex_input_name("texput.tex")
        .format_name("latex")
        .format_cache_path(format_cache_path)
        .keep_logs(false)
        .keep_intermediates(false)
        .print_stdout(false)
        .output_format(OutputFormat::Pdf)
        .filesystem_root(work_dir)
        .do_not_write_output_files();

    let mut sess = sb.create(&mut status).map_err(|e| {
        StorageError::PdfError(format!("tectonic LaTeX compilation failed: {e}"))
    })?;

    sess.run(&mut status).map_err(|e| {
        StorageError::PdfError(format!("tectonic LaTeX compilation failed: {e}"))
    })?;

    let mut files = sess.into_file_data();
    let pdf_data = files
        .remove("texput.pdf")
        .ok_or_else(|| StorageError::PdfError("tectonic: no PDF output produced".into()))?
        .data;

    // Ensure output directory exists
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, &pdf_data)?;

    Ok(())
}

// ───────────────────────── date helper ─────────────────────────

/// Current date as `YYYY/MM/DD`.
fn current_date() -> String {
    // Extracted from `date +%Y/%m/%d` logic, no chrono dependency.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let days = secs / 86400;
    let mut y = 1970i32;
    let mut rem = days;

    loop {
        let ylen: i64 = if is_leap(y) { 366 } else { 365 };
        if rem < ylen {
            break;
        }
        rem -= ylen;
        y += 1;
    }

    let mut m = 1u32;
    loop {
        let mlen = month_days(y, m) as i64;
        if rem < mlen {
            break;
        }
        rem -= mlen;
        m += 1;
    }
    let d = rem as u32 + 1;
    format!("{y:04}/{m:02}/{d:02}")
}

fn is_leap(y: i32) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

fn month_days(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap(y) { 29 } else { 28 },
        _ => 30,
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── LaTeX escape ──

    #[test]
    fn latex_escape_basic() {
        assert_eq!(latex_escape("hello"), "hello");
    }

    #[test]
    fn latex_escape_special_chars() {
        assert_eq!(latex_escape("a & b"), r"a \& b");
        assert_eq!(latex_escape("100%"), r"100\%");
        assert_eq!(latex_escape("$x$"), r"\$x\$");
        assert_eq!(latex_escape("item #1"), r"item \#1");
        assert_eq!(latex_escape("a_b"), r"a\_b");
        assert_eq!(latex_escape("{x}"), r"\{x\}");
    }

    #[test]
    fn latex_escape_tilde_caret_backslash() {
        assert_eq!(latex_escape("~"), r"\textasciitilde{}");
        assert_eq!(latex_escape("^"), r"\textasciicircum{}");
        assert_eq!(latex_escape(r"\"), r"\textbackslash{}");
    }

    #[test]
    fn latex_escape_unicode_chars() {
        // Dashes
        assert_eq!(latex_escape("\u{2013}"), "--");  // en-dash
        assert_eq!(latex_escape("\u{2014}"), "---"); // em-dash
        // Euro and multiplication (the two characters causing rendering bugs)
        assert_eq!(latex_escape("29.99\u{20AC}"), r"29.99\texteuro{}");
        assert_eq!(latex_escape("1.5\u{00D7}"), r"1.5$\times$");
        // Quotes
        assert_eq!(latex_escape("\u{201C}hello\u{201D}"), "``hello''");
    }

    // ── severity colour ──

    #[test]
    fn severity_latex_color_known() {
        assert_eq!(severity_latex_color("Critical"), "SevCritical");
        assert_eq!(severity_latex_color("high"), "SevHigh");
        assert_eq!(severity_latex_color("MEDIUM"), "SevMedium");
        assert_eq!(severity_latex_color("Low"), "SevLow");
        assert_eq!(severity_latex_color("Info"), "SevInfo");
    }

    #[test]
    fn severity_latex_color_unknown() {
        assert_eq!(severity_latex_color("banana"), "black");
    }

    // ── parse_blocks ──

    #[test]
    fn parse_blocks_title() {
        let blocks = parse_blocks("#! title My Report");
        assert_eq!(blocks, vec![Block::Title("My Report".into())]);
    }

    #[test]
    fn parse_blocks_subtitle() {
        let blocks = parse_blocks("#! subtitle target.corp");
        assert_eq!(blocks, vec![Block::Subtitle("target.corp".into())]);
    }

    #[test]
    fn parse_blocks_section() {
        let blocks = parse_blocks("#! section Executive Summary");
        assert_eq!(blocks, vec![Block::Section("Executive Summary".into())]);
    }

    #[test]
    fn parse_blocks_finding() {
        let blocks = parse_blocks("#! finding Critical 1. SQL Injection");
        assert_eq!(
            blocks,
            vec![Block::Finding("Critical".into(), "1. SQL Injection".into())]
        );
    }

    #[test]
    fn parse_blocks_meta() {
        let blocks = parse_blocks("#! meta Prepared for: ACME Corp");
        assert_eq!(
            blocks,
            vec![Block::Meta("Prepared for".into(), "ACME Corp".into())]
        );
    }

    #[test]
    fn parse_blocks_index() {
        let blocks = parse_blocks("#! index");
        assert_eq!(blocks, vec![Block::Index]);
    }

    #[test]
    fn parse_blocks_spacer() {
        let blocks = parse_blocks("#! spacer 8");
        assert_eq!(blocks, vec![Block::Spacer(8.0)]);
    }

    #[test]
    fn parse_blocks_pagebreak() {
        let blocks = parse_blocks("#! pagebreak");
        assert_eq!(blocks, vec![Block::PageBreak]);
    }

    #[test]
    fn parse_blocks_hr() {
        let blocks = parse_blocks("#! hr");
        assert_eq!(blocks, vec![Block::HRule]);
    }

    #[test]
    fn parse_blocks_comment_ignored() {
        let blocks = parse_blocks("#! comment This should not appear");
        assert!(blocks.is_empty());
    }

    #[test]
    fn parse_blocks_latex_inline() {
        let blocks = parse_blocks("#! latex \\vspace{20mm}");
        assert_eq!(blocks, vec![Block::Latex("\\vspace{20mm}".into())]);
    }

    #[test]
    fn parse_blocks_latex_block() {
        let input = "#! latex\n\\begin{center}\n\\includegraphics{logo.png}\n\\end{center}\n#! endlatex";
        let blocks = parse_blocks(input);
        assert_eq!(blocks, vec![Block::Latex("\\begin{center}\n\\includegraphics{logo.png}\n\\end{center}".into())]);
    }

    #[test]
    fn parse_blocks_plain_text() {
        let blocks = parse_blocks("Hello world.");
        assert_eq!(blocks, vec![Block::Text("Hello world.".into())]);
    }

    #[test]
    fn parse_blocks_table() {
        let input = "Sev | Count\nCritical | 3\nHigh | 5";
        let blocks = parse_blocks(input);
        assert_eq!(
            blocks,
            vec![Block::Table(vec![
                vec!["Sev".into(), "Count".into()],
                vec!["Critical".into(), "3".into()],
                vec!["High".into(), "5".into()],
            ])]
        );
    }

    #[test]
    fn parse_blocks_mixed_sequence() {
        let input = "\
#! title Report
#! spacer 4
Some text here.
#! pagebreak
#! section Details
";
        let blocks = parse_blocks(input);
        assert_eq!(blocks.len(), 5);
        assert_eq!(blocks[0], Block::Title("Report".into()));
        assert_eq!(blocks[1], Block::Spacer(4.0));
        assert_eq!(blocks[2], Block::Text("Some text here.".into()));
        assert_eq!(blocks[3], Block::PageBreak);
        assert_eq!(blocks[4], Block::Section("Details".into()));
    }

    #[test]
    fn parse_blocks_table_then_text() {
        let input = "A | B\n1 | 2\nSome paragraph after table.";
        let blocks = parse_blocks(input);
        assert_eq!(blocks.len(), 2);
        assert!(matches!(&blocks[0], Block::Table(_)));
        assert_eq!(blocks[1], Block::Text("Some paragraph after table.".into()));
    }

    #[test]
    fn parse_blocks_table_pipe_delimited_markdown() {
        // Markdown-style pipe tables with leading/trailing pipes and separator.
        let input = "| A | B | C |\n|---|---|---|\n| 1 | 2 | 3 |\n| x | y | z |";
        let blocks = parse_blocks(input);
        assert_eq!(
            blocks,
            vec![Block::Table(vec![
                vec!["A".into(), "B".into(), "C".into()],
                vec!["1".into(), "2".into(), "3".into()],
                vec!["x".into(), "y".into(), "z".into()],
            ])]
        );
    }

    #[test]
    fn parse_blocks_table_separator_only_skipped() {
        // Separator with colons (alignment markers) should also be skipped.
        let input = "| A | B |\n|:---|---:|\n| 1 | 2 |";
        let blocks = parse_blocks(input);
        assert_eq!(
            blocks,
            vec![Block::Table(vec![
                vec!["A".into(), "B".into()],
                vec!["1".into(), "2".into()],
            ])]
        );
    }

    // ── parse_inline_spans ──

    #[test]
    fn spans_plain() {
        let spans = parse_inline_spans("hello world");
        assert_eq!(spans, vec![MdSpan::Plain("hello world".into())]);
    }

    #[test]
    fn spans_bold() {
        let spans = parse_inline_spans("a **bold** b");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1], MdSpan::Bold("bold".into()));
    }

    #[test]
    fn spans_italic() {
        let spans = parse_inline_spans("a *italic* b");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1], MdSpan::Italic("italic".into()));
    }

    #[test]
    fn spans_bold_italic() {
        let spans = parse_inline_spans("***both***");
        assert_eq!(spans, vec![MdSpan::BoldItalic("both".into())]);
    }

    #[test]
    fn spans_code() {
        let spans = parse_inline_spans("use `foo()` here");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1], MdSpan::Code("foo()".into()));
    }

    #[test]
    fn spans_link() {
        let spans = parse_inline_spans("see [docs](https://example.com)");
        assert_eq!(spans.len(), 2);
        assert_eq!(
            spans[1],
            MdSpan::Link("docs".into(), "https://example.com".into())
        );
    }

    #[test]
    fn spans_image() {
        let spans = parse_inline_spans("![screenshot](proof.png)");
        assert_eq!(spans, vec![MdSpan::Image("screenshot".into(), "proof.png".into())]);
    }

    #[test]
    fn spans_image_with_text() {
        let spans = parse_inline_spans("see ![proof](img.jpg) here");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0], MdSpan::Plain("see ".into()));
        assert_eq!(spans[1], MdSpan::Image("proof".into(), "img.jpg".into()));
        assert_eq!(spans[2], MdSpan::Plain(" here".into()));
    }

    #[test]
    fn spans_image_not_confused_with_link() {
        // Ensure ![...] is parsed as image, not "!" + link
        let spans = parse_inline_spans("![alt](path.png)");
        assert_eq!(spans.len(), 1);
        assert!(matches!(&spans[0], MdSpan::Image(_, _)));
    }

    #[test]
    fn spans_mixed() {
        let spans = parse_inline_spans("**bold** and *italic* and `code`");
        assert!(spans.len() >= 5);
        assert_eq!(spans[0], MdSpan::Bold("bold".into()));
        assert_eq!(spans[2], MdSpan::Italic("italic".into()));
        assert_eq!(spans[4], MdSpan::Code("code".into()));
    }

    // ── parse_markdown ──

    #[test]
    fn md_paragraph() {
        let blocks = parse_markdown("Hello world.");
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], MdBlock::Paragraph(_)));
    }

    #[test]
    fn md_heading() {
        let blocks = parse_markdown("# Title\n## Sub\n### Sub-sub");
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], MdBlock::Heading(1, _)));
        assert!(matches!(&blocks[1], MdBlock::Heading(2, _)));
        assert!(matches!(&blocks[2], MdBlock::Heading(3, _)));
    }

    #[test]
    fn md_bullet_list() {
        let blocks = parse_markdown("- one\n- two\n- three");
        assert_eq!(blocks.len(), 3);
        for b in &blocks {
            assert!(matches!(b, MdBlock::BulletItem(_)));
        }
    }

    #[test]
    fn md_code_block() {
        let input = "```\nfn main() {}\n```";
        let blocks = parse_markdown(input);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], MdBlock::CodeBlock(_)));
        if let MdBlock::CodeBlock(code) = &blocks[0] {
            assert_eq!(code, "fn main() {}");
        }
    }

    #[test]
    fn md_mixed() {
        let input = "Paragraph.\n\n# Heading\n\n- bullet\n\n```\ncode\n```";
        let blocks = parse_markdown(input);
        assert_eq!(blocks.len(), 4);
        assert!(matches!(&blocks[0], MdBlock::Paragraph(_)));
        assert!(matches!(&blocks[1], MdBlock::Heading(1, _)));
        assert!(matches!(&blocks[2], MdBlock::BulletItem(_)));
        assert!(matches!(&blocks[3], MdBlock::CodeBlock(_)));
    }

    // ── spans_to_plain ──

    #[test]
    fn spans_to_plain_basic() {
        let spans = parse_inline_spans("**bold** and *italic*");
        let plain = spans_to_plain(&spans);
        assert_eq!(plain, "bold and italic");
    }

    #[test]
    fn spans_to_plain_link() {
        let spans = vec![MdSpan::Link("click".into(), "https://x.com".into())];
        assert_eq!(spans_to_plain(&spans), "click");
    }

    // ── spans_to_latex ──

    #[test]
    fn spans_to_latex_plain() {
        let spans = vec![MdSpan::Plain("hello".into())];
        assert_eq!(spans_to_latex(&spans), "hello");
    }

    #[test]
    fn spans_to_latex_bold() {
        let spans = vec![MdSpan::Bold("strong".into())];
        assert_eq!(spans_to_latex(&spans), r"\textbf{strong}");
    }

    #[test]
    fn spans_to_latex_italic() {
        let spans = vec![MdSpan::Italic("em".into())];
        assert_eq!(spans_to_latex(&spans), r"\textit{em}");
    }

    #[test]
    fn spans_to_latex_bold_italic() {
        let spans = vec![MdSpan::BoldItalic("bi".into())];
        assert_eq!(spans_to_latex(&spans), r"\textbf{\textit{bi}}");
    }

    #[test]
    fn spans_to_latex_code() {
        let spans = vec![MdSpan::Code("x()".into())];
        assert_eq!(spans_to_latex(&spans), r"\code{x()}");
    }

    #[test]
    fn spans_to_latex_link() {
        let spans = vec![MdSpan::Link("site".into(), "https://x.com".into())];
        assert_eq!(
            spans_to_latex(&spans),
            r"\href{https://x.com}{site}"
        );
    }

    #[test]
    fn spans_to_latex_image() {
        let spans = vec![MdSpan::Image("proof".into(), "proof.png".into())];
        let latex = spans_to_latex(&spans);
        assert!(latex.contains(r"\includegraphics"));
        assert!(latex.contains("proof.png"));
        assert!(latex.contains("proof")); // alt text
    }

    #[test]
    fn spans_to_latex_escapes_special() {
        let spans = vec![MdSpan::Plain("a & b".into())];
        assert_eq!(spans_to_latex(&spans), r"a \& b");
    }

    // ── md_to_latex ──

    #[test]
    fn md_to_latex_paragraph() {
        let result = md_to_latex("Hello world.");
        assert!(result.contains("Hello world."));
    }

    #[test]
    fn md_to_latex_heading() {
        let result = md_to_latex("# Title");
        assert!(result.contains(r"\subsection*{Title}"));
    }

    #[test]
    fn md_to_latex_heading_levels() {
        let result = md_to_latex("## Sub\n### SubSub");
        assert!(result.contains(r"\subsubsection*{Sub}"));
        assert!(result.contains(r"\paragraph*{SubSub}"));
    }

    #[test]
    fn md_to_latex_bullets() {
        let result = md_to_latex("- one\n- two");
        assert!(result.contains(r"\begin{itemize}"));
        assert!(result.contains(r"\item one"));
        assert!(result.contains(r"\item two"));
        assert!(result.contains(r"\end{itemize}"));
    }

    #[test]
    fn md_to_latex_code_block() {
        let result = md_to_latex("```\ncode here\n```");
        assert!(result.contains(r"\begin{lstlisting}"));
        assert!(result.contains("code here"));
        assert!(result.contains(r"\end{lstlisting}"));
    }

    #[test]
    fn md_to_latex_inline_formatting() {
        let result = md_to_latex("Use **bold** and *italic* and `code` together.");
        assert!(result.contains(r"\textbf{bold}"));
        assert!(result.contains(r"\textit{italic}"));
        assert!(result.contains(r"\code{code}"));
    }

    #[test]
    fn md_to_latex_image() {
        let result = md_to_latex("See below:\n\n![proof screenshot](proof.png)");
        assert!(result.contains(r"\includegraphics"));
        assert!(result.contains("proof.png"));
    }

    // ── rewrite_description_images ──

    #[test]
    fn rewrite_images_no_images() {
        let desc = "No images here.";
        assert_eq!(rewrite_description_images(desc, &[], "slug"), "No images here.");
    }

    #[test]
    fn rewrite_images_matching_basename() {
        let desc = "See ![proof](../img/xss.jpg) for details.";
        let images = vec!["img/xss.jpg".to_string()];
        let result = rewrite_description_images(desc, &images, "stored-xss");
        assert_eq!(result, "See ![proof](stored-xss-xss.jpg) for details.");
    }

    #[test]
    fn rewrite_images_no_match() {
        let desc = "See ![proof](../img/other.jpg) for details.";
        let images = vec!["img/xss.jpg".to_string()];
        let result = rewrite_description_images(desc, &images, "stored-xss");
        // No match: original path is preserved
        assert_eq!(result, "See ![proof](../img/other.jpg) for details.");
    }

    #[test]
    fn rewrite_images_multiple() {
        let desc = "![a](img/one.png) and ![b](img/two.png)";
        let images = vec!["img/one.png".to_string(), "img/two.png".to_string()];
        let result = rewrite_description_images(desc, &images, "vuln");
        assert!(result.contains("vuln-one.png"));
        assert!(result.contains("vuln-two.png"));
    }

    // ── blocks_to_latex ──

    #[test]
    fn btl_title() {
        let latex = blocks_to_latex(&[Block::Title("My Report".into())], "test");
        assert!(latex.contains("My Report"));
    }

    #[test]
    fn btl_subtitle() {
        let latex = blocks_to_latex(&[Block::Subtitle("acme.corp".into())], "test");
        assert!(latex.contains("acme.corp"));
    }

    #[test]
    fn btl_section() {
        let latex = blocks_to_latex(&[Block::Section("Details".into())], "test");
        assert!(latex.contains(r"\section{Details}"));
    }

    #[test]
    fn btl_finding() {
        let latex = blocks_to_latex(&[Block::Finding("Critical".into(), "SQLi".into())], "test");
        assert!(latex.contains("SQLi"));
        assert!(latex.contains("Critical"));
    }

    #[test]
    fn btl_meta() {
        let latex = blocks_to_latex(&[Block::Meta("Asset".into(), "web.corp".into())], "test");
        assert!(latex.contains("Asset"));
        assert!(latex.contains("web.corp"));
    }

    #[test]
    fn btl_table() {
        let rows = vec![
            vec!["A".into(), "B".into()],
            vec!["1".into(), "2".into()],
        ];
        let latex = blocks_to_latex(&[Block::Table(rows)], "test");
        assert!(latex.contains(r"\begin{tabularx}"));
        assert!(latex.contains("1 & 2"));
    }

    #[test]
    fn btl_table_three_cols() {
        let rows = vec![
            vec!["A".into(), "B".into(), "C".into()],
            vec!["1".into(), "2".into(), "3".into()],
        ];
        let latex = blocks_to_latex(&[Block::Table(rows)], "test");
        assert!(latex.contains(r"\begin{tabularx}"));
        assert!(latex.contains("1 & 2 & 3"));
    }

    #[test]
    fn btl_text_markdown() {
        let latex = blocks_to_latex(&[Block::Text("**bold** text".into())], "test");
        assert!(latex.contains(r"\textbf{bold}"));
    }

    #[test]
    fn btl_latex() {
        let latex = blocks_to_latex(&[Block::Latex("\\begin{center}\n\\includegraphics{proof.png}\n\\end{center}".into())], "test");
        assert!(latex.contains(r"\includegraphics{proof.png}"));
        assert!(latex.contains(r"\begin{center}"));
    }

    #[test]
    fn btl_index() {
        let latex = blocks_to_latex(&[Block::Index], "test");
        assert!(latex.contains(r"\tableofcontents"));
    }

    #[test]
    fn btl_spacer() {
        let latex = blocks_to_latex(&[Block::Spacer(10.0)], "test");
        assert!(latex.contains(r"\vspace{10mm}"));
    }

    #[test]
    fn btl_pagebreak() {
        let latex = blocks_to_latex(&[Block::PageBreak], "test");
        assert!(latex.contains(r"\clearpage"));
    }

    #[test]
    fn btl_hrule() {
        let latex = blocks_to_latex(&[Block::HRule], "test");
        assert!(latex.contains(r"\rule"));
    }

    #[test]
    fn btl_full_document_structure() {
        let latex = blocks_to_latex(&[Block::Title("T".into())], "test");
        assert!(latex.contains(r"\documentclass"));
        assert!(latex.contains(r"\begin{document}"));
        assert!(latex.contains(r"\end{document}"));
    }

    // ── date helpers ──

    #[test]
    fn leap_year_detection() {
        assert!(is_leap(2000));
        assert!(is_leap(2024));
        assert!(!is_leap(1900));
        assert!(!is_leap(2023));
    }

    #[test]
    fn month_days_normal() {
        assert_eq!(month_days(2023, 1), 31);
        assert_eq!(month_days(2023, 2), 28);
        assert_eq!(month_days(2023, 4), 30);
    }

    #[test]
    fn month_days_leap_feb() {
        assert_eq!(month_days(2024, 2), 29);
    }

    #[test]
    fn current_date_format() {
        let d = current_date();
        assert_eq!(d.len(), 10);
        assert_eq!(&d[4..5], "/");
        assert_eq!(&d[7..8], "/");
    }

    // ── helper functions ──

    #[test]
    fn try_parse_link_valid() {
        let chars: Vec<char> = "[docs](https://x.com) rest".chars().collect();
        let result = try_parse_link(&chars, 0);
        assert!(result.is_some());
        let (display, url, end) = result.unwrap();
        assert_eq!(display, "docs");
        assert_eq!(url, "https://x.com");
        assert_eq!(end, 21);
    }

    #[test]
    fn try_parse_link_invalid_no_paren() {
        let chars: Vec<char> = "[docs] rest".chars().collect();
        let result = try_parse_link(&chars, 0);
        assert!(result.is_none());
    }

    #[test]
    fn extract_delimited_backtick() {
        let chars: Vec<char> = "`code` rest".chars().collect();
        let result = extract_delimited(&chars, 0, '`');
        assert!(result.is_some());
        let (content, end) = result.unwrap();
        assert_eq!(content, "code");
        assert_eq!(end, 6);
    }

    #[test]
    fn extract_delimited_empty_returns_none() {
        let chars: Vec<char> = "`` rest".chars().collect();
        let result = extract_delimited(&chars, 0, '`');
        assert!(result.is_none());
    }

    #[test]
    fn extract_between_double_star() {
        let chars: Vec<char> = "bold** rest".chars().collect();
        let result = extract_between(&chars, 0, "**");
        assert!(result.is_some());
        let (content, end) = result.unwrap();
        assert_eq!(content, "bold");
        assert_eq!(end, 6);
    }

    #[test]
    fn extract_between_no_match() {
        let chars: Vec<char> = "no end marker".chars().collect();
        let result = extract_between(&chars, 0, "**");
        assert!(result.is_none());
    }

    // ── integration: blocks_to_latex with mixed content ──

    #[test]
    fn integration_mixed_blocks() {
        let blocks = vec![
            Block::Title("Security Report".into()),
            Block::Subtitle("acme.corp".into()),
            Block::PageBreak,
            Block::Section("Executive Summary".into()),
            Block::Text("This is a **test** report.".into()),
            Block::Spacer(4.0),
            Block::Table(vec![
                vec!["Sev".into(), "Count".into()],
                vec!["Critical".into(), "2".into()],
            ]),
            Block::PageBreak,
            Block::Section("Findings".into()),
            Block::Finding("Critical".into(), "1. SQL Injection".into()),
            Block::Meta("Asset".into(), "web.corp".into()),
            Block::Text("Description with `code` and **bold**.".into()),
            Block::HRule,
        ];
        let latex = blocks_to_latex(&blocks, "test");

        // Verify document structure
        assert!(latex.contains(r"\documentclass"));
        assert!(latex.contains(r"\begin{document}"));
        assert!(latex.contains(r"\end{document}"));

        // Verify blocks rendered
        assert!(latex.contains("Security Report"));
        assert!(latex.contains("acme.corp"));
        assert!(latex.contains(r"\clearpage"));
        assert!(latex.contains(r"\section{Executive Summary}"));
        assert!(latex.contains(r"\textbf{test}"));
        assert!(latex.contains(r"\begin{tabularx}"));
        assert!(latex.contains(r"\section{Findings}"));
        assert!(latex.contains("SQL Injection"));
        assert!(latex.contains(r"\code{code}"));
    }

    #[test]
    fn integration_full_parse_and_render() {
        let input = "\
#! title Test Report
#! subtitle target.local
#! spacer 8
#! meta Date: 2025/01/01
#! pagebreak
#! section Summary
This report has **bold** and *italic* content.
#! spacer 4
#! table
Severity | Count
Critical | 1
#! pagebreak
#! section Findings
#! finding High 1. XSS Attack
#! meta Severity: High
#! meta Asset: web.app
Reflected XSS in the `search` parameter.
- Step 1: inject payload
- Step 2: observe alert
#! hr
";
        let blocks = parse_blocks(input);
        let latex = blocks_to_latex(&blocks, "test");

        assert!(latex.contains(r"\documentclass"));
        assert!(latex.contains("Test Report"));
        assert!(latex.contains("target.local"));
        assert!(latex.contains(r"\section{Summary}"));
        assert!(latex.contains(r"\textbf{bold}"));
        assert!(latex.contains(r"\textit{italic}"));
        assert!(latex.contains(r"\begin{tabularx}"));
        assert!(latex.contains("SevHigh"));
        assert!(latex.contains("XSS Attack"));
        assert!(latex.contains(r"\code{search}"));
        assert!(latex.contains(r"\begin{itemize}"));
        assert!(latex.contains(r"\item"));
        assert!(latex.contains(r"\end{itemize}"));
        // Finding starts on its own page
        assert!(latex.contains(r"\clearpage"));
    }

    // ── finding page break ──

    #[test]
    fn finding_starts_on_new_page() {
        let blocks = vec![
            Block::Text("Some text.".into()),
            Block::Finding("High".into(), "1. Test".into()),
        ];
        let latex = blocks_to_latex(&blocks, "test");
        let finding_pos = latex.find("1. Test").unwrap();
        let clearpage_before = latex[..finding_pos].rfind(r"\clearpage");
        assert!(clearpage_before.is_some());
    }

    #[test]
    fn first_finding_after_section_no_clearpage() {
        let blocks = vec![
            Block::Section("Detailed Findings".into()),
            Block::Finding("High".into(), "1. Test".into()),
        ];
        let latex = blocks_to_latex(&blocks, "test");
        let section_pos = latex.find(r"\section{Detailed Findings}").unwrap();
        let finding_pos = latex.find("1. Test").unwrap();
        let between = &latex[section_pos..finding_pos];
        assert!(!between.contains(r"\clearpage"));
    }

    #[test]
    fn multiple_findings_each_on_own_page() {
        let blocks = vec![
            Block::Finding("Critical".into(), "1. First".into()),
            Block::Text("Description.".into()),
            Block::Finding("High".into(), "2. Second".into()),
        ];
        let latex = blocks_to_latex(&blocks, "test");
        let clearpage_count = latex.matches(r"\clearpage").count();
        assert!(clearpage_count >= 2);
    }

    // ── render_pdf error on invalid LaTeX ──

    #[test]
    fn render_pdf_with_empty_latex_handles_error() {
        // Empty input is not valid LaTeX — tectonic should return an error.
        let tmp = tempfile::tempdir().unwrap();
        let result = render_pdf("", "/tmp/pog_test_nonexistent.pdf", tmp.path());
        assert!(result.is_err());
    }
}
