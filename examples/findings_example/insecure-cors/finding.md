# Insecure CORS Policy

- **Severity:** Medium
- **Asset:** Helix Mobile
- **Date:** 2025/11/22
- **Location:** https://mobile-api.helix.corp/api/data
- **Status:** Open

## Description

The API responds with `Access-Control-Allow-Origin: *` and
`Access-Control-Allow-Credentials: true` on authenticated endpoints.
This combination allows any third-party website to make credentialed
cross-origin requests and read the response.

A proof-of-concept page hosted on an external domain was able to fetch
the authenticated user's profile data and API keys via a simple
`fetch()` call with `credentials: 'include'`.

**Impact:** Sensitive data exfiltration from any authenticated user who
visits an attacker-controlled page.
