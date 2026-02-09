# Command Injection in Ping Utility

- **Severity:** Critical
- **Asset:** Orion Gateway
- **Date:** 2025/06/12
- **Location:** https://gw.orion.corp/api/ping
- **Status:** Resolved

## Description

The `host` parameter is passed directly to a shell `exec("ping -c 1 " + host)`
call without any sanitization. Injecting `; id` appends the output of
the `id` command to the ping response.

**Impact:** Full remote command execution as the web server process user.
