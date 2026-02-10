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
    pub description: String,
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
        description: impl Into<String>,
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
            description: description.into(),
            status,
            images: Vec::new(),
        }
    }

    /// Convenience builder to attach image paths.
    pub fn with_images(mut self, images: Vec<String>) -> Self {
        self.images = images;
        self
    }

    /// Sample findings for demonstration / testing purposes.
    pub fn sample_findings() -> Vec<Finding> {
        vec![
            // ── Mar 2025 ──
            Finding::new("Exposed Admin Panel", Severity::High, "nexus_portal", "2025/03/05", "https://portal.nexus.corp/admin", "Admin panel accessible on public IP without VPN.", Status::Resolved),
            Finding::new("Missing Rate Limiting", Severity::Medium, "nexus_portal", "2025/03/18", "https://portal.nexus.corp/api/login", "No rate limiting on login endpoint enables brute-force.", Status::Resolved),

            // ── Apr 2025 ──
            Finding::new("Default Credentials", Severity::Critical, "orion_gateway", "2025/04/02", "https://gw.orion.corp/admin", "Factory default admin:admin credentials still active.", Status::Resolved),
            Finding::new("Unpatched Log4j", Severity::Critical, "orion_gateway", "2025/04/14", "https://gw.orion.corp/api/log", "Log4Shell (CVE-2021-44228) exploitable via User-Agent header.", Status::Resolved),
            Finding::new("Missing HSTS Header", Severity::Info, "nexus_portal", "2025/04/22", "https://portal.nexus.corp", "Strict-Transport-Security header not set.", Status::Resolved),

            // ── May 2025 ──
            Finding::new("Session Fixation", Severity::High, "helix_mobile", "2025/05/07", "https://mobile-api.helix.corp/login", "Session token not regenerated after authentication.", Status::Resolved),
            Finding::new("Cleartext Storage of Password", Severity::Critical, "helix_mobile", "2025/05/19", "https://mobile-api.helix.corp/db", "User passwords stored as plain SHA-1 hashes without salt.", Status::Resolved),

            // ── Jun 2025 ──
            Finding::new("GraphQL Introspection Enabled", Severity::Medium, "nexus_portal", "2025/06/03", "https://portal.nexus.corp/graphql", "Full schema introspection available in production.", Status::Open),
            Finding::new("Command Injection", Severity::Critical, "orion_gateway", "2025/06/12", "https://gw.orion.corp/api/ping", "Host parameter passed to shell exec without sanitization.", Status::Resolved),
            Finding::new("Insecure Randomness", Severity::Medium, "helix_mobile", "2025/06/25", "https://mobile-api.helix.corp/api/token", "Password reset tokens generated with Math.random().", Status::Resolved),

            // ── Jul 2025 ──
            Finding::new("Exposed .env File", Severity::High, "nexus_portal", "2025/07/08", "https://portal.nexus.corp/.env", "Environment file with database credentials publicly accessible.", Status::Resolved),
            Finding::new("HTTP Request Smuggling", Severity::High, "orion_gateway", "2025/07/16", "https://gw.orion.corp", "CL.TE desync between load balancer and backend.", Status::InProgress),
            Finding::new("Verbose Server Header", Severity::Info, "orion_gateway", "2025/07/28", "https://gw.orion.corp", "Server header discloses Apache/2.4.49 version.", Status::Open),

            // ── Aug 2025 ──
            Finding::new("Prototype Pollution", Severity::High, "helix_mobile", "2025/08/04", "https://mobile-api.helix.corp/api/merge", "Deep merge of user input enables __proto__ pollution.", Status::Open),
            Finding::new("Open S3 Bucket", Severity::Critical, "nexus_portal", "2025/08/15", "https://s3.amazonaws.com/nexus-uploads", "Public read/write access on uploads bucket.", Status::Resolved),
            Finding::new("Weak Session Timeout", Severity::Low, "helix_mobile", "2025/08/22", "https://mobile-api.helix.corp", "Session tokens valid for 30 days without re-auth.", Status::Open),

            // ── Sep 2025 ──
            Finding::new("Open Redirect", Severity::Medium, "nexus_portal", "2025/09/03", "https://portal.nexus.corp/goto", "Unvalidated redirect via query parameter.", Status::Resolved),
            Finding::new("Verbose Error Messages", Severity::Info, "nexus_portal", "2025/09/10", "https://portal.nexus.corp/api/debug", "Stack traces exposed in production error responses.", Status::Open),
            Finding::new("TLS Certificate Mismatch", Severity::Low, "orion_gateway", "2025/09/18", "https://gw.orion.corp", "Certificate CN does not match hostname.", Status::FalsePositive),
            Finding::new("Insecure WebSocket", Severity::Medium, "helix_mobile", "2025/09/26", "https://mobile-api.helix.corp/ws", "WebSocket endpoint accepts connections without origin check.", Status::Open),

            // ── Oct 2025 ──
            Finding::new("SQL Injection", Severity::Critical, "nexus_portal", "2025/10/02", "https://portal.nexus.corp/api/users?id=1", "User input is directly concatenated into SQL query without sanitization.", Status::Open),
            Finding::new("XML External Entity (XXE)", Severity::High, "orion_gateway", "2025/10/09", "https://gw.orion.corp/import", "XML parser resolves external entities from untrusted input.", Status::Open),
            Finding::new("Cross-Site Scripting (XSS)", Severity::High, "nexus_portal", "2025/10/14", "https://portal.nexus.corp/search", "Reflected XSS vulnerability in search parameter.", Status::InProgress),
            Finding::new("Directory Listing Enabled", Severity::Low, "nexus_portal", "2025/10/20", "https://portal.nexus.corp/static/", "Web server exposes directory listing for static assets.", Status::Resolved),
            Finding::new("Clickjacking", Severity::Medium, "helix_mobile", "2025/10/27", "https://mobile-api.helix.corp/dashboard", "Missing X-Frame-Options header on sensitive page.", Status::Open),

            // ── Nov 2025 ──
            Finding::new("Buffer Overflow", Severity::Critical, "orion_gateway", "2025/11/01", "https://gw.orion.corp/upload", "Stack buffer overflow in file upload handler.", Status::Open),
            Finding::new("Authentication Bypass", Severity::Critical, "nexus_portal", "2025/11/06", "https://portal.nexus.corp/admin", "Admin panel accessible without authentication.", Status::Resolved),
            Finding::new("Privilege Escalation", Severity::High, "orion_gateway", "2025/11/12", "https://gw.orion.corp/api/role", "Users can modify their own role parameter.", Status::InProgress),
            Finding::new("Information Disclosure", Severity::Medium, "nexus_portal", "2025/11/17", "https://portal.nexus.corp/.git", "Git repository exposed to public.", Status::Open),
            Finding::new("Insecure CORS Policy", Severity::Medium, "helix_mobile", "2025/11/22", "https://mobile-api.helix.corp/api/data", "Access-Control-Allow-Origin set to wildcard.", Status::Open),
            Finding::new("Missing Security Headers", Severity::Info, "orion_gateway", "2025/11/28", "https://gw.orion.corp/health", "Response missing Content-Security-Policy header.", Status::Open),

            // ── Dec 2025 ──
            Finding::new("Remote Code Execution", Severity::Critical, "orion_gateway", "2025/12/03", "https://gw.orion.corp/eval", "User input passed to eval() function.", Status::Open),
            Finding::new("Insecure Deserialization", Severity::High, "helix_mobile", "2025/12/08", "https://mobile-api.helix.corp/api/session", "Untrusted data deserialized without validation.", Status::Open),
            Finding::new("Server-Side Request Forgery", Severity::High, "nexus_portal", "2025/12/14", "https://portal.nexus.corp/proxy", "User-supplied URL fetched without allowlist validation.", Status::InProgress),
            Finding::new("Path Traversal", Severity::Medium, "nexus_portal", "2025/12/19", "https://portal.nexus.corp/files", "File path parameter allows directory traversal.", Status::Open),
            Finding::new("Denial of Service", Severity::Medium, "orion_gateway", "2025/12/22", "https://gw.orion.corp/api/export", "No rate limiting on resource-intensive endpoint.", Status::FalsePositive),
            Finding::new("Cookie Without Secure Flag", Severity::Low, "helix_mobile", "2025/12/28", "https://mobile-api.helix.corp", "Session cookie transmitted over unencrypted channel.", Status::Open),

            // ── Jan 2026 ──
            Finding::new("JWT Secret Key Weak", Severity::Critical, "helix_mobile", "2026/01/04", "https://mobile-api.helix.corp/auth", "JWT signed with easily guessable secret key.", Status::Open),
            Finding::new("Hardcoded Credentials", Severity::High, "orion_gateway", "2026/01/10", "https://gw.orion.corp/config", "Database password hardcoded in source.", Status::InProgress),
            Finding::new("CSRF Token Missing", Severity::Medium, "nexus_portal", "2026/01/15", "https://portal.nexus.corp/settings", "Form submission lacks CSRF protection.", Status::Open),
            Finding::new("Weak Password Policy", Severity::Low, "helix_mobile", "2026/01/20", "https://mobile-api.helix.corp/register", "No minimum password length requirement.", Status::Resolved),
            Finding::new("HTTP Only Flag Missing", Severity::Info, "nexus_portal", "2026/01/25", "https://portal.nexus.corp", "Session cookie missing HttpOnly flag.", Status::Open),
            Finding::new("Unvalidated File Upload", Severity::High, "nexus_portal", "2026/01/28", "https://portal.nexus.corp/upload", "No file type validation on upload endpoint.", Status::Open),

            // ── Feb 2026 ──
            Finding::new("Mass Assignment", Severity::High, "helix_mobile", "2026/02/02", "https://mobile-api.helix.corp/api/profile", "API accepts and persists undocumented fields.", Status::Open),
            Finding::new("Broken Access Control", Severity::Critical, "nexus_portal", "2026/02/07", "https://portal.nexus.corp/api/invoices", "IDOR allows accessing other users' invoices.", Status::Open),
        ]
    }
}
