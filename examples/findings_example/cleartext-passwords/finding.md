---
title: Cleartext Storage of Passwords
severity: Critical
asset: Helix Mobile
date: 2025/05/19
location: https://mobile-api.helix.corp/db
status: Resolved
---

User passwords are stored as unsalted SHA-1 hashes. Rainbow-table
look-ups recover approximately 40% of passwords from a sample dump
within minutes using publicly available tables.

**Impact:** Mass credential compromise in the event of a database breach.
