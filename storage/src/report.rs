// storage/src/report.rs — LaTeX-based PDF report generation via pdflatex.
//
// The public entry point is [`generate_report`].  It renders a MiniJinja
// template (`.tmpl`), parses the result into `Block`s, converts them to a
// LaTeX document, writes the `.tex` source to a temp directory, invokes
// `pdflatex`, and copies the finished PDF to the requested output path.

use crate::error::{Result, StorageError};
use models::{Finding, Severity};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

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
) -> Result<()> {
    let raw = fs::read_to_string(template_path)?;

    // ── build MiniJinja context ──
    let mut env = minijinja::Environment::new();
    env.add_template("report", &raw)
        .map_err(|e| StorageError::ParseError(e.to_string()))?;
    let tmpl = env
        .get_template("report")
        .map_err(|e| StorageError::ParseError(e.to_string()))?;

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
            map.insert("description".to_string(), minijinja::Value::from(f.description.as_str()));
            map.insert("status".to_string(), minijinja::Value::from(f.status.as_str()));
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
        .map_err(|e| StorageError::ParseError(e.to_string()))?;

    // ── parse blocks and render via LaTeX ──
    let blocks = parse_blocks(&rendered);
    let latex_src = blocks_to_latex(&blocks);
    render_pdf(&latex_src, output_path)?;

    Ok(())
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
            }
            // #! comment and #! image are silently ignored
            continue;
        }

        // ── pipe-delimited table row ──
        if trimmed.contains('|') && !trimmed.starts_with('-') {
            flush_text(&mut text_buf, &mut blocks);
            let cols: Vec<String> = trimmed.split('|').map(|c| c.trim().to_string()).collect();
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
fn blocks_to_latex(blocks: &[Block]) -> String {
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
                // data rows
                for row in rows.iter().skip(1) {
                    let cells: Vec<String> =
                        row.iter().map(|c| latex_escape(c)).collect();
                    body.push_str(&cells.join(" & "));
                    body.push_str(" \\\\\n");
                }
                body.push_str("\\bottomrule\n\\end{tabularx}\n\\vspace{4mm}\n\n");
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
        PREAMBLE = latex_preamble(),
        body = body,
    )
}

/// The LaTeX preamble: document class, packages, colour definitions, and
/// style settings that produce a professional-looking security report.
fn latex_preamble() -> String {
    r#"\documentclass[11pt,a4paper]{article}

% ── geometry ──
\usepackage[top=25mm,bottom=30mm,left=25mm,right=25mm]{geometry}

% ── encoding & fonts ──
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
\fancyhead[L]{\small\color{CorpGray}\textit{Security Assessment Report}}
\fancyhead[R]{\small\color{CorpGray}\thepage}
\fancyfoot[C]{\small\color{CorpGray}\textit{Confidential}}
\renewcommand{\footrulewidth}{0pt}
"#
    .to_string()
}

// ───────────────────────── PDF compilation ─────────────────────────

/// Write the LaTeX source to a temp directory, invoke `pdflatex` twice
/// (for TOC resolution), and copy the resulting PDF to `output_path`.
fn render_pdf(latex_src: &str, output_path: &str) -> Result<()> {
    let tmp = std::env::temp_dir().join(format!("pog_report_{}", std::process::id()));
    fs::create_dir_all(&tmp)?;

    let tex_path = tmp.join("report.tex");
    {
        let mut f = fs::File::create(&tex_path)?;
        f.write_all(latex_src.as_bytes())?;
    }

    // Find pdflatex
    let pdflatex = find_pdflatex().ok_or_else(|| {
        StorageError::ParseError(
            "pdflatex not found. Please install a TeX distribution (e.g. texlive).".to_string(),
        )
    })?;

    // Run pdflatex twice so that TOC / hyperref references resolve.
    for _ in 0..2 {
        let output = Command::new(&pdflatex)
            .args([
                "-interaction=nonstopmode",
                "-halt-on-error",
                "-output-directory",
            ])
            .arg(&tmp)
            .arg(&tex_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stdout);
            // pdflatex writes errors to stdout, not stderr.
            return Err(StorageError::ParseError(format!(
                "pdflatex failed (exit {}):\n{}",
                output.status,
                stderr.lines().rev().take(30).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n"),
            )));
        }
    }

    let pdf_path = tmp.join("report.pdf");
    if !pdf_path.exists() {
        return Err(StorageError::ParseError(
            "pdflatex ran successfully but report.pdf was not created".to_string(),
        ));
    }

    // Ensure output directory exists
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&pdf_path, output_path)?;

    // Best-effort cleanup
    let _ = fs::remove_dir_all(&tmp);

    Ok(())
}

/// Search for `pdflatex` in `$PATH` and common TeX Live locations.
fn find_pdflatex() -> Option<String> {
    // Try $PATH first
    if let Ok(output) = Command::new("which").arg("pdflatex").output() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() && output.status.success() {
            return Some(path);
        }
    }

    // Common install locations
    let candidates = [
        "/usr/bin/pdflatex",
        "/usr/local/bin/pdflatex",
        "/usr/local/texlive/2024/bin/x86_64-linux/pdflatex",
        "/usr/local/texlive/2025/bin/x86_64-linux/pdflatex",
        "/Library/TeX/texbin/pdflatex",
    ];
    for c in &candidates {
        if Path::new(c).exists() {
            return Some(c.to_string());
        }
    }

    None
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

    // ── blocks_to_latex ──

    #[test]
    fn btl_title() {
        let latex = blocks_to_latex(&[Block::Title("My Report".into())]);
        assert!(latex.contains("My Report"));
    }

    #[test]
    fn btl_subtitle() {
        let latex = blocks_to_latex(&[Block::Subtitle("acme.corp".into())]);
        assert!(latex.contains("acme.corp"));
    }

    #[test]
    fn btl_section() {
        let latex = blocks_to_latex(&[Block::Section("Details".into())]);
        assert!(latex.contains(r"\section{Details}"));
    }

    #[test]
    fn btl_finding() {
        let latex = blocks_to_latex(&[Block::Finding("Critical".into(), "SQLi".into())]);
        assert!(latex.contains("SQLi"));
        assert!(latex.contains("Critical"));
    }

    #[test]
    fn btl_meta() {
        let latex = blocks_to_latex(&[Block::Meta("Asset".into(), "web.corp".into())]);
        assert!(latex.contains("Asset"));
        assert!(latex.contains("web.corp"));
    }

    #[test]
    fn btl_table() {
        let rows = vec![
            vec!["A".into(), "B".into()],
            vec!["1".into(), "2".into()],
        ];
        let latex = blocks_to_latex(&[Block::Table(rows)]);
        assert!(latex.contains(r"\begin{tabularx}"));
        assert!(latex.contains("1 & 2"));
    }

    #[test]
    fn btl_table_three_cols() {
        let rows = vec![
            vec!["A".into(), "B".into(), "C".into()],
            vec!["1".into(), "2".into(), "3".into()],
        ];
        let latex = blocks_to_latex(&[Block::Table(rows)]);
        assert!(latex.contains(r"\begin{tabularx}"));
        assert!(latex.contains("1 & 2 & 3"));
    }

    #[test]
    fn btl_text_markdown() {
        let latex = blocks_to_latex(&[Block::Text("**bold** text".into())]);
        assert!(latex.contains(r"\textbf{bold}"));
    }

    #[test]
    fn btl_index() {
        let latex = blocks_to_latex(&[Block::Index]);
        assert!(latex.contains(r"\tableofcontents"));
    }

    #[test]
    fn btl_spacer() {
        let latex = blocks_to_latex(&[Block::Spacer(10.0)]);
        assert!(latex.contains(r"\vspace{10mm}"));
    }

    #[test]
    fn btl_pagebreak() {
        let latex = blocks_to_latex(&[Block::PageBreak]);
        assert!(latex.contains(r"\clearpage"));
    }

    #[test]
    fn btl_hrule() {
        let latex = blocks_to_latex(&[Block::HRule]);
        assert!(latex.contains(r"\rule"));
    }

    #[test]
    fn btl_full_document_structure() {
        let latex = blocks_to_latex(&[Block::Title("T".into())]);
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

    // ── find_pdflatex ──

    #[test]
    fn find_pdflatex_returns_option() {
        // Just verify it doesn't panic; availability depends on system.
        let _result = find_pdflatex();
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
        let latex = blocks_to_latex(&blocks);

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
        let latex = blocks_to_latex(&blocks);

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
        let latex = blocks_to_latex(&blocks);
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
        let latex = blocks_to_latex(&blocks);
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
        let latex = blocks_to_latex(&blocks);
        let clearpage_count = latex.matches(r"\clearpage").count();
        assert!(clearpage_count >= 2);
    }

    // ── render_pdf error when pdflatex missing ──

    #[test]
    fn render_pdf_with_empty_latex_handles_error() {
        // This tests that render_pdf handles missing pdflatex gracefully
        // or pdflatex errors on empty content — either way, no panic.
        let result = render_pdf("", "/tmp/pog_test_nonexistent.pdf");
        // If pdflatex is not installed, we expect a ParseError.
        // If it is installed, it will fail on empty input.
        assert!(result.is_err());
    }
}
