use std::fs;
use std::path::Path;

use models::{Finding, Severity, Status};

use crate::error::{Result, StorageError};
use crate::pogdir::PogDir;

/// Import a single finding from a folder.
///
/// The folder is expected to contain:
/// - Exactly one `.md` file (the finding write-up)
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
/// Expected front-matter format (lines at the top of the file):
///
/// ```markdown
/// # Title of Finding
///
/// - **Severity:** Critical
/// - **Asset:** web_app
/// - **Location:** https://example.com/vuln
/// - **Status:** Open
///
/// ## Description
///
/// Free-form description text…
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
    let mut description = String::new();
    let mut in_description = false;

    for line in raw.lines() {
        let trimmed = line.trim();

        // Title: first `# …` heading
        if !in_description && trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            title = trimmed.trim_start_matches('#').trim().to_string();
            continue;
        }

        // Section heading for description
        if trimmed.starts_with("## ") {
            let heading = trimmed.trim_start_matches('#').trim().to_lowercase();
            in_description = heading == "description";
            continue;
        }

        // Metadata bullet points
        if !in_description {
            if let Some(value) = extract_field(trimmed, "severity") {
                severity = value.parse().unwrap_or(Severity::Info);
            } else if let Some(value) = extract_field(trimmed, "asset") {
                asset = normalise_asset(&value);
            } else if let Some(value) = extract_field(trimmed, "date") {
                date = value;
            } else if let Some(value) = extract_field(trimmed, "location") {
                location = value;
            } else if let Some(value) = extract_field(trimmed, "status") {
                status = value.parse().unwrap_or(Status::Open);
            }
            continue;
        }

        // Accumulate description lines
        if in_description {
            if !description.is_empty() {
                description.push('\n');
            }
            description.push_str(line);
        }
    }

    let description = description.trim().to_string();

    Ok(Finding {
        id: None,
        title,
        severity,
        asset,
        date,
        location,
        description,
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_md() -> &'static str {
        "\
# SQL Injection

- **Severity:** Critical
- **Asset:** Web App
- **Date:** 2026/01/15
- **Location:** https://example.com/api/users?id=1
- **Status:** Open

## Description

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
        assert!(f.description.contains("User input is directly concatenated"));
    }

    #[test]
    fn test_parse_minimal_md() {
        let md = "# Buffer Overflow\n\n## Description\n\nStack smash.\n";
        let f = parse_finding_md(md, "buffer-overflow").unwrap();
        assert_eq!(f.title, "Buffer Overflow");
        assert_eq!(f.severity, Severity::Info); // default
        assert_eq!(f.asset, "unknown");         // default
        assert_eq!(f.date, "");                 // default
        assert_eq!(f.status, Status::Open);     // default
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
            "# Finding A\n\n- **Severity:** High\n- **Asset:** web_app\n- **Date:** 2026/01/15\n\n## Description\n\nDesc A\n");
        create_finding_folder(&tmp, "finding-b",
            "# Finding B\n\n- **Severity:** Low\n- **Asset:** web_app\n- **Date:** 2026/01/16\n\n## Description\n\nDesc B\n");

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
            "# SQL Injection v2\n\n- **Severity:** Critical\n- **Asset:** Web App\n- **Date:** 2026/01/20\n- **Status:** Resolved\n\n## Description\n\nFixed.\n"
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
        assert_eq!(
            extract_field("- **Severity:** Critical", "severity"),
            Some("Critical".into())
        );
        assert_eq!(
            extract_field("- Severity: High", "severity"),
            Some("High".into())
        );
        assert_eq!(
            extract_field("- **Location:** https://example.com", "location"),
            Some("https://example.com".into())
        );
        assert_eq!(
            extract_field("- **Asset:** Web App", "asset"),
            Some("Web App".into())
        );
        assert_eq!(
            extract_field("- **Date:** 2026/01/15", "date"),
            Some("2026/01/15".into())
        );
        assert_eq!(extract_field("random line", "severity"), None);
    }
}
