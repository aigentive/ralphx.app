# RalphX External MCP Server

The External MCP Server (`ralphx-external-mcp`) exposes 40 orchestration-level tools to external agents via the Model Context Protocol over HTTP. It runs on port 3848 and proxies authenticated requests to the Tauri backend on port 3847.

## Architecture

```
External Agent
     |
     | Bearer rxk_live_... (Authorization header)
     v
ralphx-external-mcp (:3848)
  - IP auth throttle check
  - Bearer token validation (30s TTL cache → :3847/api/auth/validate-key)
  - Per-key token bucket rate limit (10 req/s)
  - MCP tool dispatch
     |
     | X-RalphX-Project-Scope header (injected)
     | X-RalphX-External-MCP: 1 header
     v
Tauri backend (:3847, localhost-only)
  - ProjectScopeGuard enforcement
  - Business logic
  - SQLite via DbConnection
```

## Quick Start

### Prerequisites

- Node.js 18+
- RalphX Tauri backend running on port 3847

### Start the server

```bash
cd plugins/app/ralphx-external-mcp
node build/index.js
```

The server listens on `http://127.0.0.1:3848` by default.

### Verify it is running

```bash
curl http://localhost:3848/health
# {"status":"ok"}

curl http://localhost:3848/ready
# {"status":"ready","backend":"reachable"}  — or 503 if :3847 is down
```

### Make an MCP tool call

```bash
curl -X POST http://localhost:3848/mcp \
  -H "Authorization: Bearer rxk_live_<your_key>" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"v1_list_projects","arguments":{}}}'
```

## Tool Categories

| Category | Count | Tools |
|----------|-------|-------|
| Discovery | 5 | v1_register_project, v1_get_agent_guide, v1_list_projects, v1_get_project_status, v1_get_pipeline_overview |
| Ideation | 13 | v1_start_ideation, v1_get_ideation_status, v1_send_ideation_message, v1_list_proposals, v1_get_proposal_detail, v1_get_plan, v1_accept_plan_and_schedule, v1_modify_proposal, v1_analyze_dependencies, v1_list_ideation_sessions, v1_get_ideation_messages, v1_trigger_plan_verification, v1_get_plan_verification |
| Tasks | 3 | v1_get_task_steps, v1_get_session_tasks, v1_batch_task_status |
| Pipeline | 12 | v1_get_task_detail, v1_get_task_diff, v1_get_review_summary, v1_approve_review, v1_request_changes, v1_get_merge_pipeline, v1_resolve_escalation, v1_pause_task, v1_cancel_task, v1_retry_task, v1_resume_scheduling, v1_create_task_note |
| Events | 4 | v1_subscribe_events, v1_get_recent_events, v1_get_attention_items, v1_get_execution_capacity |
| Webhooks | 3 | v1_register_webhook, v1_unregister_webhook, v1_list_webhooks |

## Documentation Index

| Document | Contents |
|----------|----------|
| [api-versioning.md](api-versioning.md) | v1_ prefix convention, deprecation policy, migration guide |
| [security-model.md](security-model.md) | Auth layers, TLS requirements, key lifecycle, permissions |
| [operational-runbook.md](operational-runbook.md) | Starting, health checks, key management, troubleshooting |
| [openapi-schema.yaml](openapi-schema.yaml) | Full tool schema catalog (OpenAPI 3.0 format) |
| [configuration.md](configuration.md) | All config options, environment variables, examples |
| [load-testing.md](load-testing.md) | Rate limits, token bucket behavior, k6 sample script |
| [webhooks.md](webhooks.md) | Webhook registration, HMAC signatures, failure handling, retry policy |
| [event-types.md](event-types.md) | Full event catalog with payload schemas (task, review, merge, ideation, system) |
| [autonomous-workflows.md](autonomous-workflows.md) | How autonomous agents navigate the full pipeline via MCP tools + webhooks |

## Webhooks — Real-Time Event Push

RalphX pushes pipeline events to registered external endpoints via HTTP POST. This is the primary transport for real-time integration — polling is available as a fallback only.

**Quick registration:**

```bash
curl -X POST http://localhost:3848/api/external/webhooks/register \
  -H "Authorization: Bearer rxk_live_<your_key>" \
  -H "Content-Type: application/json" \
  -d '{"url":"http://127.0.0.1:18789/hooks/ralphx"}'
# → {"webhook_id":"wh-abc123","secret":"<hex_secret>"}
```

**Verify delivery** by checking `X-RalphX-Signature` on each incoming POST using HMAC-SHA256 with the returned secret.

See [webhooks.md](webhooks.md) for full details including failure handling and idempotent re-registration.

## Source Location

```
plugins/app/ralphx-external-mcp/
├── src/
│   ├── index.ts          — Server entry point, request routing
│   ├── auth.ts           — Bearer token validation, 30s TTL cache
│   ├── rate-limiter.ts   — Token bucket + IP auth throttle
│   ├── tls.ts            — TLS validation and enforcement
│   ├── health.ts         — /health and /ready endpoints
│   ├── backend-client.ts — HTTP proxy to :3847
│   ├── types.ts          — Shared types and Permission constants
│   ├── events/           — SSE event types
│   ├── composites/       — Multi-step saga tools
│   └── tools/            — Tool handler implementations
│       ├── discovery.ts  — Flow 1: 5 discovery/onboarding tools
│       ├── ideation.ts   — Flow 2: 13 ideation tools
│       ├── tasks.ts      — Flow 3: 3 task tools
│       ├── pipeline.ts   — Flow 4: 12 pipeline tools
│       └── events.ts     — Flow 5: 7 event + webhook tools
└── build/                — Compiled JS (run after npm run build)
```
