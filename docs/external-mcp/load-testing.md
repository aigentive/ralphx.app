# Load Testing — RalphX External MCP Server

## Theoretical Limits

| Limit | Default | Config Key |
|-------|---------|------------|
| Requests per second per API key | 10 | `rate_limit.requests_per_second` |
| Max concurrent connections | 50 | `rate_limit.max_connections` |
| Auth failures before IP lockout | 5 | `rate_limit.auth_failures_before_lockout` |
| IP lockout duration | 30s | `rate_limit.lockout_duration_secs` |
| Auth cache TTL | 30s | hardcoded |
| Backend request timeout | 30s | hardcoded (`BackendClient`) |

For multiple API keys, limits are independent per key. Two keys can each sustain 10 req/s simultaneously (20 req/s total through the server).

## Token Bucket Algorithm

The per-key rate limiter uses a continuous token bucket:

```
bucket.capacity = requests_per_second (default 10)
bucket.tokens   = capacity            (starts full)
bucket.refill   = elapsed_seconds * requests_per_second  (continuous)
```

**Behavior under load:**

| Scenario | Behavior |
|----------|----------|
| Idle key, then 10 rapid requests | All 10 succeed (bucket was full) |
| Sustained 10 req/s | All requests succeed (refill equals consumption) |
| Sustained 11 req/s | ~10% of requests receive 429 |
| Sustained 20 req/s | ~50% of requests receive 429 |
| Burst of 20, then idle | First 10 succeed, next 10 fail; bucket refills in 1s |

The bucket refills fractionally — a key that makes 5 requests in 0.5 seconds has 5 tokens remaining. After 0.5 more seconds of idle, the bucket is full again.

**Recovery from 429:** Stop sending requests for `<tokens_needed> / requests_per_second` seconds. For an empty bucket, wait 1 second to regain full capacity.

## Auth Cache Impact

The 30-second TTL auth cache significantly reduces backend load under sustained traffic from the same key. Without the cache, every request would require a round-trip to `:3847/api/auth/validate-key`.

**Cache hit rate under load:**
- Single key, 10 req/s sustained: ~100% cache hit rate after the first request
- Multiple keys: cache hit rate scales with key reuse frequency
- Cache miss triggers one backend validation call; subsequent requests in the same 30s window are served from cache

This means the Tauri backend sees at most 1 auth validation per 30 seconds per unique key, regardless of request rate. Auth validation load is effectively `(unique_keys_per_30s) / 30` calls per second to the backend.

## Expected Responses at Limits

| Condition | HTTP Status | Response Body |
|-----------|-------------|---------------|
| Rate limit exceeded (token bucket empty) | 429 | `{"error":"Rate limit exceeded — reduce request frequency"}` |
| IP auth lockout active | 429 | `{"error":"Too many authentication failures — try again later"}` |
| Max connections reached | 503 | TCP connection refused (Node.js rejects at socket level) |
| Backend timeout (>30s) | 504 | `{"error":"backend_error","status":504,"message":"Backend request timed out"}` |
| Backend unreachable | 503 | `{"error":"backend_error","status":503,"message":"Backend unreachable"}` |

## Recommended Load Testing Tools

### autocannon (npm)

Simple, fast HTTP benchmarking. Good for quick throughput tests.

```bash
npm install -g autocannon

# Basic throughput test — will be rate-limited to 10 req/s by server
autocannon \
  -c 5 \          # 5 concurrent connections
  -d 30 \         # 30 second duration
  -m POST \
  -H "Authorization: Bearer rxk_live_<your_key>" \
  -H "Content-Type: application/json" \
  -b '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  http://localhost:3848/mcp
```

### k6

More sophisticated — supports scenarios, thresholds, and metrics. Recommended for systematic load testing.

```bash
brew install k6  # macOS
# or: https://k6.io/docs/get-started/installation/
```

## Sample k6 Test Script

Save as `load-test.js` and run with `k6 run load-test.js`.

```javascript
import http from "k6/http";
import { check, sleep } from "k6";
import { Rate, Trend } from "k6/metrics";

// Custom metrics
const rateLimitRate = new Rate("rate_limited_requests");
const toolLatency = new Trend("tool_call_latency_ms");

// Configuration — set these via k6 env vars or edit directly
const BASE_URL = __ENV.MCP_URL || "http://localhost:3848";
const API_KEY = __ENV.API_KEY || "rxk_live_yourkeyhere";
const PROJECT_ID = __ENV.PROJECT_ID || "your-project-id";

export const options = {
  // Ramp up to 10 req/s over 30s, sustain for 60s, ramp down
  stages: [
    { duration: "30s", target: 10 },
    { duration: "60s", target: 10 },
    { duration: "10s", target: 0 },
  ],
  thresholds: {
    // Less than 5% of requests should be rate limited at exactly the rate limit
    rate_limited_requests: ["rate<0.05"],
    // 95th percentile tool call latency under 2s (includes backend round trip)
    tool_call_latency_ms: ["p(95)<2000"],
    // Overall HTTP error rate under 10%
    http_req_failed: ["rate<0.10"],
  },
};

const headers = {
  "Authorization": `Bearer ${API_KEY}`,
  "Content-Type": "application/json",
};

export default function () {
  // Test 1: Health check (no auth — should always succeed)
  const healthRes = http.get(`${BASE_URL}/health`);
  check(healthRes, {
    "health 200": (r) => r.status === 200,
    "health ok": (r) => JSON.parse(r.body).status === "ok",
  });

  // Test 2: MCP tool call — v1_list_projects
  const start = Date.now();
  const toolRes = http.post(
    `${BASE_URL}/mcp`,
    JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "tools/call",
      params: {
        name: "v1_list_projects",
        arguments: {},
      },
    }),
    { headers }
  );

  const latency = Date.now() - start;
  toolLatency.add(latency);

  const isRateLimited = toolRes.status === 429;
  rateLimitRate.add(isRateLimited);

  check(toolRes, {
    "tool call not 5xx": (r) => r.status < 500,
    "tool call authenticated": (r) => r.status !== 401,
  });

  // Test 3: Readiness check
  const readyRes = http.get(`${BASE_URL}/ready`);
  check(readyRes, {
    "ready endpoint responded": (r) => r.status === 200 || r.status === 503,
  });

  // Sleep to stay at target RPS — k6 controls VU concurrency
  sleep(0.1);
}

export function handleSummary(data) {
  return {
    "load-test-results.json": JSON.stringify(data, null, 2),
  };
}
```

Run the test:

```bash
# Basic run
k6 run load-test.js

# With environment variables
MCP_URL=http://localhost:3848 \
API_KEY=rxk_live_yourkeyhere \
PROJECT_ID=proj-abc123 \
k6 run load-test.js

# With HTML report (requires k6 reporter)
k6 run --out json=results.json load-test.js
```

## Key Metrics to Monitor During Load Test

| Metric | What to Watch | Warning Sign |
|--------|--------------|--------------|
| `http_req_duration` (p95) | Should be <2s at 10 req/s | >2s indicates backend bottleneck |
| `rate_limited_requests` rate | Should be ~0% at or below 10 req/s | >5% means client is above limit |
| `http_req_failed` | Auth failures, 5xx errors | Any 5xx warrants investigation |
| Backend `/health` during test | Should always 200 | 503 means backend is overwhelmed |
| Process memory (Node.js) | Should be stable | Growing memory = session leak |

## Tuning Recommendations

**Increase throughput per key:**

Set `rate_limit.requests_per_second` in ralphx.yaml. Values above 20 risk overloading the Tauri backend — monitor backend response times and SQLite query latency before increasing further.

**Handle more concurrent clients:**

Increase `rate_limit.max_connections`. The Node.js HTTP server handles connections efficiently, but the constraint is usually the Tauri backend's SQLite throughput. Each concurrent tool call becomes a concurrent backend request.

**Reduce backend load:**

The 30s auth cache handles this automatically for sustained single-key traffic. For many unique keys making infrequent requests, cache miss rate will be higher. No configuration knob exists for this — the 30s TTL is a fixed security/performance tradeoff.

**Expected throughput ceiling:**

At default settings with a healthy backend:
- Single key: 10 req/s sustained, up to ~10 req/burst
- 5 keys: 50 req/s aggregate (limited by `max_connections: 50`)
- Backend becomes the bottleneck before the MCP server does under typical conditions
