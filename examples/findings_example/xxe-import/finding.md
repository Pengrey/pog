---
title: XML External Entity Injection
severity: High
asset: Orion Gateway
date: 2025/10/09
location: https://gw.orion.corp/import
status: Open
---

The XML import endpoint parses uploaded XML documents using a default
parser configuration that resolves external entities. Submitting a
payload containing `<!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]>`
successfully exfiltrates the contents of `/etc/passwd` in the server response.

**Impact:** Arbitrary file read on the server, potential for SSRF via
`http://` entity URIs, and denial-of-service via recursive entity
expansion (Billion Laughs).
