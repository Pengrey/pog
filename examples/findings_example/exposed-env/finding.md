# Exposed .env File

- **Severity:** High
- **Asset:** Nexus Portal
- **Date:** 2025/07/08
- **Location:** https://portal.nexus.corp/.env
- **Status:** Resolved

## Description

The `.env` file is served by the web server and contains database
connection strings, API keys, and SMTP credentials in plaintext.
Accessing `https://portal.nexus.corp/.env` returns the full file
contents with a `200 OK` response.

**Impact:** Credential leakage enabling database access, third-party
API abuse, and lateral movement.
