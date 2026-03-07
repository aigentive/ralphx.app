# Operational Runbook — RalphX External MCP Server

## Prerequisites

- Node.js 18 or later
- RalphX Tauri backend running and listening on `http://127.0.0.1:3847`
- Built server assets: `ralphx-plugin/ralphx-external-mcp/build/index.js`

If the build directory is missing, rebuild:
```bash
cd ralphx-plugin/ralphx-external-mcp
npm install
npm run build
```

## Starting the Server

### Localhost only (development)

```bash
node ralphx-plugin/ralphx-external-mcp/build/index.js
```

The server starts on `http://127.0.0.1:3848`. Log output goes to stderr:
```
[ralphx-external-mcp] Server listening on http://127.0.0.1:3848
[ralphx-external-mcp] MCP endpoint: http://127.0.0.1:3848/mcp
```

### Remote bind (production, requires TLS)

```bash
EXTERNAL_MCP_HOST=0.0.0.0 \
EXTERNAL_MCP_PORT=3848 \
EXTERNAL_MCP_TLS_CERT=/etc/ssl/certs/ralphx.crt \
EXTERNAL_MCP_TLS_KEY=/etc/ssl/private/ralphx.key \
node ralphx-plugin/ralphx-external-mcp/build/index.js
```

The server refuses to start if TLS is not configured for non-localhost binds.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `EXTERNAL_MCP_PORT` | `3848` | TCP port to bind |
| `EXTERNAL_MCP_HOST` | `127.0.0.1` | IP address to bind |
| `RALPHX_BACKEND_URL` | `http://127.0.0.1:3847` | URL of the Tauri backend |
| `EXTERNAL_MCP_TLS_CERT` | — | Path to TLS certificate file (PEM format). Required when host is not localhost. |
| `EXTERNAL_MCP_TLS_KEY` | — | Path to TLS private key file (PEM format). Required when host is not localhost. |

## Health Checks

### Liveness: `/health`

Returns 200 if the process is alive. Does not check backend connectivity.

```bash
curl http://localhost:3848/health
# {"status":"ok"}
```

Use this for process-level health monitoring (systemd `ExecStartPost`, Docker `HEALTHCHECK`).

### Readiness: `/ready`

Returns 200 only if the Tauri backend is reachable. Returns 503 if the backend is down.

```bash
curl http://localhost:3848/ready
# {"status":"ready","backend":"reachable"}     — backend up
# {"status":"not_ready","backend":"unreachable"} — backend down (503)
```

Use this to gate traffic routing (load balancer health check, Kubernetes readiness probe).

## Monitoring

### Key Metrics to Watch

| Metric | How to Detect | Alert Threshold |
|--------|---------------|-----------------|
| Auth failures | Look for `recordAuthFailure` log entries or 401 response rate | >10/min from single IP suggests brute force |
| Rate limit hits | `429 Rate limit exceeded` in access logs | Sustained 429s indicate client is over quota |
| Backend unreachable | `/ready` returns 503 | Any 503 on /ready requires immediate investigation |
| TLS errors | Startup fatal with `TlsError` | Cert expiry, file permission changes |
| Session leaks | `activeTransports` map growing unboundedly | Normal sessions close on disconnect |

### Log Format

All server logs go to stderr in the format:
```
[ralphx-external-mcp] <message>
```

Key log events:
- `Server listening on <url>` — successful startup
- `MCP endpoint: <url>` — MCP is ready
- `Session initialized: <uuid>` — new MCP client session
- `Session closed: <uuid>` — client disconnected
- `Transport error: <err>` — protocol-level error (investigate if frequent)
- `Unhandled request error: <err>` — unexpected error in request handler

## Key Management

API keys are managed through the RalphX backend API. The external MCP server has no key management endpoints of its own.

### Create a key

```bash
curl -X POST http://127.0.0.1:3847/api/auth/keys \
  -H "Content-Type: application/json" \
  -d '{
    "project_ids": ["proj-abc123"],
    "permissions": 3,
    "label": "reefbot-production"
  }'
# Returns: {"key_id":"...","raw_key":"rxk_live_..."}
# The raw_key is shown only once — store it immediately.
```

### Rotate a key

```bash
curl -X POST http://127.0.0.1:3847/api/auth/keys/<key_id>/rotate
# Returns new raw_key. Old key valid for 60s grace period.
```

During the 60-second grace period, update all consumers to use the new key. After 60 seconds, the old key is invalidated and will return 401.

### Revoke a key

```bash
curl -X DELETE http://127.0.0.1:3847/api/auth/keys/<key_id>
```

Revocation takes effect immediately. The MCP server's 30-second auth cache is invalidated via an event from the backend, so revoked keys stop working within milliseconds.

### List keys

```bash
curl http://127.0.0.1:3847/api/auth/keys
```

Returns key metadata (key_id, label, project_ids, permissions, created_at). Raw keys are never returned after creation.

## Troubleshooting

### Backend unreachable at startup

**Symptom:** `/ready` returns 503 immediately after start.

**Cause:** The Tauri backend is not running or not listening on the expected URL.

**Fix:**
1. Confirm the Tauri app is running.
2. Verify the backend URL: `curl http://127.0.0.1:3847/health` should return 200.
3. If using a non-default backend port, set `RALPHX_BACKEND_URL` accordingly.

### Server refuses to start: TLS required

**Symptom:** Fatal error at startup: `TLS is required when binding to non-localhost address '0.0.0.0'`.

**Cause:** `EXTERNAL_MCP_HOST` is set to a non-localhost address without TLS configuration.

**Fix:** Set `EXTERNAL_MCP_TLS_CERT` and `EXTERNAL_MCP_TLS_KEY` to valid certificate/key file paths, or change `EXTERNAL_MCP_HOST` to `127.0.0.1`.

### TLS cert file not readable

**Symptom:** Fatal error: `TLS cert file not readable: /path/to/cert`.

**Cause:** File does not exist or process lacks read permission.

**Fix:** Verify the file path and that the user running the server has read access to both cert and key files.

### Client receives 401 after key rotation

**Symptom:** Client starts getting 401 after calling the rotate endpoint.

**Cause:** Client is still using the old key after the 60-second grace period expired.

**Fix:** Ensure the client updated to the new `raw_key` returned by the rotate endpoint. If the 60-second window was missed, issue a new key and revoke the old one.

### Client receives 429 "Too many authentication failures"

**Symptom:** Client is locked out for 30 seconds after repeated auth failures.

**Cause:** 5 consecutive authentication failures from the client's IP address.

**Fix:** Wait 30 seconds for the lockout to expire. Ensure the correct key is configured on the client. If using a reverse proxy, note that all clients share the proxy's IP for throttle purposes.

### Client receives 429 "Rate limit exceeded"

**Symptom:** Client gets 429 on every request even with a valid key.

**Cause:** The client is exceeding 10 requests per second for the API key.

**Fix:** Reduce request frequency or implement backoff when receiving 429. The rate limit resets automatically — slow down to below 10 req/s to recover the token bucket.

### MCP sessions not closing

**Symptom:** `activeTransports` map grows unboundedly (visible in memory usage).

**Cause:** Clients disconnect without sending a proper MCP session close.

**Fix:** This is expected behavior for abrupt disconnects. Sessions are removed on `onsessionclosed` events. If using the server behind a proxy, ensure the proxy forwards connection close events properly. A server restart clears all sessions.
