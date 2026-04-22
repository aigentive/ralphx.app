# Security Model — RalphX External MCP

## Overview

The external MCP server uses a dual-layer security model to protect the RalphX orchestration API. No unauthenticated or out-of-scope request reaches the business logic layer.

```
Internet / External Network
          |
    [Layer 1: MCP middleware — :3848]
    - IP auth throttle (5 failures → 30s lockout)
    - Bearer token format validation
    - Key validation against :3847 (30s TTL cache)
    - Per-key rate limiting (token bucket, 10 req/s)
    - Project scope header injection
          |
    [Layer 2: Axum backend — :3847, localhost-only]
    - ProjectScopeGuard: enforces X-RalphX-Project-Scope header
    - Returns 403 if requested resource is outside scope
    - Cannot be bypassed by external traffic (localhost-only bind)
```

## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| `:3848` | External surface — accepts connections from the network. All security enforcement happens here before proxying to backend. |
| `:3847` | Internal surface — bound to `127.0.0.1` only. Unreachable from the network. |
| Localhost processes | Processes running on the same machine as RalphX can call `:3847` directly, bypassing Layer 1. This is an accepted risk: the machine itself is already trusted (it runs the Tauri app). External agents cannot exploit this. |

## Tauri-Owned Local Bypass

RalphX can route native project-chat agents through the external MCP process without issuing a user API key. Tauri generates `RALPHX_TAURI_MCP_BYPASS_TOKEN`, passes it to the external MCP supervisor and Codex child process, and Codex sends it as the MCP Bearer token. The external MCP server accepts that token only from loopback clients and marks proxied backend calls with `X-RalphX-Tauri-MCP: 1`.

This bypass is local-only and intended for app-owned agents. Network clients still need normal `rxk_live_` API keys.

## API Key Format and Storage

API keys use the format: `rxk_live_{32 random alphanumeric characters}`

- Keys are generated with cryptographically secure randomness.
- The raw key is shown to the user once at creation time and never stored.
- The backend stores only the SHA-256 hash of the raw key in the `api_keys` table.
- Validation at `:3847/api/auth/validate-key` compares the SHA-256 hash of the presented key.

## Authentication Flow

```
Client → Authorization: Bearer rxk_live_<key>
         |
         1. Extract token from Authorization header
         2. Validate rxk_live_ prefix (reject 401 if missing)
         3. Check 30s TTL cache (return cached ApiKeyContext if fresh)
         4. GET :3847/api/auth/validate-key with Bearer token
            → 401/403 from backend: remove stale cache, return 401
            → 200: cache ApiKeyContext for 30s
         5. Return ApiKeyContext {keyId, projectIds, permissions}
```

The auth cache reduces load on the backend validation endpoint. Cache entries are invalidated immediately when key rotation events are received, ensuring revoked keys stop working within milliseconds (not 30 seconds).

## Key Lifecycle

```
Creation
  └─ raw key displayed once → SHA-256 hash stored
        |
        v
   Active
  └─ validated on each request (cached 30s)
        |
        v
   Rotation (60s grace period)
  └─ old key valid for 60 seconds after new key issued
  └─ cache invalidated immediately on rotation event
  └─ use this window to update all consumers
        |
        v
   Revocation
  └─ backend returns 401
  └─ cache entry deleted immediately
  └─ all subsequent requests fail
```

## Permission Bitmask

Permissions are stored as a bitmask integer on each API key. The MCP server and backend both enforce permission checks.

| Value | Constant | Capabilities |
|-------|----------|-------------|
| 1 | `READ` | List projects, get status, get task details, get review summaries, poll events, get attention items, get capacity |
| 2 | `WRITE` | All READ capabilities, plus: start ideation, send ideation messages, modify proposals, accept plans, pause/cancel/retry tasks, request changes |
| 4 | `ADMIN` | All WRITE capabilities, plus: approve reviews, resolve escalations, manage API keys |

Permissions combine: value `3` = READ + WRITE, value `7` = READ + WRITE + ADMIN.

## IP Auth Throttle

The IP throttle prevents brute-force attacks on the key validation endpoint.

- Source IP is extracted from the TCP socket's `remoteAddress` (not `X-Forwarded-For`, which can be spoofed).
- **5 consecutive auth failures** from the same IP trigger a **30-second lockout**.
- A successful authentication resets the failure counter for that IP.
- During lockout, the server returns `429 Too Many Requests` before even attempting key validation.

**Note:** If the external MCP server is behind a reverse proxy, all clients will appear to share the same IP (the proxy's IP), which would cause a single client's failures to lock out all clients. Do not use a reverse proxy without configuring the server to trust a specific proxy IP and read `X-Forwarded-For` from it.

## Per-Key Rate Limiting

Each API key has an independent token bucket rate limiter:

- **Bucket capacity:** 10 tokens (maximum burst)
- **Refill rate:** 10 tokens per second
- A request consumes 1 token. If the bucket is empty, the server returns `429 Rate limit exceeded`.
- The bucket refills continuously — a key that makes 0 requests for 1 second regains 10 tokens.

Token bucket behavior under sustained load:
- Steady state: 10 req/s sustained indefinitely
- Initial burst: up to 10 requests in the first moment (full bucket)
- After exhaustion: exactly 1 request processed per 100ms

## TLS Requirements

TLS is mandatory when binding to any address other than `127.0.0.1`, `::1`, or `localhost`. The server refuses to start without TLS in this case, because Bearer tokens must not travel in cleartext on the network.

The TLS check is enforced at startup in `validateTlsConfig()`:
- If host is non-localhost and `tls` config is absent: fatal error, process exits.
- If `cert_path` or `key_path` files are unreadable: fatal error, process exits.
- Localhost binds: TLS is optional (token travels only on loopback).

## Audit Logging

All tool invocations are recorded in the `api_audit_log` table on the backend:

| Column | Description |
|--------|-------------|
| `key_id` | The API key that made the call |
| `tool_name` | The v1_ tool name invoked |
| `project_id` | Project scope (if applicable) |
| `latency_ms` | End-to-end latency for the backend call |
| `success` | Whether the tool call succeeded |
| `created_at` | Timestamp |

Audit logs are retained indefinitely and are queryable via the RalphX admin UI.

## Headers Injected by MCP Server

The MCP server injects two headers on every proxied request to `:3847`:

| Header | Value | Purpose |
|--------|-------|---------|
| `X-RalphX-Project-Scope` | Comma-separated project IDs from key's scope | Backend ProjectScopeGuard uses this to enforce access control |
| `X-RalphX-External-MCP` | `1` | Marks the request as coming from external MCP (for logging and routing) |

The `X-RalphX-Project-Scope` header is the mechanism by which Layer 2 enforcement works. The backend rejects any request that attempts to access resources outside the listed project IDs.
