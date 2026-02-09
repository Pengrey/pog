# JWT Signed with Weak Secret

- **Severity:** Critical
- **Asset:** Helix Mobile
- **Date:** 2026/01/04
- **Location:** https://mobile-api.helix.corp/auth
- **Status:** Open

## Description

The application uses HS256-signed JWTs for session management. The
signing secret was found to be `secret123` using `hashcat` with the
`rockyou.txt` wordlist in under 10 seconds.

With knowledge of the secret, an attacker can forge tokens for any user
by setting the `sub` claim to an arbitrary user ID and re-signing the
token.

**Impact:** Complete authentication bypass; any user account can be
impersonated, including administrators.
