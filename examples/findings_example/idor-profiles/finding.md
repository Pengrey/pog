---
title: IDOR on User Profiles
severity: High
asset: Helix Mobile
date: 2025/12/08
location: https://mobile-api.example.com/v2/users/1337/profile
status: Open
---

Authenticated users can access any other user's profile by changing the
numeric user ID in the URL path. The server does not verify that the
requesting user owns the targeted resource.

Tested by authenticating as user `42` and requesting
`/v2/users/1337/profile` — the full profile (name, email, phone, address)
was returned with a `200 OK`.
