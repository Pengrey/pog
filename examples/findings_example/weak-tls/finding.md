# Weak TLS Configuration

- **Severity:** Low
- **Asset:** Orion Gateway
- **Date:** 2025/09/18
- **Location:** https://api.example.com
- **Status:** Open

## Description

The server still supports TLS 1.0 and TLS 1.1, both of which are
considered deprecated (RFC 8996). Additionally, the following weak cipher
suites were observed:

- `TLS_RSA_WITH_3DES_EDE_CBC_SHA`
- `TLS_RSA_WITH_RC4_128_SHA`

While exploitation requires a privileged network position, downgrade
attacks (e.g., POODLE, BEAST) become feasible.
