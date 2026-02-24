---
title: Clickjacking on Dashboard
severity: Medium
asset: Helix Mobile
date: 2025/10/27
location: https://mobile-api.helix.corp/dashboard
status: Open
---

The dashboard page does not set `X-Frame-Options` or a
`Content-Security-Policy` `frame-ancestors` directive, allowing the page
to be embedded in an attacker-controlled `<iframe>`.

A proof-of-concept page was created that overlays transparent buttons on
the framed dashboard, tricking users into performing actions such as
changing notification settings or initiating a data export.

**Impact:** Social-engineering users into unintended actions within their
authenticated session.
