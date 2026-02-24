---
title: Exposed .env File
severity: High
asset: Nexus Portal
date: 2025/07/08
location: https://portal.nexus.corp/.env
status: Resolved
---

The `.env` file is served by the web server and contains database
connection strings, API keys, and SMTP credentials in plaintext.
Accessing `https://portal.nexus.corp/.env` returns the full file
contents with a `200 OK` response.

**Impact:** Credential leakage enabling database access, third-party
API abuse, and lateral movement.
