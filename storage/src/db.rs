use std::path::PathBuf;

use models::{Asset, Finding, Severity, Status};
use rusqlite::{params, Connection};

use crate::error::Result;

/// Thin wrapper around the SQLite connection.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) the database at `path` and run migrations.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let conn = Connection::open(path.into())?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Open an in-memory database (useful for tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    // ------------------------------------------------------------------
    // Migrations
    // ------------------------------------------------------------------

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS findings (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                hex_id      TEXT    NOT NULL,
                title       TEXT    NOT NULL,
                severity    TEXT    NOT NULL,
                asset       TEXT    NOT NULL DEFAULT 'unknown',
                date        TEXT    NOT NULL DEFAULT '',
                location    TEXT    NOT NULL DEFAULT '',
                description TEXT    NOT NULL DEFAULT '',
                status      TEXT    NOT NULL DEFAULT 'Open',
                slug        TEXT    NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS finding_images (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                finding_id  INTEGER NOT NULL REFERENCES findings(id) ON DELETE CASCADE,
                path        TEXT    NOT NULL
            );

            CREATE TABLE IF NOT EXISTS assets (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT    NOT NULL UNIQUE,
                description TEXT    NOT NULL DEFAULT '-',
                contact     TEXT    NOT NULL DEFAULT '-',
                criticality TEXT    NOT NULL DEFAULT '-',
                dns_or_ip   TEXT    NOT NULL DEFAULT '-'
            );
            "
        )?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // ID generation
    // ------------------------------------------------------------------

    /// Return the next hex ID for a given asset, e.g. `0x001`, `0x002`, …
    pub fn next_hex_id(&self, asset: &str) -> Result<String> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM findings WHERE asset = ?1",
            params![asset],
            |row| row.get(0),
        )?;
        Ok(format!("0x{:03X}", count + 1))
    }

    // ------------------------------------------------------------------
    // Write operations
    // ------------------------------------------------------------------

    /// Insert a finding. Returns the new row id.
    pub fn insert_finding(&self, finding: &Finding, slug: &str, hex_id: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO findings (hex_id, title, severity, asset, date, location, description, status, slug)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                hex_id,
                finding.title,
                finding.severity.as_str(),
                finding.asset,
                finding.date,
                finding.location,
                finding.description,
                finding.status.as_str(),
                slug,
            ],
        )?;
        let id = self.conn.last_insert_rowid();

        for img in &finding.images {
            self.conn.execute(
                "INSERT INTO finding_images (finding_id, path) VALUES (?1, ?2)",
                params![id, img],
            )?;
        }

        Ok(id)
    }

    /// Update an existing finding by its slug, or insert if new.
    /// Returns `(row_id, hex_id, is_new)`.
    pub fn upsert_finding(&self, finding: &Finding, slug: &str) -> Result<(i64, String, bool)> {
        let existing: Option<(i64, String)> = self.conn
            .query_row(
                "SELECT id, hex_id FROM findings WHERE slug = ?1",
                params![slug],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        if let Some((id, hex_id)) = existing {
            self.conn.execute(
                "UPDATE findings SET title = ?1, severity = ?2, asset = ?3, date = ?4,
                 location = ?5, description = ?6, status = ?7 WHERE id = ?8",
                params![
                    finding.title,
                    finding.severity.as_str(),
                    finding.asset,
                    finding.date,
                    finding.location,
                    finding.description,
                    finding.status.as_str(),
                    id,
                ],
            )?;
            // Replace images
            self.conn.execute("DELETE FROM finding_images WHERE finding_id = ?1", params![id])?;
            for img in &finding.images {
                self.conn.execute(
                    "INSERT INTO finding_images (finding_id, path) VALUES (?1, ?2)",
                    params![id, img],
                )?;
            }
            Ok((id, hex_id, false))
        } else {
            let hex_id = self.next_hex_id(&finding.asset)?;
            let id = self.insert_finding(finding, slug, &hex_id)?;
            Ok((id, hex_id, true))
        }
    }

    // ------------------------------------------------------------------
    // Read operations
    // ------------------------------------------------------------------

    /// Load all findings from the database.
    pub fn all_findings(&self) -> Result<Vec<Finding>> {
        self.findings_filtered(None, None, None)
    }

    /// Load findings filtered by optional asset and date range.
    pub fn findings_filtered(
        &self,
        asset: Option<&str>,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Vec<Finding>> {
        let (where_clause, param_values) = build_where_clause(asset, from, to);

        let sql = format!(
            "SELECT id, hex_id, title, severity, asset, date, location, description, status, slug \
             FROM findings{} ORDER BY asset, hex_id",
            where_clause
        );

        let params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|v| v as &dyn rusqlite::types::ToSql).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params.as_slice(), |row| Ok(FindingRow::from_row(row)))?;

        let mut findings = Vec::new();
        for row in rows {
            let r = row?;
            let images = self.images_for(r.id)?;
            findings.push(r.into_finding(images));
        }
        Ok(findings)
    }

    /// Count findings grouped by severity.
    pub fn severity_counts(&self) -> Result<Vec<(String, u64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT severity, COUNT(*) FROM findings GROUP BY severity"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
        })?;

        let mut counts = Vec::new();
        for row in rows {
            counts.push(row?);
        }
        Ok(counts)
    }

    /// Update the status of a finding identified by its asset and hex ID.
    /// Returns the finding title on success.
    pub fn update_finding_status(&self, asset: &str, hex_id: &str, status: &str) -> Result<String> {
        let title: String = self.conn.query_row(
            "SELECT title FROM findings WHERE asset = ?1 AND hex_id = ?2",
            params![asset, hex_id],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "UPDATE findings SET status = ?1 WHERE asset = ?2 AND hex_id = ?3",
            params![status, asset, hex_id],
        )?;
        Ok(title)
    }

    /// Get the hex_id for a finding by its slug.
    pub fn hex_id_for_slug(&self, slug: &str) -> Result<String> {
        let hex_id: String = self.conn.query_row(
            "SELECT hex_id FROM findings WHERE slug = ?1",
            params![slug],
            |row| row.get(0),
        )?;
        Ok(hex_id)
    }

    // ------------------------------------------------------------------
    // Asset operations
    // ------------------------------------------------------------------

    /// Insert or update an asset by name. Returns the row id.
    pub fn upsert_asset(&self, asset: &Asset) -> Result<i64> {
        let existing: Option<i64> = self.conn
            .query_row(
                "SELECT id FROM assets WHERE name = ?1",
                params![asset.name],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            self.conn.execute(
                "UPDATE assets SET description = ?1, contact = ?2, criticality = ?3, dns_or_ip = ?4 WHERE id = ?5",
                params![asset.description, asset.contact, asset.criticality, asset.dns_or_ip, id],
            )?;
            Ok(id)
        } else {
            self.conn.execute(
                "INSERT INTO assets (name, description, contact, criticality, dns_or_ip) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![asset.name, asset.description, asset.contact, asset.criticality, asset.dns_or_ip],
            )?;
            Ok(self.conn.last_insert_rowid())
        }
    }

    /// Load all assets from the database.
    pub fn all_assets(&self) -> Result<Vec<Asset>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, contact, criticality, dns_or_ip FROM assets ORDER BY name"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Asset {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                description: row.get(2)?,
                contact: row.get(3)?,
                criticality: row.get(4)?,
                dns_or_ip: row.get(5)?,
            })
        })?;

        let mut assets = Vec::new();
        for row in rows {
            assets.push(row?);
        }
        Ok(assets)
    }

    // ------------------------------------------------------------------
    // Destructive operations
    // ------------------------------------------------------------------

    /// Delete all findings, images, and assets from the database.
    pub fn clean(&self) -> Result<u64> {
        self.conn.execute("DELETE FROM finding_images", [])?;
        self.conn.execute("DELETE FROM assets", [])?;
        let deleted = self.conn.execute("DELETE FROM findings", [])?;
        Ok(deleted as u64)
    }

    // ------------------------------------------------------------------
    // Export
    // ------------------------------------------------------------------

    /// Export findings as CSV rows, optionally filtered by asset and/or date range.
    ///
    /// Returns the full CSV content as a `String` (header + rows).
    pub fn export_csv(
        &self,
        asset: Option<&str>,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<String> {
        let findings = self.findings_filtered(asset, from, to)?;

        let mut out = String::from("hex_id,title,severity,asset,date,location,status,description\n");
        for f in &findings {
            let fields = [
                &f.hex_id, &f.title, f.severity.as_str(), &f.asset,
                &f.date, &f.location, f.status.as_str(), &f.description,
            ];
            let line: Vec<String> = fields.iter().map(|v| csv_field(v)).collect();
            out.push_str(&line.join(","));
            out.push('\n');
        }
        Ok(out)
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn images_for(&self, finding_id: i64) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT path FROM finding_images WHERE finding_id = ?1 ORDER BY id"
        )?;
        let rows = stmt.query_map(params![finding_id], |row| row.get(0))?;
        let mut images = Vec::new();
        for row in rows {
            images.push(row?);
        }
        Ok(images)
    }
}

/// Escape a value for CSV: wrap in double quotes if it contains commas,
/// quotes or newlines; double any internal quotes.
fn csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Build a WHERE clause and parameter list from optional filters.
///
/// Returns `("", vec![])` when no filters are set, or
/// `(" WHERE asset = ?1 AND date >= ?2", vec!["web_app", "2026/01"])` etc.
fn build_where_clause(
    asset: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
) -> (String, Vec<String>) {
    let mut clauses: Vec<String> = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(a) = asset {
        values.push(a.to_string());
        clauses.push(format!("asset = ?{}", values.len()));
    }
    if let Some(f) = from {
        values.push(f.to_string());
        clauses.push(format!("date >= ?{}", values.len()));
    }
    if let Some(t) = to {
        values.push(t.to_string());
        clauses.push(format!("date <= ?{}", values.len()));
    }

    let sql = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };
    (sql, values)
}

/// Internal helper for row mapping.
struct FindingRow {
    id: i64,
    hex_id: String,
    title: String,
    severity: String,
    asset: String,
    date: String,
    location: String,
    description: String,
    status: String,
    slug: String,
}

impl FindingRow {
    fn from_row(row: &rusqlite::Row) -> Self {
        Self {
            id: row.get(0).unwrap_or_default(),
            hex_id: row.get(1).unwrap_or_default(),
            title: row.get(2).unwrap_or_default(),
            severity: row.get(3).unwrap_or_default(),
            asset: row.get(4).unwrap_or_default(),
            date: row.get(5).unwrap_or_default(),
            location: row.get(6).unwrap_or_default(),
            description: row.get(7).unwrap_or_default(),
            status: row.get(8).unwrap_or_default(),
            slug: row.get(9).unwrap_or_default(),
        }
    }

    fn into_finding(self, images: Vec<String>) -> Finding {
        let severity: Severity = self.severity.parse().unwrap_or(Severity::Info);
        let status: Status = self.status.parse().unwrap_or(Status::Open);
        Finding {
            id: Some(self.id),
            hex_id: self.hex_id,
            slug: self.slug,
            title: self.title,
            severity,
            asset: self.asset,
            date: self.date,
            location: self.location,
            description: self.description,
            status,
            images,
        }
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use models::{Severity, Status};

    #[test]
    fn test_insert_and_read() {
        let db = Database::open_in_memory().unwrap();
        let f = Finding::new("Test XSS", Severity::High, "web_app", "2026/01/15", "/search", "XSS in search", Status::Open)
            .with_images(vec!["img/proof.png".into()]);

        let id = db.insert_finding(&f, "test-xss", "0x001").unwrap();
        assert!(id > 0);

        let all = db.all_findings().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].title, "Test XSS");
        assert_eq!(all[0].severity, Severity::High);
        assert_eq!(all[0].asset, "web_app");
        assert_eq!(all[0].date, "2026/01/15");
        assert_eq!(all[0].images, vec!["img/proof.png"]);
    }

    #[test]
    fn test_upsert_updates_existing() {
        let db = Database::open_in_memory().unwrap();
        let f1 = Finding::new("SQLi", Severity::Critical, "api_server", "2026/01/15", "/api", "SQL injection", Status::Open);
        let (_, hex1, is_new1) = db.upsert_finding(&f1, "sqli").unwrap();
        assert!(is_new1);
        assert_eq!(hex1, "0x001");

        let f2 = Finding::new("SQLi v2", Severity::Critical, "api_server", "2026/01/16", "/api", "Updated desc", Status::Resolved);
        let (_, hex2, is_new2) = db.upsert_finding(&f2, "sqli").unwrap();
        assert!(!is_new2);
        assert_eq!(hex2, "0x001"); // same ID on update

        let all = db.all_findings().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].title, "SQLi v2");
        assert_eq!(all[0].status, Status::Resolved);
    }

    #[test]
    fn test_next_hex_id_per_asset() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.next_hex_id("web_app").unwrap(), "0x001");

        db.insert_finding(
            &Finding::new("A", Severity::High, "web_app", "2026/01/15", "", "", Status::Open),
            "a", "0x001",
        ).unwrap();
        assert_eq!(db.next_hex_id("web_app").unwrap(), "0x002");
        assert_eq!(db.next_hex_id("api_server").unwrap(), "0x001"); // different asset

        db.insert_finding(
            &Finding::new("B", Severity::Low, "web_app", "2026/01/16", "", "", Status::Open),
            "b", "0x002",
        ).unwrap();
        assert_eq!(db.next_hex_id("web_app").unwrap(), "0x003");
    }

    #[test]
    fn test_severity_counts() {
        let db = Database::open_in_memory().unwrap();
        db.insert_finding(
            &Finding::new("A", Severity::High, "web_app", "2026/01/15", "", "", Status::Open), "a", "0x001"
        ).unwrap();
        db.insert_finding(
            &Finding::new("B", Severity::High, "web_app", "2026/01/16", "", "", Status::Open), "b", "0x002"
        ).unwrap();
        db.insert_finding(
            &Finding::new("C", Severity::Low, "api_server", "2026/01/17", "", "", Status::Open), "c", "0x001"
        ).unwrap();

        let counts = db.severity_counts().unwrap();
        let high_count = counts.iter().find(|(s, _)| s == "High").map(|(_, c)| *c).unwrap_or(0);
        let low_count = counts.iter().find(|(s, _)| s == "Low").map(|(_, c)| *c).unwrap_or(0);
        assert_eq!(high_count, 2);
        assert_eq!(low_count, 1);
    }

    #[test]
    fn test_unique_slug_constraint() {
        let db = Database::open_in_memory().unwrap();
        let f = Finding::new("A", Severity::Info, "web_app", "2026/01/15", "", "", Status::Open);
        db.insert_finding(&f, "same-slug", "0x001").unwrap();
        let res = db.insert_finding(&f, "same-slug", "0x002");
        assert!(res.is_err());
    }
}
