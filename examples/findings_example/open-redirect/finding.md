# Open Redirect

- **Severity:** Medium
- **Asset:** Nexus Portal
- **Date:** 2025/09/03
- **Location:** https://example.com/login?next=https://evil.com
- **Status:** Open

## Description

The `next` query parameter on the login page is used in a `302` redirect
after successful authentication without validating that the target URL
belongs to the application's own domain.

An attacker can craft a phishing link that first sends the victim through
the legitimate login page, then redirects them to a credential-harvesting
site.
