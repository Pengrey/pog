---
title: Insecure WebSocket Origin
severity: Medium
asset: Helix Mobile
date: 2025/09/26
location: https://mobile-api.helix.corp/ws
status: Open
---

The WebSocket endpoint at `/ws` accepts upgrade requests from any
`Origin` header. A malicious page can open a WebSocket connection to
the authenticated endpoint and read real-time notifications, chat
messages, and presence data.

**Impact:** Cross-origin data leakage of real-time user activity.
