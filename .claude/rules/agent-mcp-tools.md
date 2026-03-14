---
paths:
  - "src-tauri/src/infrastructure/agents/**"
  - "ralphx-plugin/agents/**"
  - "ralphx-plugin/ralphx-mcp-server/src/**"
  - "src-tauri/src/http_server/**"
---

# Adding MCP Tools to Agents

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

## Architecture (2-Layer — Single Source of Truth)

Tool allowlists are now driven by **`ralphx.yaml` → `--allowed-tools` CLI arg injection**. No manual sync across 3 files.

| Layer | File | Controls | Tech |
|-------|------|----------|------|
| 1 | `ralphx-plugin/ralphx-mcp-server/src/tools.ts` | Tool handler registration (`ALL_TOOLS`) | TypeScript |
| 2 | `ralphx.yaml` → `mcp_tools: [...]` per agent | Which tools each agent receives | YAML → Rust → CLI arg |

**How it works:** Rust `create_mcp_config()` reads `mcp_tools` from `ralphx.yaml` and injects `--allowed-tools=tool1,tool2,...` into the MCP config JSON args. MCP server parses this at startup — no TOOL_ALLOWLIST lookup needed.

**Agent `.md` frontmatter** → add `"mcp__ralphx__*"` to the `tools` list (NOT `allowedTools` — that key is invalid in frontmatter; only `tools` and `disallowedTools` are valid).

## How to Add a New MCP Tool — Checklist

**2 steps required (down from 6):**

| Step | What | File | Required? |
|------|------|------|-----------|
| 1 | Register tool handler + add to `ALL_TOOLS` array | `ralphx-plugin/ralphx-mcp-server/src/tools.ts` | Yes |
| 2 | Add tool name to agent's `mcp_tools` | `ralphx.yaml` — agent's `mcp_tools: [...]` array | Yes |
| 3 | Rebuild MCP server | `cd ralphx-plugin/ralphx-mcp-server && npm run build` | Yes (after step 1) |

**What you NO LONGER need to do:**
- ~~Edit `TOOL_ALLOWLIST` in `tools.ts`~~ (bypassed by `--allowed-tools`)
- ~~Edit Rust `AGENT_CONFIGS` `allowed_mcp_tools`~~ (removed — `ralphx.yaml` is now the single source of truth)
- ~~Edit agent `.md` frontmatter `allowedTools`~~ (`allowedTools` is NOT a valid frontmatter field — add `"mcp__ralphx__*"` to `tools` instead)

## How to Add a Completely New Tool

All checklist steps above, plus:

- [ ] **Backend:** Add HTTP handler in `src-tauri/src/http_server/handlers/<domain>.rs`
- [ ] **Backend:** Add route in `src-tauri/src/http_server/mod.rs`
- [ ] **MCP:** Add tool definition to `ALL_TOOLS` array (`tools.ts`)
- [ ] **MCP:** Add handler dispatch in `CallToolRequestSchema` (`index.ts`) — GET for queries, POST for mutations
- [ ] **MCP:** If tool has `task_id` param → add to `taskScopedTools` array (`index.ts`)

## Validation

After adding a tool, verify MCP server stderr shows:
- `[RalphX MCP] Tools from --allowed-tools: tool1, tool2, new_tool` — confirms CLI arg injection worked
- No `WARN: unknown tool` for your new tool — confirms handler is registered in `ALL_TOOLS`
- ❌ `WARN: --allowed-tools not provided, using fallback TOOL_ALLOWLIST` → means Rust injection is not working (check `ralphx.yaml` syntax and rebuild)

## TOOL_ALLOWLIST — Deprecated Fallback

> **`TOOL_ALLOWLIST` in `tools.ts` is kept as a last-resort fallback only. It is NOT the production source of truth.**

| Scenario | Behavior |
|----------|----------|
| `--allowed-tools` injected (production) | TOOL_ALLOWLIST bypassed entirely |
| `--allowed-tools` absent (standalone debug) | TOOL_ALLOWLIST used + stderr deprecation warning emitted |
| `mcp_tools: []` explicit empty | `--allowed-tools=__NONE__` injected → zero tools, no fallback |
| `mcp_tools` key absent from ralphx.yaml | `--allowed-tools` not injected → TOOL_ALLOWLIST fallback |

**Do NOT edit `TOOL_ALLOWLIST` to grant tools to agents.** Changes there have no effect in production (they're only reached when `--allowed-tools` injection fails). Update `ralphx.yaml` `mcp_tools` instead.

**Fallback chain in `getAllowedToolNames()`:**
1. `RALPHX_ALLOWED_MCP_TOOLS` env var (standalone testing only — Claude CLI does not propagate env vars to MCP servers)
2. `--allowed-tools` CLI arg (production path — injected by Rust `create_mcp_config()` from `ralphx.yaml`)
3. `TOOL_ALLOWLIST[agentType]` (deprecated fallback — emits warning when reached)

## Tool Name Formats

| Context | Format | Example |
|---------|--------|---------|
| `ralphx.yaml` `mcp_tools` | bare name | `get_merge_target` |
| TS `ALL_TOOLS` definition | bare name | `name: "get_merge_target"` |
| Agent frontmatter `tools` | wildcard | `"mcp__ralphx__*"` |

## Common Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| Agent can't see tool at all | Tool not in agent's `mcp_tools` in `ralphx.yaml` | Add tool name to `mcp_tools` |
| Tool listed but "not available" | Handler missing from `ALL_TOOLS` or `index.ts` dispatch | Register handler + rebuild |
| MCP server logs "using fallback TOOL_ALLOWLIST" | `--allowed-tools` not injected | Check `ralphx.yaml` syntax; rebuild Rust |
| Tool allowed but 404 | Handler missing or wrong route | Check `index.ts` dispatch + `mod.rs` route |
| Subagent can't use tool | Agent `.md` doesn't have `"mcp__ralphx__*"` in `tools` | Add wildcard to frontmatter `tools` list |

## Current Tool Grants (per-agent reference)

| Agent | Tools in `ralphx.yaml` `mcp_tools` |
|-------|--------------------------------------|
| `orchestrator-ideation` | `*_task_proposal`, `*_plan_artifact` (incl. `create_plan_artifact`, `update_plan_artifact`, `edit_plan_artifact`, `link_proposals_to_plan`), `get_session_plan`, `list_session_proposals`, `update_plan_verification`, `get_plan_verification`, `analyze_session_dependencies`, `get_parent_session_context`, `create_child_session`, `get_session_messages` |
| `ideation-team-lead` | Same as `orchestrator-ideation` (listed separately — `mcp_tools` is full-override, not merged) |
| `ralphx-worker` / `ralphx-coder` | `get_task_context`, `get_artifact`, `get_artifact_version`, `get_related_artifacts`, `*_step`, `execution_complete`, `get_project_analysis` |
| `ralphx-reviewer` | `complete_review`, `get_task_context` |
| `ralphx-merger` | `report_conflict`, `report_incomplete`, `get_merge_target`, `get_task_context` |
| `chat-task` | `update_task`, `add_task_note`, `get_task_details` |
| `chat-project` | `suggest_task`, `list_tasks` |

**Note:** `edit_plan_artifact` is explicitly included in both `orchestrator-ideation` and `ideation-team-lead` (added to `ralphx.yaml` 2026-03-11). `orchestrator-ideation-readonly` intentionally does NOT have it.

## Example: Adding `get_merge_target` to `ralphx-merger`

```yaml
# ralphx.yaml — agent's mcp_tools list
- name: ralphx-merger
  mcp_tools:
    - report_conflict
    - report_incomplete
    - get_merge_target  # ← ADD
    - get_task_context
```

```typescript
// tools.ts — ALL_TOOLS array (tool definition)
{
  name: "get_merge_target",
  description: "...",
  inputSchema: { ... }
}
// index.ts — CallToolRequestSchema dispatch
case "get_merge_target":
  // handler implementation
```

Then rebuild: `cd ralphx-plugin/ralphx-mcp-server && npm run build`

## Subagent MCP Access — Two Spawning Paths (NON-NEGOTIABLE)

| Path | How Agent Gets MCP | `mcpServers` in Frontmatter? |
|------|-------------------|------------------------------|
| **Backend-spawned** (ClaudeCodeClient) | Rust `create_mcp_config()` injects `--allowed-tools` into temp MCP config | Not used for own access, BUT needed for Task-tool subagents it spawns |
| **Task-tool-spawned** (in-process subagent) | Frontmatter `mcpServers` field connects to MCP server | ✅ Required — without it, zero MCP tools |

### mcpServers Frontmatter Field

Any agent that uses MCP tools MUST also include `mcpServers` in frontmatter:

```yaml
mcpServers:
  - ralphx          # reference by name (from .mcp.json)
```

Without `mcpServers`, the subagent has zero MCP tools — `tools` entries for `mcp__ralphx__*` are ignored because there's no MCP server connected.

**Three fields work together:**

| Field | Purpose | Without It |
|-------|---------|------------|
| `mcpServers` | Connects to MCP server | Zero MCP tools available |
| `tools` (with `"mcp__ralphx__*"`) | Allowlists which MCP tools are exposed | All MCP tools from connected servers available |
| `disallowedTools` | Blocks specific MCP tools | All allowed MCP tools available |

❌ `allowedTools` is NOT a valid frontmatter field — Claude Code silently ignores it
✅ Add `"mcp__ralphx__*"` to the `tools` list AND include `mcpServers`

```yaml
# ✅ Correct — mcpServers + MCP wildcard in tools
---
name: my-agent
tools:
  - Read
  - Grep
  - "mcp__ralphx__*"
mcpServers:
  - ralphx:
      type: stdio
      command: node
      args:
        - "${CLAUDE_PLUGIN_ROOT}/ralphx-mcp-server/build/index.js"
        - "--agent-type"
        - "my-agent"
---

# ❌ Wrong — allowedTools is not a valid frontmatter field (silently ignored)
---
name: my-agent
tools:
  - Read
  - Grep
allowedTools:
  - "mcp__ralphx__*"
---
```
