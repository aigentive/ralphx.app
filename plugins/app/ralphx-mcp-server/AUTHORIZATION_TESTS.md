# Memory Write Tools - Authorization Tests

## Overview

This document specifies authorization test cases for memory write tools. These tools are RESTRICTED to memory agents only (`ralphx-memory-maintainer` and `ralphx-memory-capture`). All other agents (worker, reviewer, orchestrator, etc.) MUST be denied access.

## Test Cases

### Test 1: ralphx-memory-maintainer has access to all write tools

**Setup:**
- Set `RALPHX_AGENT_TYPE=ralphx-memory-maintainer`
- Start MCP server

**Test:**
- Call `list_tools` MCP method
- **Expected:** Tools list includes:
  - `upsert_memories`
  - `mark_memory_obsolete`
  - `refresh_memory_rule_index`
  - `ingest_rule_file`
  - `rebuild_archive_snapshots`

### Test 2: ralphx-memory-capture has access to upsert_memories only

**Setup:**
- Set `RALPHX_AGENT_TYPE=ralphx-memory-capture`
- Start MCP server

**Test:**
- Call `list_tools` MCP method
- **Expected:** Tools list includes `upsert_memories` but NOT:
  - `mark_memory_obsolete`
  - `refresh_memory_rule_index`
  - `ingest_rule_file`
  - `rebuild_archive_snapshots`

### Test 3: worker agent is denied all memory write tools

**Setup:**
- Set `RALPHX_AGENT_TYPE=ralphx-execution-worker`
- Start MCP server

**Test:**
- Call `list_tools` MCP method
- **Expected:** Tools list does NOT include any memory write tools

- Attempt to call `upsert_memories` tool
- **Expected:** Error response with message like:
  ```
  Tool "upsert_memories" is not available for agent type "ralphx-execution-worker"
  ```

### Test 4: reviewer agent is denied all memory write tools

**Setup:**
- Set `RALPHX_AGENT_TYPE=ralphx-execution-reviewer`
- Start MCP server

**Test:**
- Call `list_tools` MCP method
- **Expected:** Tools list does NOT include any memory write tools

- Attempt to call `mark_memory_obsolete` tool
- **Expected:** Error response with message like:
  ```
  Tool "mark_memory_obsolete" is not available for agent type "ralphx-execution-reviewer"
  ```

### Test 5: ralphx-ideation agent is denied all memory write tools

**Setup:**
- Set `RALPHX_AGENT_TYPE=ralphx-ideation`
- Start MCP server

**Test:**
- Call `list_tools` MCP method
- **Expected:** Tools list does NOT include any memory write tools

- Attempt to call `refresh_memory_rule_index` tool
- **Expected:** Error response with message like:
  ```
  Tool "refresh_memory_rule_index" is not available for agent type "ralphx-ideation"
  ```

### Test 6: Project scope validation enforced

**Setup:**
- Set `RALPHX_AGENT_TYPE=ralphx-memory-maintainer`
- Set `RALPHX_PROJECT_ID=project-123`
- Start MCP server

**Test:**
- Call `upsert_memories` with `project_id: "project-456"` (wrong project)
- **Expected:** Error response with message like:
  ```
  Project scope violation.

  You are assigned to project "project-123" but attempted to access project "project-456".
  ```

- Call `upsert_memories` with `project_id: "project-123"` (correct project)
- **Expected:** Request forwarded to Tauri backend (no authorization error)

## Manual Test Instructions

To manually test authorization:

1. **Test ralphx-memory-maintainer access:**
   ```bash
   cd plugins/app/ralphx-mcp-server
   RALPHX_AGENT_TYPE=ralphx-memory-maintainer node src/index.js --agent-type=ralphx-memory-maintainer
   # Send list_tools request via stdio
   # Verify all 5 write tools are present
   ```

2. **Test worker denial:**
   ```bash
   cd plugins/app/ralphx-mcp-server
   RALPHX_AGENT_TYPE=ralphx-execution-worker node src/index.js --agent-type=ralphx-execution-worker
   # Send list_tools request via stdio
   # Verify NO memory write tools are present
   # Attempt to call upsert_memories
   # Verify error response
   ```

3. **Test project scope validation:**
   ```bash
   cd plugins/app/ralphx-mcp-server
   RALPHX_AGENT_TYPE=ralphx-memory-maintainer RALPHX_PROJECT_ID=project-123 node src/index.js --agent-type=ralphx-memory-maintainer
   # Call upsert_memories with project_id: "project-456"
   # Verify project scope violation error
   ```

## Implementation Notes

Authorization is enforced at THREE layers (defense in depth):

1. **MCP Server TOOL_ALLOWLIST** (plugins/app/ralphx-mcp-server/src/tools.ts)
   - Hard-coded mapping of agent types to allowed tool names
   - Tools not in allowlist are filtered from `list_tools` response
   - Unauthorized tool calls return "tool not available" error

2. **MCP Server Request Handler** (plugins/app/ralphx-mcp-server/src/index.ts)
   - Validates `project_id` parameter matches `RALPHX_PROJECT_ID` env var
   - Returns "project scope violation" error if mismatch

3. **Backend Agent Config** (src-tauri/src/infrastructure/agents/claude/agent_config/mod.rs)
   - Rust-side agent definitions specify allowed skills/tools
   - Final enforcement layer before spawning agent

All three layers MUST agree for a tool call to succeed.

## Status

✅ Layer 1 (MCP TOOL_ALLOWLIST): Implemented in tools.ts
✅ Layer 2 (Project scope validation): Implemented in index.ts
⏳ Layer 3 (Rust agent config): Pending (separate work package)

## Related Files

- `plugins/app/ralphx-mcp-server/src/tools.ts` - TOOL_ALLOWLIST definitions
- `plugins/app/ralphx-mcp-server/src/index.ts` - Request handler with authorization checks
- `plugins/app/ralphx-mcp-server/src/agentNames.ts` - Agent name constants
- `src-tauri/src/http_server/handlers/memory.rs` - HTTP handlers for memory tools
- `src-tauri/src/http_server/types.rs` - Request/response types
- `src-tauri/src/http_server/mod.rs` - Route definitions
