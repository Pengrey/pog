# Exposed Admin Panel

- **Severity:** High
- **Asset:** Nexus Portal
- **Date:** 2025/03/05
- **Location:** https://portal.nexus.corp/admin
- **Status:** Resolved

## Description

The admin panel is reachable on the public IP without requiring a VPN
connection. While password-protected, the login form itself should not
be exposed to the public internet as it enables targeted brute-force
attacks and credential-stuffing campaigns.

**Impact:** Increased attack surface; combined with weak credentials
this could lead to full administrative access.
