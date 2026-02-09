# Verbose Error Messages

- **Severity:** Info
- **Asset:** Nexus Portal
- **Date:** 2025/09/10
- **Location:** https://example.com/api/search?q=%27
- **Status:** Open

## Description

Sending a single quote `'` in the `q` parameter causes the application to
return a full stack trace including internal file paths, framework version
and database engine details:

```
PG::SyntaxError: ERROR: unterminated quoted string at or near "'"
LINE 1: SELECT * FROM products WHERE name LIKE '%'%'
/app/vendor/bundle/ruby/3.1.0/gems/activerecord-7.0.4/lib/...
```

While not directly exploitable, this information aids further attacks
(e.g., confirming PostgreSQL for SQL injection payloads).
