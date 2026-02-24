---
title: Insecure CORS Policy
severity: Medium
asset: Helix Mobile
date: 2025/11/22
location: https://mobile-api.helix.corp/api/data
status: Open
---

The API responds with `Access-Control-Allow-Origin: *` and
`Access-Control-Allow-Credentials: true` on authenticated endpoints.
This combination allows any third-party website to make credentialed
cross-origin requests and read the response.

A proof-of-concept page hosted on an external domain was able to fetch
the authenticated user's profile data and API keys via a simple
`fetch()` call with `credentials: 'include'`.

**Impact:** Sensitive data exfiltration from any authenticated user who
visits an attacker-controlled page.
