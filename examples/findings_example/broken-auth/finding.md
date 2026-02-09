# Broken Authentication on Admin Panel

- **Severity:** Critical
- **Asset:** Orion Gateway
- **Date:** 2025/11/06
- **Location:** https://api.example.com/admin
- **Status:** InProgress

## Description

The `/admin` endpoint relies solely on a predictable session token
generated from `md5(username + timestamp)`. No rate-limiting or account
lockout is enforced, allowing brute-force attacks against the token space.

Using a wordlist of common usernames and a 24-hour timestamp window, a
valid admin session was obtained in under 5 minutes.

**Impact:** Full administrative access to the API backend, including user
management, configuration changes and data export.
