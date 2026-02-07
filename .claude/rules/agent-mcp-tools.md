# Adding MCP Tools to Agents

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

## Three-Layer Allowlist (ALL required)

Adding an MCP tool to an agent requires updates in **three places**. Missing any one → tool silently unavailable.

| # | Layer | File | What to Update | Controls |
|---|-------|------|----------------|----------|
| 1 | **Rust spawn config** | `src-tauri/src/infrastructure/agents/claude/agent_config.rs` | `AGENT_CONFIGS` → agent's `allowed_mcp_tools` array | `--allowedTools` flag at spawn time |
| 2 | **MCP server filter** | `ralphx-plugin/ralphx-mcp-server/src/tools.ts` | `TOOL_ALLOWLIST` → agent's array | Server-side tool filtering |
| 3 | **Agent frontmatter** | `ralphx-plugin/agents/<name>.md` | `allowedTools:` YAML list (prefix: `mcp__ralphx__`) | Subagent spawning + documentation |

## Adding a New Tool to an Existing Agent — Checklist

- [ ] **Layer 1 (Rust):** Add tool name to `allowed_mcp_tools` in `AGENT_CONFIGS` (`agent_config.rs`)
- [ ] **Layer 1 (Rust):** Update corresponding test (`test_get_allowed_mcp_tools_<agent>`)
- [ ] **Layer 2 (MCP):** Add tool name to `TOOL_ALLOWLIST[agent]` (`tools.ts`)
- [ ] **Layer 2 (MCP):** Add handler in `CallToolRequestSchema` dispatch (`index.ts`)
- [ ] **Layer 2 (MCP):** If tool has `task_id` param → add to `taskScopedTools` array (`index.ts`)
- [ ] **Layer 3 (Agent):** Add `mcp__ralphx__<tool_name>` to frontmatter `allowedTools` (`agents/<name>.md`)

## Adding a Completely New Tool — Checklist

All of the above, plus:

- [ ] **Backend:** Add HTTP handler in `src-tauri/src/http_server/handlers/<domain>.rs`
- [ ] **Backend:** Add route in `src-tauri/src/http_server/mod.rs`
- [ ] **MCP:** Add tool definition to `ALL_TOOLS` array (`tools.ts`)
- [ ] **MCP:** Add handler dispatch in `CallToolRequestSchema` (`index.ts`) — GET for queries, POST for mutations

## Tool Name Formats

| Context | Format | Example |
|---------|--------|---------|
| Rust `allowed_mcp_tools` | bare name | `"get_merge_target"` |
| TS `TOOL_ALLOWLIST` | bare name | `"get_merge_target"` |
| Agent frontmatter `allowedTools` | prefixed | `mcp__ralphx__get_merge_target` |
| TS `ALL_TOOLS` definition | bare name | `name: "get_merge_target"` |

## Common Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| Agent can't see tool at all | Missing from Layer 1 (Rust `allowed_mcp_tools`) | Tool not in `--allowedTools` at spawn |
| Tool listed but "not available" error | Missing from Layer 2 (`TOOL_ALLOWLIST`) | MCP server rejects at runtime |
| Subagent can't use tool | Missing from Layer 3 (frontmatter `allowedTools`) | Only affects subagent spawning |
| Tool allowed but 404 | Handler missing or wrong route | Check `index.ts` dispatch + `mod.rs` route |

## Example: Adding `get_merge_target` to `ralphx-merger`

```rust
// Layer 1: agent_config.rs
AgentConfig {
    name: "ralphx-merger",
    allowed_mcp_tools: &[
        "complete_merge",
        "report_conflict",
        "report_incomplete",
        "get_merge_target",  // ← ADD
        "get_task_context",
    ],
    ...
}
```

```typescript
// Layer 2: tools.ts
"ralphx-merger": [
    "complete_merge",
    "report_conflict",
    "report_incomplete",
    "get_merge_target",  // ← ADD
    "get_task_context",
],
```

```yaml
# Layer 3: agents/merger.md frontmatter
allowedTools:
  - mcp__ralphx__complete_merge
  - mcp__ralphx__report_conflict
  - mcp__ralphx__report_incomplete
  - mcp__ralphx__get_merge_target  # ← ADD
  - mcp__ralphx__get_task_context
```
