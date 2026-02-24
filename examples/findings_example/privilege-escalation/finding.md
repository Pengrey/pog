---
title: Privilege Escalation via Role Parameter
severity: High
asset: Orion Gateway
date: 2025/11/12
location: https://gw.orion.corp/api/role
status: InProgress
---

The `PUT /api/role` endpoint accepts a `role` field in the JSON body and
applies it without verifying that the requesting user has administrative
privileges. A standard user can send `{"role": "admin"}` to elevate their
own account to administrator.

Verified by creating a new test account, issuing the role change request,
and confirming admin-level access to user management and configuration
endpoints.

**Impact:** Complete bypass of role-based access controls; any
authenticated user can become an administrator.
