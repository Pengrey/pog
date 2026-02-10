# Internal API SSRF

- **Severity:** High
- **Asset:** Helix Mobile
- **Date:** 2025/11/05
- **Location:** https://mobile-api.helix.corp/api/fetch
- **Status:** False Positive

## Description

The scanner reported a Server-Side Request Forgery (SSRF) vulnerability in
the `/api/fetch` endpoint, claiming that user-supplied URLs are fetched
without restriction. Manual review shows the endpoint only accepts URLs
from a strict allowlist of approved external services, validated both at
the application layer and via egress firewall rules.

The payload used by the scanner (`http://169.254.169.254/latest/meta-data`)
returned a `403 Forbidden` and never reached the metadata service. This is
a false positive.
