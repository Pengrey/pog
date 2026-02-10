# TLS Certificate Mismatch

- **Severity:** Low
- **Asset:** Orion Gateway
- **Date:** 2025/09/18
- **Location:** https://gw.orion.corp
- **Status:** False Positive

## Description

The automated scanner flagged a TLS certificate mismatch between the
Common Name (CN) in the certificate and the hostname being tested. Upon
manual investigation, the certificate is a wildcard certificate
(`*.orion.corp`) that correctly covers the `gw.orion.corp` subdomain.

The scanner failed to validate the wildcard match and reported a false
positive. The TLS configuration is correct and the certificate is valid.
