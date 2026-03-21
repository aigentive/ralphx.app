# Webhooks

RalphX pushes pipeline events to registered HTTP endpoints in real-time. Webhooks are the primary transport for external integrations — polling via `v1_get_recent_events` is available as a fallback when webhooks are unavailable.

**Event delivery latency:** < 1s under normal conditions. Retries handle transient failures automatically.

---

## Registration Flow

### 1. Register your endpoint

```bash
curl -X POST http://localhost:3848/api/external/webhooks/register \
  -H "Authorization: Bearer rxk_live_<your_key>" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "http://127.0.0.1:18789/hooks/ralphx",
    "event_types": ["task:status_changed", "review:ready", "merge:completed"],
    "project_ids": ["proj-abc123"]
  }'
```

**Response:**

```json
{
  "webhook_id": "wh-ghi789",
  "url": "http://127.0.0.1:18789/hooks/ralphx",
  "secret": "a3f8c2e1d4b9...",
  "active": true,
  "created_at": "2026-03-20T14:00:00Z"
}
```

**Save the `secret` immediately** — it is returned only once and cannot be retrieved later. To rotate, delete and re-register.

### Registration options

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | HTTP/HTTPS endpoint to receive events |
| `event_types` | string[] | No | Event type filter. Omit to receive all events |
| `project_ids` | string[] | No | Project scope filter. Omit to receive events for all authorized projects |

**Project scope enforcement:** If `project_ids` is specified, each must be within the API key's authorized projects. Unauthorized project IDs return `403`. If omitted, the registration inherits the API key's project scope at registration time.

### 2. Receive events

RalphX sends `POST` requests to your endpoint with JSON bodies:

```http
POST /hooks/ralphx HTTP/1.1
Content-Type: application/json
X-RalphX-Signature: sha256=<hmac_hex>
X-RalphX-Webhook-Id: wh-ghi789
X-RalphX-Timestamp: 1710940800

{
  "event_type": "task:status_changed",
  "task_id": "task-abc123",
  "project_id": "proj-xyz",
  "old_status": "Executing",
  "new_status": "PendingReview",
  "timestamp": "2026-03-20T14:15:00Z"
}
```

**Your endpoint must respond with `2xx` within 30 seconds.** Any other response (4xx, 5xx, timeout) triggers the retry policy.

---

## HMAC Signature Verification

Every webhook request includes an `X-RalphX-Signature` header for authenticity verification.

**Algorithm:** HMAC-SHA256 over the raw request body using the registration secret.

**Format:** `sha256=<hex_digest>`

### Verification example (TypeScript/Bun)

```typescript
import { createHmac, timingSafeEqual } from 'crypto';

function verifyWebhookSignature(
  rawBody: Buffer,
  signatureHeader: string,
  secret: string
): boolean {
  const expected = 'sha256=' + createHmac('sha256', secret)
    .update(rawBody)
    .digest('hex');
  const actual = signatureHeader;

  // Use constant-time comparison to prevent timing attacks
  if (expected.length !== actual.length) return false;
  return timingSafeEqual(Buffer.from(expected), Buffer.from(actual));
}

// Express / Bun.serve handler
async function handleRalphXWebhook(req: Request): Promise<Response> {
  const rawBody = Buffer.from(await req.arrayBuffer());
  const signature = req.headers.get('X-RalphX-Signature') ?? '';

  if (!verifyWebhookSignature(rawBody, signature, process.env.RALPHX_WEBHOOK_SECRET!)) {
    return new Response('Unauthorized', { status: 401 });
  }

  const event = JSON.parse(rawBody.toString());
  await processEvent(event);
  return new Response('OK', { status: 200 });
}
```

**Always verify signatures** before processing events. Unverified endpoints are an attack surface.

---

## Retry Policy

When delivery fails, RalphX retries with exponential backoff:

| Attempt | Delay | Total elapsed |
|---------|-------|---------------|
| 1 (initial) | — | 0s |
| 2 | 1s | 1s |
| 3 | 2s | 3s |
| 4 | 4s | 7s |

After 4 failed attempts, the event is dropped for that delivery. Retryable vs. non-retryable failures:

| HTTP Status | Retried? |
|-------------|----------|
| 5xx | Yes |
| 429 (rate limited) | Yes |
| Timeout (> 30s) | Yes |
| Connection error | Yes |
| 4xx (except 429) | No — immediate fail |
| 2xx | No — success |

**Per-delivery timeout:** 10s to connect, 30s total per HTTP POST.

---

## Failure Tracking and Auto-Deactivation

RalphX tracks consecutive delivery failures per webhook:

| Failure count | Behavior |
|---------------|----------|
| 1–9 | Retries continue; failures logged |
| 10 | Webhook marked `active: false`; removed from delivery pool; `system:webhook_unhealthy` event emitted |

Check webhook health via `v1_list_webhooks`:

```json
{
  "webhook_id": "wh-ghi789",
  "url": "http://127.0.0.1:18789/hooks/ralphx",
  "active": false,
  "failure_count": 10,
  "last_failure_at": "2026-03-20T15:00:00Z"
}
```

---

## Idempotent Re-Registration

Re-registering the **same URL** is safe and idempotent:

- Returns the **existing `webhook_id`** — no duplicates created
- Resets `failure_count` to 0 and sets `active: true`
- **Does NOT regenerate the secret** — use the original secret for verification

This makes reconnect flows safe: on MCP reconnect, call `v1_register_webhook` with the same URL; the existing registration is reactivated automatically.

```typescript
// Safe to call on every reconnect
const { webhook_id } = await mcp.call('v1_register_webhook', {
  url: GATEWAY_URL,
  event_types: [],  // receive all events
});
```

After re-registration, call `v1_get_recent_events` with your last-seen cursor to backfill events missed during the disconnect window.

---

## Managing Webhooks

### List registered webhooks

```bash
curl http://localhost:3848/api/external/webhooks \
  -H "Authorization: Bearer rxk_live_<your_key>"
```

Or via MCP tool: `v1_list_webhooks`

### Unregister a webhook

```bash
curl -X DELETE "http://localhost:3848/api/external/webhooks/wh-ghi789" \
  -H "Authorization: Bearer rxk_live_<your_key>"
```

Or via MCP tool: `v1_unregister_webhook`

---

## Event Filtering

Receive only the events you care about by specifying `event_types` at registration:

```json
{
  "url": "http://127.0.0.1:18789/hooks/ralphx",
  "event_types": [
    "review:ready",
    "review:escalated",
    "merge:conflict"
  ]
}
```

Omitting `event_types` delivers all event types. See [event-types.md](event-types.md) for the full catalog.

---

## Fallback: Cursor-Based Polling

When webhooks are unavailable, poll `v1_get_recent_events` with a cursor:

```typescript
let cursor: number | undefined;

async function pollEvents() {
  const { events, next_cursor } = await mcp.call('v1_get_recent_events', {
    project_id: 'proj-xyz',
    cursor,
    limit: 50,
  });
  for (const event of events) {
    await processEvent(event);
  }
  cursor = next_cursor;
}

// Run every 5 seconds as fallback
setInterval(pollEvents, 5000);
```

The cursor is an integer event ID. Store the last-seen cursor and pass it on the next call to avoid re-processing events.
