---
title: Default Credentials on Gateway Admin
severity: Critical
asset: Orion Gateway
date: 2025/04/02
location: https://gw.orion.corp/admin
status: Resolved
---

The gateway management interface still uses factory default credentials
`admin:admin`. Logging in grants full access to routing rules, upstream
configuration, and TLS certificate management.

**Impact:** Complete compromise of the API gateway, including the ability
to intercept, modify, or redirect all traffic.
