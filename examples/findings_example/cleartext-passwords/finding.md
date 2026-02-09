# Cleartext Storage of Passwords

- **Severity:** Critical
- **Asset:** Helix Mobile
- **Date:** 2025/05/19
- **Location:** https://mobile-api.helix.corp/db
- **Status:** Resolved

## Description

User passwords are stored as unsalted SHA-1 hashes. Rainbow-table
look-ups recover approximately 40% of passwords from a sample dump
within minutes using publicly available tables.

**Impact:** Mass credential compromise in the event of a database breach.
