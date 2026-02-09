# IDOR on User Profiles

- **Severity:** High
- **Asset:** Helix Mobile
- **Date:** 2025/12/08
- **Location:** https://mobile-api.example.com/v2/users/1337/profile
- **Status:** Open

## Description

Authenticated users can access any other user's profile by changing the
numeric user ID in the URL path. The server does not verify that the
requesting user owns the targeted resource.

Tested by authenticating as user `42` and requesting
`/v2/users/1337/profile` — the full profile (name, email, phone, address)
was returned with a `200 OK`.
