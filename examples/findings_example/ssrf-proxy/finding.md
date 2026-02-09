# Server-Side Request Forgery

- **Severity:** High
- **Asset:** Nexus Portal
- **Date:** 2025/12/14
- **Location:** https://portal.nexus.corp/proxy
- **Status:** InProgress

## Description

The `/proxy` endpoint fetches a user-supplied URL and returns the response
body. No allowlist or blocklist is enforced, enabling requests to internal
services. Submitting `url=http://169.254.169.254/latest/meta-data/`
returns AWS instance metadata, including IAM role credentials.

Internal port scanning was also demonstrated by iterating over
`http://10.0.0.1:<port>` and observing response time differences.

**Impact:** Access to cloud provider metadata and internal network
services; potential for credential theft and lateral movement.
