---
paths:
  - ".mcp.json"
  - "config/ralphx.yaml"
  - "plugins/app/ralphx-mcp-server/**"
  - "plugins/app/ralphx-external-mcp/**"
  - "src-tauri/src/application/external_mcp_supervisor.rs"
  - "src-tauri/src/http_server/**"
  - "docs/external-mcp/**"
---

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# MCP Servers

Two MCP servers exist in this repo. They serve **different audiences** and must not be confused.

## 1. Internal Agent MCP — `plugins/app/ralphx-mcp-server/`

**Purpose:** Stdio-based MCP proxy for RalphX internal agent workflows. Today the embedded stdio bootstrap is primarily used by the Claude harness; other harnesses may reach the same RalphX internals through different runtime adapters.

| Aspect | Detail |
|--------|--------|
| **Transport** | Stdio (today bootstrapped through Claude CLI via `.mcp.json`) |
| **Audience** | Internal RalphX agents only |
| **Port** | N/A (stdio) — proxies to Tauri backend `:3847` |
| **Auth** | Agent-type filtering via `RALPHX_AGENT_TYPE` env + `config/ralphx.yaml` `mcp_tools` |
| **Tools** | ~42 (41 base + `permission_request`), filtered per agent type (see `agent-mcp-tools.md`) |
| **Config** | `.mcp.json` registers the Claude stdio path; `config/ralphx.yaml` controls per-agent tool access |
| **Build** | `cd plugins/app/ralphx-mcp-server && npm run build` (NON-NEGOTIABLE after source changes) |

```
Claude CLI → stdio → ralphx-mcp-server → HTTP :3847 → Tauri Backend
```

## Harness Scope Rule

| Rule | Detail |
|------|--------|
| Name the bootstrap path precisely | `.mcp.json`, `claude mcp add-json`, and stdio frontmatter behavior are Claude-path details, not universal harness rules. |
| Keep internal-tool semantics provider-neutral | Tool authz, `config/ralphx.yaml` tool grants, and backend handler behavior must stay reusable across Claude, Codex, and future harnesses. |
| Do not infer HTTP exposure from internal MCP | Internal MCP remains an internal transport choice; external/public access still belongs to `ralphx-external-mcp`. |

## 2. External API MCP — `plugins/app/ralphx-external-mcp/`

**Purpose:** HTTP+SSE MCP server exposing orchestration tools to external agents (e.g., reefbot.ai) and Tauri-owned project-chat agents. Auto-started by Tauri app when enabled.

| Aspect | Detail |
|--------|--------|
| **Transport** | HTTP+SSE (stateful sessions) |
| **Audience** | External bots, integrations, and local Tauri-owned agents that need the public orchestration surface |
| **Port** | `:3848` (configurable via `EXTERNAL_MCP_PORT`) |
| **Auth** | Bearer tokens (`rxk_live_` prefix), 30s TTL cache, TLS required for non-localhost |
| **Rate limiting** | Token bucket (10 req/s per key) + IP-based auth throttle |
| **Tools** | 34 tools (`v1_` prefixed): discovery(3), ideation(13), tasks(2), pipeline(11), events(4), onboarding(1) |
| **Startup** | Auto-managed by `ExternalMcpSupervisor` when `external_mcp.enabled = true` in `config/ralphx.yaml`. Health checks + auto-restart (up to 3x) |
| **Config** | `config/ralphx.yaml` (`external_mcp` section) + env vars (`EXTERNAL_MCP_PORT`, `EXTERNAL_MCP_HOST`, TLS cert/key) |
| **Build** | `cd plugins/app/ralphx-external-mcp && npm run build` |
| **Docs** | `docs/external-mcp/README.md`, `docs/external-mcp/api-versioning.md`, `docs/external-mcp/operational-runbook.md` |

```
External Agent → Bearer token → ralphx-external-mcp (:3848) → HTTP :3847 → Tauri Backend
Tauri-owned Agent → loopback bypass token → ralphx-external-mcp (:3848) → HTTP :3847 → Tauri Backend
```

## Key Disambiguation

| Question | Internal (`ralphx-mcp-server`) | External (`ralphx-external-mcp`) |
|----------|-------------------------------|----------------------------------|
| Who calls it? | RalphX internal harness runtimes for private implementation helpers (today mostly Claude) | Third-party bots, external integrations, Tauri-owned agents for public orchestration |
| How is it started? | Claude path is registered via `claude mcp add-json` by Tauri app; other harnesses may use different runtime adapters | Auto-started by `ExternalMcpSupervisor` (when enabled in `config/ralphx.yaml`) |
| Where are tools defined? | `src/tools.ts` + `config/ralphx.yaml` | `src/tools/*.ts` (discovery, ideation, pipeline, events, tasks, guide) |
| Domain logic? | ❌ Pure proxy + authz | ❌ Pure proxy + authz + rate limiting |
| Both proxy to? | Tauri backend `:3847` | Tauri backend `:3847` |

❌ Do NOT confuse the two — modifying the wrong server's tools will break either internal agents or external API consumers.
❌ Do NOT add external-facing auth (Bearer tokens, TLS) to the internal server — the Claude path uses stdio, not HTTP.
❌ Do NOT add internal agent-type filtering to the external server — it uses API keys, not agent types.
