# Prototype Pollution

- **Severity:** High
- **Asset:** Helix Mobile
- **Date:** 2025/08/04
- **Location:** https://mobile-api.helix.corp/api/merge
- **Status:** Open

## Description

The `/api/merge` endpoint performs a recursive deep merge of user-supplied
JSON into server-side objects. Sending `{"__proto__": {"isAdmin": true}}`
pollutes the object prototype, causing all subsequently created objects
to inherit the `isAdmin` property.

**Impact:** Privilege escalation and potential remote code execution
depending on how downstream code consumes the polluted prototype.
