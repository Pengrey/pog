// storage/src/report.rs — Typst-based PDF report generation.
//
// The public entry point is [`generate_report`].  It reads a Typst
// template (`.typ`), injects finding data as `sys.inputs`, compiles it
// via `typst-as-lib`, and exports a PDF with `typst-pdf`.

use crate::error::{Result, StorageError};
use crate::pogdir::PogDir;
use models::{Finding, Severity};
use std::fs;
use std::path::Path;
use typst::foundations::{Dict, IntoValue, Str, Value};
use typst::layout::PagedDocument;
use typst_as_lib::TypstEngine;

// ───────────────────────── public API ─────────────────────────

/// Generate a PDF report from a `.typ` Typst template.
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
    let template_src = fs::read_to_string(template_path)?;

    // ── set up work directory for images ──
    let work_dir = tempfile::tempdir()?;
    prepare_finding_images(findings, pog, work_dir.path())?;

    let template_dir = Path::new(template_path)
        .parent()
        .unwrap_or(Path::new("."));

    copy_template_assets(template_dir, work_dir.path())?;

    // ── build Typst input dictionary ──
    let inputs = build_inputs(findings, asset, from, to);

    // ── compile Typst → PDF ──
    let engine = TypstEngine::builder()
        .main_file(template_src.as_str())
        .search_fonts_with(
            typst_as_lib::typst_kit_options::TypstKitFontOptions::default()
                .include_system_fonts(true)
                .include_embedded_fonts(true),
        )
        .with_file_system_resolver(work_dir.path())
        .build();

    let doc: PagedDocument = engine
        .compile_with_input(inputs)
        .output
        .map_err(|e| StorageError::PdfError(format!("Typst compilation error: {e}")))?;

    let options = typst_pdf::PdfOptions::default();
    let pdf_data = typst_pdf::pdf(&doc, &options)
        .map_err(|e| StorageError::PdfError(format!("PDF export error: {e:?}")))?;

    // ── write output ──
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, pdf_data)?;

    Ok(())
}

// ───────────────────────── input builder ─────────────────────────

/// Build a Typst [`Dict`] containing all template data.
fn build_inputs(findings: &[Finding], asset: &str, from: &str, to: &str) -> Dict {
    let finding_values: Vec<Value> = findings
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let mut d = Dict::new();
            d.insert(Str::from("num"), (i as i64 + 1).into_value());
            d.insert(Str::from("title"), f.title.as_str().into_value());
            d.insert(
                Str::from("severity"),
                f.severity.as_str().into_value(),
            );
            d.insert(Str::from("asset"), f.asset.as_str().into_value());
            d.insert(Str::from("date"), f.date.as_str().into_value());
            d.insert(
                Str::from("location"),
                f.location.as_str().into_value(),
            );
            let desc = rewrite_report_content_images(&f.report_content, &f.images, &f.slug);
            let desc_typ = md_to_typst(&desc);
            d.insert(Str::from("report-content"), desc_typ.into_value());
            d.insert(Str::from("status"), f.status.as_str().into_value());

            let img_values: Vec<Value> = f
                .images
                .iter()
                .filter_map(|img| {
                    Path::new(img)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|basename| format!("{}-{}", f.slug, basename).into_value())
                })
                .collect();
            d.insert(
                Str::from("images"),
                Value::Array(img_values.into_iter().collect()),
            );

            Value::Dict(d)
        })
        .collect();

    let count = |sev: Severity| -> i64 {
        findings.iter().filter(|f| f.severity == sev).count() as i64
    };

    let mut inputs = Dict::new();
    inputs.insert(
        Str::from("findings"),
        Value::Array(finding_values.into_iter().collect()),
    );
    inputs.insert(Str::from("date"), current_date().into_value());
    inputs.insert(Str::from("asset"), asset.into_value());
    inputs.insert(Str::from("from"), from.into_value());
    inputs.insert(Str::from("to"), to.into_value());
    inputs.insert(
        Str::from("total"),
        (findings.len() as i64).into_value(),
    );
    inputs.insert(
        Str::from("critical"),
        count(Severity::Critical).into_value(),
    );
    inputs.insert(
        Str::from("high"),
        count(Severity::High).into_value(),
    );
    inputs.insert(
        Str::from("medium"),
        count(Severity::Medium).into_value(),
    );
    inputs.insert(
        Str::from("low"),
        count(Severity::Low).into_value(),
    );
    inputs.insert(
        Str::from("info"),
        count(Severity::Info).into_value(),
    );

    inputs
}

// ───────────────────────── markdown → typst ─────────────────────────

/// Convert a markdown report content into Typst markup.
///
/// Handles the subset of markdown commonly found in finding report content:
/// headings, bold, italic, bold-italic, inline code, fenced code blocks,
/// bullet lists, images, links, and plain paragraphs.
fn md_to_typst(md: &str) -> String {
    let mut out = String::with_capacity(md.len() * 2);
    let mut in_code_block = false;
    let mut code_buf = String::new();
    let mut in_list = false;
    let mut table_rows: Vec<String> = Vec::new();

    for line in md.lines() {
        let trimmed = line.trim();

        // ── fenced code blocks ──
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            if in_code_block {
                out.push_str("```\n");
                in_code_block = false;
                continue;
            } else {
                let lang = trimmed.trim_start_matches('`').trim_start_matches('~').trim();
                if in_list {
                    in_list = false;
                }
                flush_table(&mut table_rows, &mut out);
                if lang.is_empty() {
                    out.push_str("```\n");
                } else {
                    out.push_str(&format!("```{lang}\n"));
                }
                in_code_block = true;
                code_buf.clear();
                continue;
            }
        }

        if in_code_block {
            out.push_str(line);
            out.push('\n');
            continue;
        }

        // ── table rows (lines starting and ending with |) ──
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            if in_list {
                in_list = false;
            }
            table_rows.push(trimmed.to_string());
            continue;
        }

        // If we were accumulating table rows and now hit a non-table line,
        // flush the table before processing this line.
        flush_table(&mut table_rows, &mut out);

        // ── blank line: paragraph break ──
        if trimmed.is_empty() {
            if in_list {
                in_list = false;
            }
            out.push('\n');
            continue;
        }

        // ── headings ──
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|&c| c == '#').count();
            let text = trimmed[level..].trim();
            if in_list {
                in_list = false;
            }
            // Typst headings: = / == / === / ====
            let prefix: String = "=".repeat(level);
            out.push_str(&format!("{prefix} {}\n", convert_inline(text)));
            continue;
        }

        // ── bullet lists ──
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            in_list = true;
            let content = trimmed[2..].trim();
            out.push_str(&format!("- {}\n", convert_inline(content)));
            continue;
        }

        // ── numbered lists ──
        if let Some(rest) = try_strip_numbered_prefix(trimmed) {
            in_list = true;
            out.push_str(&format!("+ {}\n", convert_inline(rest)));
            continue;
        }

        // ── plain paragraph line ──
        if in_list {
            in_list = false;
            out.push('\n');
        }
        out.push_str(&convert_inline(trimmed));
        out.push('\n');
    }

    // Close any unclosed code block
    if in_code_block {
        out.push_str("```\n");
    }

    // Flush any trailing table
    flush_table(&mut table_rows, &mut out);

    out.trim().to_string()
}

/// Returns true if every cell in the row consists only of dashes, colons,
/// and spaces — i.e. it is a markdown table separator like `|---|:---:|`.
fn is_separator_row(row: &str) -> bool {
    let inner = row.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').all(|cell| {
        let c = cell.trim();
        !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
    })
}

/// Parse the cells out of a markdown table row like `| a | b | c |`.
fn parse_table_cells(row: &str) -> Vec<String> {
    let inner = row.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

/// If `rows` is non-empty, emit a Typst `#table(…)` and clear the buffer.
fn flush_table(rows: &mut Vec<String>, out: &mut String) {
    if rows.is_empty() {
        return;
    }

    // Separate header, separator, and data rows.
    // Markdown tables: row 0 = header, row 1 = separator, rest = data.
    let mut header: Option<Vec<String>> = None;
    let mut data: Vec<Vec<String>> = Vec::new();

    for (i, row) in rows.iter().enumerate() {
        if is_separator_row(row) {
            continue; // skip separator
        }
        let cells = parse_table_cells(row);
        if i == 0 {
            header = Some(cells);
        } else {
            data.push(cells);
        }
    }

    let ncols = header
        .as_ref()
        .map(|h| h.len())
        .unwrap_or_else(|| data.first().map(|r| r.len()).unwrap_or(1));

    // Build columns spec: all auto
    let cols_spec = std::iter::repeat("auto")
        .take(ncols)
        .collect::<Vec<_>>()
        .join(", ");

    out.push_str(&format!(
        "#table(\n  columns: ({cols_spec}),\n  inset: 6pt,\n  stroke: 0.4pt + gray,\n"
    ));

    if let Some(hdr) = &header {
        out.push_str("  table.header(");
        for (j, cell) in hdr.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            let converted = convert_inline(cell);
            out.push_str(&format!("[*{converted}*]"));
        }
        out.push_str("),\n");
    }

    for row_cells in &data {
        out.push_str("  ");
        for (j, cell) in row_cells.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            let converted = convert_inline(cell);
            out.push_str(&format!("[{converted}]"));
        }
        out.push_str(",\n");
    }

    out.push_str(")\n");

    rows.clear();
}

/// Try to strip a numbered list prefix like `1. `, `2) `, etc.
fn try_strip_numbered_prefix(s: &str) -> Option<&str> {
    let mut chars = s.chars();
    if !chars.next().map_or(false, |c| c.is_ascii_digit()) {
        return None;
    }
    // consume remaining digits
    let rest = s.trim_start_matches(|c: char| c.is_ascii_digit());
    if rest.starts_with(". ") || rest.starts_with(") ") {
        Some(rest[2..].trim())
    } else {
        None
    }
}

/// Convert inline markdown spans to Typst markup.
///
/// Handles: `**bold**`, `*italic*`, `***bold-italic***`, `` `code` ``,
/// `[text](url)`, `![alt](path)`.
fn convert_inline(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;

    while i < len {
        // ── inline code: `…` (must be checked first to avoid
        //    interpreting markdown inside code spans) ──
        if chars[i] == '`' {
            if let Some(end) = text[byte_pos(text, i) + 1..].find('`') {
                let code = &text[byte_pos(text, i) + 1..][..end];
                out.push('`');
                out.push_str(code);
                out.push('`');
                i += 1 + char_count(code) + 1;
                continue;
            }
        }

        // ── image: ![alt](path) ──
        if i + 1 < len && chars[i] == '!' && chars[i + 1] == '[' {
            if let Some((alt, path, consumed)) = parse_md_image(&text[byte_pos(text, i)..]) {
                out.push_str(&format!("#figure(image(\"{path}\"), caption: [{alt}])"));
                i += char_count(&text[byte_pos(text, i)..][..consumed]);
                continue;
            }
        }

        // ── link: [text](url) ──
        if chars[i] == '[' {
            if let Some((display, url, end_byte)) = try_parse_md_link(&text[byte_pos(text, i)..]) {
                out.push_str(&format!("#link(\"{url}\")[{display}]"));
                i += char_count(&text[byte_pos(text, i)..][..end_byte]);
                continue;
            }
        }

        // ── bold-italic: ***…*** ──
        if i + 2 < len && chars[i] == '*' && chars[i + 1] == '*' && chars[i + 2] == '*' {
            if let Some((content, skip)) = extract_between_str(&text[byte_pos(text, i) + 3..], "***") {
                out.push_str(&format!("*_{content}_*"));
                i += 3 + char_count(&text[byte_pos(text, i) + 3..][..skip + 3]);
                continue;
            }
        }

        // ── bold: **…** ──
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some((content, skip)) = extract_between_str(&text[byte_pos(text, i) + 2..], "**") {
                out.push_str(&format!("*{content}*"));
                i += 2 + char_count(&text[byte_pos(text, i) + 2..][..skip + 2]);
                continue;
            }
        }

        // ── italic: *…* ──
        if chars[i] == '*' {
            if let Some((content, skip)) = extract_between_str(&text[byte_pos(text, i) + 1..], "*") {
                out.push_str(&format!("_{content}_"));
                i += 1 + char_count(&text[byte_pos(text, i) + 1..][..skip + 1]);
                continue;
            }
        }

        out.push(chars[i]);
        i += 1;
    }

    out
}

/// Find byte position of the i-th char in a string.
fn byte_pos(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(pos, _)| pos)
        .unwrap_or(s.len())
}

/// Count the number of chars in a string slice.
fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// Extract content between the current position and the next occurrence
/// of `delim` in the string. Returns `(content, byte_offset_of_delim_start)`.
fn extract_between_str<'a>(s: &'a str, delim: &str) -> Option<(&'a str, usize)> {
    let pos = s.find(delim)?;
    if pos == 0 {
        return None; // empty content
    }
    Some((&s[..pos], pos))
}

/// Try to parse `[text](url)` from the start of `s`.
/// Returns `(display, url, total_bytes_consumed)`.
fn try_parse_md_link(s: &str) -> Option<(String, String, usize)> {
    if !s.starts_with('[') {
        return None;
    }
    let after = &s[1..];
    let close_bracket = after.find(']')?;
    let display = after[..close_bracket].to_string();
    let rest = &after[close_bracket + 1..];
    if !rest.starts_with('(') {
        return None;
    }
    let close_paren = rest[1..].find(')')?;
    let url = rest[1..1 + close_paren].to_string();
    let consumed = 1 + close_bracket + 1 + 1 + close_paren + 1;
    Some((display, url, consumed))
}

// ───────────────────────── image helpers ─────────────────────────

/// Copy all images from the POGDIR finding directories into the work directory
/// so they can be found by Typst during compilation.
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

/// Rewrite markdown image references in a finding report content so that the
/// basenames match the files copied into the work directory by
/// [`prepare_finding_images`].
fn rewrite_report_content_images(desc: &str, images: &[String], slug: &str) -> String {
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
/// are available during Typst compilation.  This makes templates fully
/// self-contained — they can reference their own images in Typst markup
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
    use models::{Finding, Severity, Status};

    // ── build_inputs ──

    #[test]
    fn build_inputs_contains_expected_keys() {
        let f = Finding::new(
            "SQL Injection",
            Severity::Critical,
            "web.app",
            "2025/01/01",
            "/login",
            "Vuln description",
            Status::Open,
        );
        let dict = build_inputs(&[f], "web.app", "ACME Corp", "Security Ltd");
        assert!(dict.get("findings").is_ok());
        assert!(dict.get("asset").is_ok());
        assert!(dict.get("from").is_ok());
        assert!(dict.get("to").is_ok());
        assert!(dict.get("date").is_ok());
    }

    // ── rewrite_report_content_images ──

    #[test]
    fn rewrite_images_no_images() {
        let desc = "No images here.";
        assert_eq!(rewrite_report_content_images(desc, &[], "slug"), "No images here.");
    }

    #[test]
    fn rewrite_images_matching_basename() {
        let desc = "See ![proof](../img/xss.jpg) for details.";
        let images = vec!["img/xss.jpg".to_string()];
        let result = rewrite_report_content_images(desc, &images, "stored-xss");
        assert_eq!(result, "See ![proof](stored-xss-xss.jpg) for details.");
    }

    #[test]
    fn rewrite_images_no_match() {
        let desc = "See ![proof](../img/other.jpg) for details.";
        let images = vec!["img/xss.jpg".to_string()];
        let result = rewrite_report_content_images(desc, &images, "stored-xss");
        assert_eq!(result, "See ![proof](../img/other.jpg) for details.");
    }

    #[test]
    fn rewrite_images_multiple() {
        let desc = "![a](img/one.png) and ![b](img/two.png)";
        let images = vec!["img/one.png".to_string(), "img/two.png".to_string()];
        let result = rewrite_report_content_images(desc, &images, "vuln");
        assert!(result.contains("vuln-one.png"));
        assert!(result.contains("vuln-two.png"));
    }

    // ── parse_md_image ──

    #[test]
    fn parse_md_image_valid() {
        let result = parse_md_image("![alt text](path/to/img.png)");
        assert!(result.is_some());
        let (alt, path, end) = result.unwrap();
        assert_eq!(alt, "alt text");
        assert_eq!(path, "path/to/img.png");
        assert_eq!(end, "![alt text](path/to/img.png)".len());
    }

    #[test]
    fn parse_md_image_not_image() {
        assert!(parse_md_image("just text").is_none());
        assert!(parse_md_image("[link](url)").is_none());
    }

    // ── md_to_typst ──

    #[test]
    fn md_to_typst_headings() {
        assert_eq!(md_to_typst("# Heading"), "= Heading");
        assert_eq!(md_to_typst("## Sub"), "== Sub");
        assert_eq!(md_to_typst("### Deep"), "=== Deep");
    }

    #[test]
    fn md_to_typst_bold_italic() {
        assert_eq!(md_to_typst("**bold**"), "*bold*");
        assert_eq!(md_to_typst("*italic*"), "_italic_");
        assert_eq!(md_to_typst("***both***"), "*_both_*");
    }

    #[test]
    fn md_to_typst_inline_code() {
        assert_eq!(md_to_typst("use `foo` here"), "use `foo` here");
    }

    #[test]
    fn md_to_typst_code_block() {
        let md = "before\n```python\nprint(1)\n```\nafter";
        let typ = md_to_typst(md);
        assert!(typ.contains("```python\nprint(1)\n```"));
    }

    #[test]
    fn md_to_typst_link() {
        assert_eq!(
            md_to_typst("[click](https://example.com)"),
            "#link(\"https://example.com\")[click]"
        );
    }

    #[test]
    fn md_to_typst_image() {
        let result = md_to_typst("![alt text](img.png)");
        assert_eq!(result, "#figure(image(\"img.png\"), caption: [alt text])");
    }

    #[test]
    fn md_to_typst_bullet_list() {
        assert_eq!(md_to_typst("- item one\n- item two"), "- item one\n- item two");
    }

    #[test]
    fn md_to_typst_numbered_list() {
        assert_eq!(md_to_typst("1. first\n2. second"), "+ first\n+ second");
    }

    #[test]
    fn md_to_typst_table() {
        let md = "| Name | Value |\n|------|-------|\n| foo  | 42    |\n| bar  | 99    |";
        let typ = md_to_typst(md);
        assert!(typ.contains("#table("));
        assert!(typ.contains("columns:"));
        assert!(typ.contains("[*Name*]"));
        assert!(typ.contains("[*Value*]"));
        assert!(typ.contains("[foo]"));
        assert!(typ.contains("[42]"));
        assert!(typ.contains("[bar]"));
        assert!(typ.contains("[99]"));
    }

    #[test]
    fn md_to_typst_table_with_inline() {
        let md = "| Header |\n|--------|\n| **bold** |";
        let typ = md_to_typst(md);
        assert!(typ.contains("[*Header*]"));
        assert!(typ.contains("[*bold*]"));
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
}
