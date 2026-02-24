---
title: JWT Signed with Weak Secret
severity: Critical
asset: Helix Mobile
date: 2026/01/04
location: https://mobile-api.helix.corp/auth
status: Open
---

The application uses HS256-signed JWTs for session management. The
signing secret was found to be `secret123` using `hashcat` with the
`rockyou.txt` wordlist in under 10 seconds.

With knowledge of the secret, an attacker can forge tokens for any user
by setting the `sub` claim to an arbitrary user ID and re-signing the
token.

**Impact:** Complete authentication bypass; any user account can be
impersonated, including administrators.
