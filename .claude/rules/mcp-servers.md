> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# MCP Servers

Two MCP servers exist in this repo. They serve **different audiences** and must not be confused.

## 1. Internal Agent MCP — `ralphx-plugin/ralphx-mcp-server/`

**Purpose:** Stdio-based MCP proxy for RalphX's own Claude agents (workers, reviewers, mergers, ideation orchestrators). Runs embedded inside Claude CLI — NOT a standalone service.

| Aspect | Detail |
|--------|--------|
| **Transport** | Stdio (launched by Claude CLI via `.mcp.json`) |
| **Audience** | Internal RalphX agents only |
| **Port** | N/A (stdio) — proxies to Tauri backend `:3847` |
| **Auth** | Agent-type filtering via `RALPHX_AGENT_TYPE` env + `ralphx.yaml` `mcp_tools` |
| **Tools** | ~50, filtered per agent type (see `agent-mcp-tools.md`) |
| **Config** | `.mcp.json` registers it; `ralphx.yaml` controls per-agent tool access |
| **Build** | `cd ralphx-plugin/ralphx-mcp-server && npm run build` (NON-NEGOTIABLE after source changes) |

```
Claude CLI → stdio → ralphx-mcp-server → HTTP :3847 → Tauri Backend
```

## 2. External API MCP — `ralphx-plugin/ralphx-external-mcp/`

**Purpose:** HTTP+SSE MCP server exposing orchestration tools to external agents (e.g., reefbot.ai) over the network. Standalone service started manually.

| Aspect | Detail |
|--------|--------|
| **Transport** | HTTP+SSE (stateful sessions) |
| **Audience** | External bots and integrations |
| **Port** | `:3848` (configurable via `EXTERNAL_MCP_PORT`) |
| **Auth** | Bearer tokens (`rxk_live_` prefix), 30s TTL cache, TLS required for non-localhost |
| **Rate limiting** | Token bucket (10 req/s per key) + IP-based auth throttle |
| **Tools** | 33 tools (`v1_` prefixed): discovery, ideation, tasks, pipeline, events, onboarding |
| **Config** | Environment variables (`EXTERNAL_MCP_PORT`, `EXTERNAL_MCP_HOST`, TLS cert/key) |
| **Build** | `cd ralphx-plugin/ralphx-external-mcp && npm run build` |
| **Docs** | `docs/external-mcp/README.md`, `docs/external-mcp/api-versioning.md`, `docs/external-mcp/operational-runbook.md` |

```
External Agent → Bearer token → ralphx-external-mcp (:3848) → HTTP :3847 → Tauri Backend
```

## Key Disambiguation

| Question | Internal (`ralphx-mcp-server`) | External (`ralphx-external-mcp`) |
|----------|-------------------------------|----------------------------------|
| Who calls it? | RalphX's own Claude agents | Third-party bots, external integrations |
| How is it started? | Automatically by Claude CLI | Manually (`node build/index.js`) |
| Where are tools defined? | `src/tools.ts` + `ralphx.yaml` | `src/tools/*.ts` (discovery, ideation, pipeline, events, tasks, guide) |
| Domain logic? | ❌ Pure proxy + authz | ❌ Pure proxy + authz + rate limiting |
| Both proxy to? | Tauri backend `:3847` | Tauri backend `:3847` |

❌ Do NOT confuse the two — modifying the wrong server's tools will break either internal agents or external API consumers.
❌ Do NOT add external-facing auth (Bearer tokens, TLS) to the internal server — it uses stdio, not HTTP.
❌ Do NOT add internal agent-type filtering to the external server — it uses API keys, not agent types.
