---
title: Subdomain Takeover
severity: Medium
asset: Orion Gateway
date: 2026/02/12
location: https://staging.orion.corp
status: Open
---

The DNS CNAME record for `staging.orion.corp` points to
`orion-staging.herokuapp.com`, which is no longer provisioned. An attacker
can claim this Heroku application name and serve arbitrary content under
the `orion.corp` domain.

Verified by confirming the Heroku 404 "No such app" response and
checking that the CNAME is still active via `dig staging.orion.corp`.

**Impact:** Phishing, cookie theft (if parent domain cookies are scoped
broadly), and reputational damage through content served on a trusted
subdomain.
