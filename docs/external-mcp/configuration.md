# Configuration Reference — RalphX External MCP Server

## Configuration Sources

Configuration is read from two sources, in priority order:

1. **Environment variables** (highest priority — override everything)
2. **ralphx.yaml** `external_mcp` section (base configuration)

## ralphx.yaml Schema

```yaml
external_mcp:
  # TCP port to listen on
  # Default: 3848
  port: 3848

  # IP address to bind
  # Use 127.0.0.1 for localhost-only (development)
  # Use 0.0.0.0 to accept connections from all interfaces (production — requires TLS)
  # Default: "127.0.0.1"
  host: "127.0.0.1"

  # URL of the Tauri backend
  # Default: "http://127.0.0.1:3847"
  backend_url: "http://127.0.0.1:3847"

  # TLS configuration — required when host is not 127.0.0.1, ::1, or localhost
  # Omit entirely for localhost-only operation
  tls:
    # Path to TLS certificate file (PEM format)
    cert_path: "/etc/ssl/certs/ralphx.crt"
    # Path to TLS private key file (PEM format)
    key_path: "/etc/ssl/private/ralphx.key"

  # Rate limiting configuration
  # All values have defaults — omit this section to use defaults
  rate_limit:
    # Max requests per second per API key (token bucket capacity and refill rate)
    # Default: 10
    requests_per_second: 10

    # Max concurrent TCP connections
    # Default: 50
    max_connections: 50

    # Number of consecutive auth failures from one IP before lockout
    # Default: 5
    auth_failures_before_lockout: 5

    # Duration of IP lockout after auth failure threshold
    # Default: 30
    lockout_duration_secs: 30

    # Maximum simultaneous external ideation sessions
    # Prevents resource exhaustion from concurrent orchestrator agents spawned via API
    # Default: 1
    max_external_ideation_sessions: 1
```

## Environment Variable Overrides

Environment variables take precedence over ralphx.yaml values. They are read at startup and cannot be changed without a restart.

| Variable | ralphx.yaml equivalent | Description |
|----------|------------------------|-------------|
| `EXTERNAL_MCP_PORT` | `external_mcp.port` | TCP port to bind |
| `EXTERNAL_MCP_HOST` | `external_mcp.host` | IP address to bind |
| `RALPHX_BACKEND_URL` | `external_mcp.backend_url` | Tauri backend URL |
| `EXTERNAL_MCP_TLS_CERT` | `external_mcp.tls.cert_path` | Path to TLS certificate (PEM) |
| `EXTERNAL_MCP_TLS_KEY` | `external_mcp.tls.key_path` | Path to TLS private key (PEM) |

Rate limiting options are only configurable via ralphx.yaml (not environment variables).

## Configuration Validation Rules

The server validates configuration at startup and refuses to start if validation fails.

| Rule | Error |
|------|-------|
| `host` is non-localhost and `tls` is absent | Fatal: `TLS is required when binding to non-localhost address` |
| `tls.cert_path` file is not readable | Fatal: `TLS cert file not readable: <path>` |
| `tls.key_path` file is not readable | Fatal: `TLS key file not readable: <path>` |
| `tls.cert_path` or `tls.key_path` is empty string | Fatal: `TLS config is incomplete: both cert_path and key_path must be specified` |

Localhost addresses exempt from TLS: `127.0.0.1`, `::1`, `localhost`.

## Example: Minimal localhost configuration

No TLS required. Suitable for development and for integrations where the agent runs on the same machine as RalphX.

```yaml
external_mcp:
  port: 3848
  host: "127.0.0.1"
  backend_url: "http://127.0.0.1:3847"
```

Or equivalently, use environment variables only (no ralphx.yaml section needed):

```bash
node build/index.js
# Uses all defaults: port 3848, host 127.0.0.1, backend http://127.0.0.1:3847
```

## Example: Production configuration (remote, TLS)

For running on a server where external agents connect over the network.

```yaml
external_mcp:
  port: 3848
  host: "0.0.0.0"
  backend_url: "http://127.0.0.1:3847"
  tls:
    cert_path: "/etc/ssl/certs/ralphx-mcp.crt"
    key_path: "/etc/ssl/private/ralphx-mcp.key"
  rate_limit:
    requests_per_second: 10
    max_connections: 50
    auth_failures_before_lockout: 5
    lockout_duration_secs: 30
    max_external_ideation_sessions: 2
```

Or via environment variables:

```bash
EXTERNAL_MCP_HOST=0.0.0.0 \
EXTERNAL_MCP_PORT=3848 \
RALPHX_BACKEND_URL=http://127.0.0.1:3847 \
EXTERNAL_MCP_TLS_CERT=/etc/ssl/certs/ralphx-mcp.crt \
EXTERNAL_MCP_TLS_KEY=/etc/ssl/private/ralphx-mcp.key \
node build/index.js
```

## Example: High-volume configuration

For scenarios with multiple API key holders making frequent calls.

```yaml
external_mcp:
  port: 3848
  host: "0.0.0.0"
  backend_url: "http://127.0.0.1:3847"
  tls:
    cert_path: "/etc/ssl/certs/ralphx-mcp.crt"
    key_path: "/etc/ssl/private/ralphx-mcp.key"
  rate_limit:
    requests_per_second: 20
    max_connections: 100
    auth_failures_before_lockout: 10
    lockout_duration_secs: 60
    max_external_ideation_sessions: 5
```

Note: Increasing `requests_per_second` above 20 may cause elevated load on the Tauri backend. The backend has its own rate limiting and concurrency controls. Monitor backend response times when tuning.

## Auth Cache Behavior

The 30-second TTL auth cache is not configurable. It is hardcoded at 30 seconds to balance security (revoked keys stop working quickly) with performance (reducing validation calls to the backend). Cache entries are invalidated immediately on key rotation/revocation events regardless of TTL.
