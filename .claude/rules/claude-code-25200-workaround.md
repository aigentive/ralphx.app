---
paths:
  - ralphx.yaml
  - ralphx-plugin/ralphx-mcp-server/src/tools.ts
  - ralphx-plugin/agents/plan-verifier.md
---

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# Claude Code Bug #25200 Workaround

## Bug

**Issue:** [github.com/anthropics/claude-code#25200](https://github.com/anthropics/claude-code/issues/25200)

Task subagents spawned via `Task(ralphx:agent-name)` inherit the **parent agent's MCP connection** instead of using their own `mcpServers` frontmatter. This means specialist agents (e.g., `ideation-specialist-code-quality`) cannot access tools declared in their own `mcpServers` block — they get the parent's tool allowlist instead.

**Symptom:** `ideation-specialist-*` agents spawned by `plan-verifier` cannot call `create_team_artifact` even though their own `mcpServers` frontmatter declares `--agent-type ideation-specialist-*`.

## Workaround

Add the 6 specialist tools to `plan-verifier`'s allowlist across all 3 layers. This gives specialists access to these tools via the parent's inherited MCP connection.

**Status: PENDING — not yet applied** (as of 2026-03-21). Apply to all 3 files below.

**The 6 tools to add:**

| Tool | Why specialists need it |
|------|------------------------|
| `create_team_artifact` | Specialists publish findings here |
| `list_session_proposals` | Specialists read proposal list for context |
| `get_proposal` | Specialists read full proposal details |
| `search_memories` | Specialists retrieve relevant memory |
| `get_memory` | Specialists read a single memory entry |
| `get_memories_for_paths` | Specialists load file-scoped memory |

**Apply to these 3 files:**

| # | File | Location | Change |
|---|------|----------|--------|
| 1 | `ralphx-plugin/agents/plan-verifier.md` | frontmatter `tools:` list | Add 6 `mcp__ralphx__<tool>` entries |
| 2 | `ralphx-plugin/ralphx-mcp-server/src/tools.ts` | `TOOL_ALLOWLIST[PLAN_VERIFIER]` (~line 1909) | Add 6 tool name strings |
| 3 | `ralphx.yaml` | `plan-verifier` `mcp_tools` array (~line 846) | Add 6 tool names |

**Example — `plan-verifier.md` frontmatter addition:**
```yaml
  - "mcp__ralphx__create_team_artifact"
  - "mcp__ralphx__list_session_proposals"
  - "mcp__ralphx__get_proposal"
  - "mcp__ralphx__search_memories"
  - "mcp__ralphx__get_memory"
  - "mcp__ralphx__get_memories_for_paths"
```

## Revert Instructions

When Anthropic ships a fix for #25200, remove the 6 tools from all 3 files. Specialists' own `mcpServers` frontmatter already declares the correct `--agent-type` for each specialist — the inheritance bug is the only reason they were added to plan-verifier's allowlist.

| File | Location | Remove |
|------|----------|--------|
| `ralphx-plugin/agents/plan-verifier.md` | frontmatter `tools:` list | `mcp__ralphx__create_team_artifact`, `mcp__ralphx__list_session_proposals`, `mcp__ralphx__get_proposal`, `mcp__ralphx__search_memories`, `mcp__ralphx__get_memory`, `mcp__ralphx__get_memories_for_paths` |
| `ralphx-plugin/ralphx-mcp-server/src/tools.ts` | `TOOL_ALLOWLIST[PLAN_VERIFIER]` | `"create_team_artifact"`, `"list_session_proposals"`, `"get_proposal"`, `"search_memories"`, `"get_memory"`, `"get_memories_for_paths"` |
| `ralphx.yaml` | `plan-verifier` `mcp_tools` | `create_team_artifact`, `list_session_proposals`, `get_proposal`, `search_memories`, `get_memory`, `get_memories_for_paths` |

Remove the `# workaround for #25200` comment added alongside these entries, not the other entries in those arrays.

## Verification (Post-Revert)

1. Run plan verification on a session that references existing code files (triggers `ideation-specialist-code-quality` dispatch)
2. Confirm `ideation-specialist-code-quality` successfully calls `create_team_artifact` and a team artifact appears in the session
3. If `create_team_artifact` fails with a tool-not-found error → the bug is not fully fixed; re-apply the workaround
