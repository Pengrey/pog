# Default Credentials on Gateway Admin

- **Severity:** Critical
- **Asset:** Orion Gateway
- **Date:** 2025/04/02
- **Location:** https://gw.orion.corp/admin
- **Status:** Resolved

## Description

The gateway management interface still uses factory default credentials
`admin:admin`. Logging in grants full access to routing rules, upstream
configuration, and TLS certificate management.

**Impact:** Complete compromise of the API gateway, including the ability
to intercept, modify, or redirect all traffic.
