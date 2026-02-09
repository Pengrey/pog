# Insecure WebSocket Origin

- **Severity:** Medium
- **Asset:** Helix Mobile
- **Date:** 2025/09/26
- **Location:** https://mobile-api.helix.corp/ws
- **Status:** Open

## Description

The WebSocket endpoint at `/ws` accepts upgrade requests from any
`Origin` header. A malicious page can open a WebSocket connection to
the authenticated endpoint and read real-time notifications, chat
messages, and presence data.

**Impact:** Cross-origin data leakage of real-time user activity.
