use std::fs;
use std::path::Path;

use models::{Asset, Finding, Severity, Status};

use crate::error::{Result, StorageError};
use crate::pogdir::PogDir;

/// Import a single finding from a folder.
///
/// The folder is expected to contain:
/// - Exactly one `.md` file with YAML front-matter between `---` fences
///   followed by the markdown report content
/// - Optionally an `img/` subdirectory with screenshots / proof images
///
/// The folder name is used as the finding's *slug* (unique identifier).
///
/// Findings are stored under `<POGDIR>/findings/<asset>/<hex_id>_<slug>/`.
pub fn import_finding(folder: &Path, pog: &PogDir) -> Result<Finding> {
    let slug = folder
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| StorageError::ParseError("cannot derive slug from folder name".into()))?
        .to_string();

    let md_path = find_markdown(folder)?;
    let raw = fs::read_to_string(&md_path)?;
    let mut finding = parse_finding_md(&raw, &slug)?;

    // Collect images -------------------------------------------------------
    let img_dir = folder.join("img");
    if img_dir.is_dir() {
        for entry in fs::read_dir(&img_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
            {
                finding.images.push(format!("img/{name}"));
            }
        }
        finding.images.sort();
    }

    // Persist to database (assigns hex_id) ---------------------------------
    let db = pog.open_db()?;
    let (id, hex_id, _is_new) = db.upsert_finding(&finding, &slug)?;
    finding.id = Some(id);
    finding.hex_id = hex_id.clone();

    // Copy files into POGDIR -----------------------------------------------
    let dest = pog.finding_dir(&finding.asset, &hex_id, &slug);
    fs::create_dir_all(&dest)?;
    fs::copy(&md_path, dest.join(md_path.file_name().unwrap()))?;

    if img_dir.is_dir() {
        let dest_img = dest.join("img");
        fs::create_dir_all(&dest_img)?;
        for entry in fs::read_dir(&img_dir)? {
            let entry = entry?;
            let src = entry.path();
            if src.is_file() {
                fs::copy(&src, dest_img.join(entry.file_name()))?;
            }
        }
    }

    Ok(finding)
}

/// Bulk-import: treat every sub-directory of `folder` as a finding folder.
pub fn import_bulk(folder: &Path, pog: &PogDir) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    let mut entries: Vec<_> = fs::read_dir(folder)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let finding = import_finding(&entry.path(), pog)?;
        findings.push(finding);
    }

    Ok(findings)
}

// ---------------------------------------------------------------------------
// Markdown parsing helpers
// ---------------------------------------------------------------------------

/// Locate the first `.md` file in a directory.
fn find_markdown(dir: &Path) -> Result<std::path::PathBuf> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension()
            && ext.eq_ignore_ascii_case("md")
        {
            return Ok(path);
        }
    }
    Err(StorageError::MissingMarkdown(dir.display().to_string()))
}

/// Parse a finding markdown file.
///
/// Expected format: YAML front-matter between `---` fences followed by
/// the free-form markdown report content.
///
/// ```markdown
/// ---
/// title: SQL Injection
/// severity: Critical
/// asset: web_app
/// location: https://example.com/api/users?id=1
/// date: 2025/10/02
/// status: Open
/// ---
///
/// The `id` parameter is directly concatenated into a raw SQL query …
/// ```
///
/// Parsing is intentionally lenient: missing fields get sensible defaults.
/// The asset field is normalised to lowercase with underscores for spaces.
fn parse_finding_md(raw: &str, slug: &str) -> Result<Finding> {
    let mut title = slug.to_string();
    let mut severity = Severity::Info;
    let mut asset = String::from("unknown");
    let mut date = String::new();
    let mut location = String::new();
    let mut status = Status::Open;
    let report_content;

    // ── split on front-matter fences ──
    let trimmed = raw.trim_start();
    if trimmed.starts_with("---") {
        // Find the closing `---`
        let after_open = &trimmed[3..];
        // Skip the rest of the opening line (e.g. trailing whitespace)
        let after_open = after_open.trim_start_matches(|c: char| c != '\n');
        let after_open = after_open.strip_prefix('\n').unwrap_or(after_open);

        if let Some(close) = after_open.find("\n---") {
            let front = &after_open[..close];
            let body = &after_open[close + 4..]; // skip "\n---"
            // Skip the rest of the closing `---` line
            let body = body.trim_start_matches(|c: char| c != '\n');
            let body = body.strip_prefix('\n').unwrap_or(body);

            // ── parse front-matter key: value lines ──
            for line in front.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim().to_lowercase();
                    let value = value.trim().to_string();
                    match key.as_str() {
                        "title" => title = value,
                        "severity" => severity = value.parse().unwrap_or(Severity::Info),
                        "asset" => asset = normalise_asset(&value),
                        "date" => date = value,
                        "location" => location = value,
                        "status" => status = value.parse().unwrap_or(Status::Open),
                        _ => {} // ignore unknown keys
                    }
                }
            }

            report_content = body.trim().to_string();
        } else {
            // Opening `---` but no closing fence — treat everything as report content
            report_content = raw.trim().to_string();
        }
    } else {
        // No front-matter at all — whole file is report content
        report_content = raw.trim().to_string();
    }

    Ok(Finding {
        id: None,
        hex_id: String::new(),
        slug: slug.to_string(),
        title,
        severity,
        asset,
        date,
        location,
        report_content,
        status,
        images: Vec::new(),
    })
}

/// Normalise an asset name: lowercase, spaces → underscores, collapse
/// consecutive underscores, strip leading/trailing underscores.
fn normalise_asset(raw: &str) -> String {
    let s: String = raw
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    // collapse multiple underscores
    let mut out = String::with_capacity(s.len());
    let mut prev_underscore = true; // strip leading _
    for c in s.chars() {
        if c == '_' {
            if !prev_underscore {
                out.push('_');
            }
            prev_underscore = true;
        } else {
            out.push(c);
            prev_underscore = false;
        }
    }
    // strip trailing _
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() { "unknown".into() } else { out }
}

/// Try to extract a value from a metadata line like `- **Severity:** High`.
fn extract_field(line: &str, field: &str) -> Option<String> {
    let lower = line.to_lowercase();
    // Handles both `- **Field:** value` and `- Field: value`
    let patterns = [
        format!("**{}:**", field),
        format!("{}:", field),
    ];
    for pat in &patterns {
        if let Some(pos) = lower.find(pat.as_str()) {
            let start = pos + pat.len();
            let value = line[start..].trim().to_string();
            return Some(value);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Asset import
// ---------------------------------------------------------------------------

/// Import a single asset from a Markdown file.
///
/// The file should have the format:
///
/// ```markdown
/// # asset_name
///
/// - **Description:** ...
/// - **Contact:** ...
/// - **Criticality:** ...
/// - **DNS/IP:** ...
/// ```
///
/// Only the name (title) is required; other fields default to `-`.
pub fn import_asset(file: &Path, pog: &PogDir) -> Result<Asset> {
    let raw = fs::read_to_string(file)?;
    let asset = parse_asset_md(&raw)?;
    let db = pog.open_db()?;
    let id = db.upsert_asset(&asset)?;
    write_asset_md(&asset, pog)?;
    Ok(Asset { id: Some(id), ..asset })
}

/// Bulk-import assets: the file contains multiple assets separated by `---`.
///
/// Each section follows the same markdown format as a single asset.
pub fn import_assets_bulk(file: &Path, pog: &PogDir) -> Result<Vec<Asset>> {
    let raw = fs::read_to_string(file)?;
    let mut assets = Vec::new();
    let db = pog.open_db()?;

    // Split on `---` lines
    let sections: Vec<&str> = raw.split("\n---").collect();

    for section in sections {
        let trimmed = section.trim();
        if trimmed.is_empty() {
            continue;
        }
        let asset = parse_asset_md(trimmed)?;
        let id = db.upsert_asset(&asset)?;
        write_asset_md(&asset, pog)?;
        assets.push(Asset { id: Some(id), ..asset });
    }

    Ok(assets)
}

/// Parse an asset from a markdown snippet.
fn parse_asset_md(raw: &str) -> Result<Asset> {
    let mut name = String::new();
    let mut description = String::from("-");
    let mut contact = String::from("-");
    let mut criticality = String::from("-");
    let mut dns_or_ip = String::from("-");

    for line in raw.lines() {
        let trimmed = line.trim();

        // Name: first `# …` heading
        if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            name = normalise_asset(trimmed.trim_start_matches('#').trim());
            continue;
        }

        // Metadata bullet points
        if let Some(value) = extract_field(trimmed, "description") {
            if !value.is_empty() { description = value; }
        } else if let Some(value) = extract_field(trimmed, "contact") {
            if !value.is_empty() { contact = value; }
        } else if let Some(value) = extract_field(trimmed, "criticality") {
            if !value.is_empty() { criticality = value; }
        } else if let Some(value) = extract_field(trimmed, "dns/ip") {
            if !value.is_empty() { dns_or_ip = value; }
        }
    }

    if name.is_empty() {
        return Err(StorageError::ParseError("asset must have a name (# heading)".into()));
    }

    Ok(Asset {
        id: None,
        name,
        description,
        contact,
        criticality,
        dns_or_ip,
    })
}

/// Write (or overwrite) the `asset.md` metadata file under the asset's
/// directory in POGDIR: `findings/<asset_name>/asset.md`.
fn write_asset_md(asset: &Asset, pog: &PogDir) -> Result<()> {
    let dir = pog.asset_dir(&asset.name);
    fs::create_dir_all(&dir)?;
    let md = render_asset_md(asset);
    fs::write(dir.join("asset.md"), md)?;
    Ok(())
}

/// Render an asset to its canonical Markdown representation.
fn render_asset_md(asset: &Asset) -> String {
    format!(
        "# {}\n\n- **Description:** {}\n- **Contact:** {}\n- **Criticality:** {}\n- **DNS/IP:** {}\n",
        asset.name, asset.description, asset.contact, asset.criticality, asset.dns_or_ip,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_md() -> &'static str {
        "\
---
title: SQL Injection
severity: Critical
asset: Web App
date: 2026/01/15
location: https://example.com/api/users?id=1
status: Open
---

User input is directly concatenated into SQL query without sanitization.
This allows an attacker to execute arbitrary SQL commands.
"
    }

    /// Create a minimal finding folder in a temp dir and return its path.
    fn create_finding_folder(tmp: &TempDir, name: &str, md: &str) -> std::path::PathBuf {
        let dir = tmp.path().join(name);
        fs::create_dir_all(dir.join("img")).unwrap();
        fs::write(dir.join("finding.md"), md).unwrap();
        fs::write(dir.join("img").join("proof.png"), b"fake-png").unwrap();
        dir
    }

    #[test]
    fn test_parse_finding_md() {
        let f = parse_finding_md(sample_md(), "sql-injection").unwrap();
        assert_eq!(f.title, "SQL Injection");
        assert_eq!(f.severity, Severity::Critical);
        assert_eq!(f.asset, "web_app");
        assert_eq!(f.date, "2026/01/15");
        assert_eq!(f.location, "https://example.com/api/users?id=1");
        assert_eq!(f.status, Status::Open);
        assert!(f.report_content.contains("User input is directly concatenated"));
    }

    #[test]
    fn test_parse_minimal_md() {
        let md = "---\ntitle: Buffer Overflow\n---\n\nStack smash.\n";
        let f = parse_finding_md(md, "buffer-overflow").unwrap();
        assert_eq!(f.title, "Buffer Overflow");
        assert_eq!(f.severity, Severity::Info); // default
        assert_eq!(f.asset, "unknown");         // default
        assert_eq!(f.date, "");                 // default
        assert_eq!(f.status, Status::Open);     // default
        assert!(f.report_content.contains("Stack smash"));
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let md = "Just a raw description.\n";
        let f = parse_finding_md(md, "raw-finding").unwrap();
        assert_eq!(f.title, "raw-finding"); // slug used as fallback
        assert!(f.report_content.contains("Just a raw description"));
    }

    #[test]
    fn test_normalise_asset() {
        assert_eq!(normalise_asset("Web App"), "web_app");
        assert_eq!(normalise_asset("  API Server  "), "api_server");
        assert_eq!(normalise_asset("My  Cool  App"), "my_cool_app");
        assert_eq!(normalise_asset("already_ok"), "already_ok");
        assert_eq!(normalise_asset(""), "unknown");
    }

    #[test]
    fn test_import_single_finding() {
        let tmp = TempDir::new().unwrap();
        let pog_dir = TempDir::new().unwrap();
        let pog = PogDir::init_at(pog_dir.path()).unwrap();

        let folder = create_finding_folder(&tmp, "sql-injection", sample_md());
        let f = import_finding(&folder, &pog).unwrap();

        assert_eq!(f.title, "SQL Injection");
        assert_eq!(f.asset, "web_app");
        assert!(f.id.is_some());
        assert_eq!(f.images, vec!["img/proof.png"]);

        // Verify files were copied into asset/hex_id_slug structure
        let dest = pog.finding_dir("web_app", "0x001", "sql-injection");
        assert!(dest.join("finding.md").exists());
        assert!(dest.join("img/proof.png").exists());

        // Verify DB persistence
        let db = pog.open_db().unwrap();
        let all = db.all_findings().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].title, "SQL Injection");
        assert_eq!(all[0].asset, "web_app");
    }

    #[test]
    fn test_import_bulk() {
        let tmp = TempDir::new().unwrap();
        let pog_dir = TempDir::new().unwrap();
        let pog = PogDir::init_at(pog_dir.path()).unwrap();

        create_finding_folder(&tmp, "finding-a",
            "---\ntitle: Finding A\nseverity: High\nasset: web_app\ndate: 2026/01/15\n---\n\nDesc A\n");
        create_finding_folder(&tmp, "finding-b",
            "---\ntitle: Finding B\nseverity: Low\nasset: web_app\ndate: 2026/01/16\n---\n\nDesc B\n");

        let findings = import_bulk(tmp.path(), &pog).unwrap();
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].title, "Finding A");
        assert_eq!(findings[1].title, "Finding B");

        // Both should be under web_app with incrementing hex IDs
        let db = pog.open_db().unwrap();
        let all = db.all_findings().unwrap();
        assert_eq!(all.len(), 2);

        assert!(pog.finding_dir("web_app", "0x001", "finding-a").exists());
        assert!(pog.finding_dir("web_app", "0x002", "finding-b").exists());
    }

    #[test]
    fn test_reimport_upserts() {
        let tmp = TempDir::new().unwrap();
        let pog_dir = TempDir::new().unwrap();
        let pog = PogDir::init_at(pog_dir.path()).unwrap();

        let folder = create_finding_folder(&tmp, "sqli", sample_md());
        import_finding(&folder, &pog).unwrap();

        // Update the markdown and re-import
        fs::write(folder.join("finding.md"),
            "---\ntitle: SQL Injection v2\nseverity: Critical\nasset: Web App\ndate: 2026/01/20\nstatus: Resolved\n---\n\nFixed.\n"
        ).unwrap();
        let f = import_finding(&folder, &pog).unwrap();
        assert_eq!(f.title, "SQL Injection v2");
        assert_eq!(f.status, Status::Resolved);

        let db = pog.open_db().unwrap();
        let all = db.all_findings().unwrap();
        assert_eq!(all.len(), 1); // still just one
        assert_eq!(all[0].title, "SQL Injection v2");
    }

    #[test]
    fn test_missing_md_errors() {
        let tmp = TempDir::new().unwrap();
        let pog_dir = TempDir::new().unwrap();
        let pog = PogDir::init_at(pog_dir.path()).unwrap();

        let empty = tmp.path().join("empty-folder");
        fs::create_dir_all(&empty).unwrap();
        let res = import_finding(&empty, &pog);
        assert!(res.is_err());
        let err = res.unwrap_err().to_string();
        assert!(err.contains("missing a markdown"));
    }

    #[test]
    fn test_extract_field() {
        // extract_field is still used for asset markdown parsing
        assert_eq!(
            extract_field("- **Description:** some desc", "description"),
            Some("some desc".into())
        );
        assert_eq!(
            extract_field("- **Contact:** admin@corp.com", "contact"),
            Some("admin@corp.com".into())
        );
        assert_eq!(extract_field("random line", "severity"), None);
    }
}
