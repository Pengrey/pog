---
title: Session Fixation
severity: High
asset: Helix Mobile
date: 2025/05/07
location: https://mobile-api.helix.corp/login
status: Resolved
---

The session token issued before authentication is reused after a
successful login without regeneration. An attacker who knows or sets
the pre-auth token (e.g., via a link) can hijack the victim's
authenticated session.

**Impact:** Full session hijack; attacker gains the victim's
authenticated privileges.
