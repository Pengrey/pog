# SQL Injection

- **Severity:** Critical
- **Asset:** Nexus Portal
- **Date:** 2025/10/02
- **Location:** https://example.com/api/users?id=1
- **Status:** Open

## Description

The `id` parameter in the `/api/users` endpoint is directly concatenated
into a raw SQL query without any parameterisation or input sanitisation.

Injecting `1 OR 1=1--` returns the full user table. Further exploitation
confirmed the ability to `UNION SELECT` from `information_schema.tables`,
exposing the entire database schema.

**Impact:** Full read access to the database; potential for data
exfiltration, privilege escalation or destructive operations.
