---
title: Prototype Pollution
severity: High
asset: Helix Mobile
date: 2025/08/04
location: https://mobile-api.helix.corp/api/merge
status: Open
---

The `/api/merge` endpoint performs a recursive deep merge of user-supplied
JSON into server-side objects. Sending `{"__proto__": {"isAdmin": true}}`
pollutes the object prototype, causing all subsequently created objects
to inherit the `isAdmin` property.

**Impact:** Privilege escalation and potential remote code execution
depending on how downstream code consumes the polluted prototype.
