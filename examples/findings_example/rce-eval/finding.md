# Remote Code Execution via eval()

- **Severity:** Critical
- **Asset:** Orion Gateway
- **Date:** 2025/12/03
- **Location:** https://gw.orion.corp/eval
- **Status:** Open

## Description

The `/eval` endpoint accepts a `expression` parameter that is passed
directly into a server-side `eval()` call without sanitisation. Submitting
`expression=require('child_process').execSync('id').toString()` returns the
output of the `id` command, confirming arbitrary command execution.

The endpoint appears to be a debugging utility left enabled in production.

**Impact:** Full remote code execution on the application server with the
privileges of the Node.js process (running as `www-data`).
