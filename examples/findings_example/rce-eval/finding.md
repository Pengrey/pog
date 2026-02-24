---
title: Remote Code Execution via eval()
severity: Critical
asset: Orion Gateway
date: 2025/12/03
location: https://gw.orion.corp/eval
status: Open
---

The `/eval` endpoint accepts a `expression` parameter that is passed
directly into a server-side `eval()` call without sanitisation. Submitting
`expression=require('child_process').execSync('id').toString()` returns the
output of the `id` command, confirming arbitrary command execution.

The endpoint appears to be a debugging utility left enabled in production.

**Impact:** Full remote code execution on the application server with the
privileges of the Node.js process (running as `www-data`).
