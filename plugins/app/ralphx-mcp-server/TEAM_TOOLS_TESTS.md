# Team Tools - Authorization and Functionality Tests

## Overview

This document specifies test cases for the new team coordination tools and agent types. Tests verify:
- Tool allowlist enforcement for 3 new agent types (ralphx-ideation-team-lead, ideation-team-member, worker-team-member)
- RALPHX_ALLOWED_MCP_TOOLS env var override functionality
- Input schema validation for 6 new team tools

## Agent Type Authorization Tests

### Test 1: ralphx-ideation-team-lead has access to team coordination tools

**Setup:**
- Set `RALPHX_AGENT_TYPE=ralphx-ideation-team-lead`
- Start MCP server

**Test:**
- Call `list_tools` MCP method
- **Expected:** Tools list includes:
  - `request_team_plan`
  - `request_teammate_spawn`
  - `create_team_artifact`
  - `get_team_artifacts`
  - `get_team_session_state`
  - `save_team_session_state`
  - All existing ralphx-ideation tools (create_task_proposal, create_plan_artifact, etc.)

### Test 2: ideation-team-member has limited read-only access

**Setup:**
- Set `RALPHX_AGENT_TYPE=ideation-team-member`
- Start MCP server

**Test:**
- Call `list_tools` MCP method
- **Expected:** Tools list includes:
  - `create_team_artifact`
  - `get_team_artifacts`
  - `get_session_plan`
  - `list_session_proposals`
  - `get_artifact`
  - Memory read tools (search_memories, get_memory, get_memories_for_paths)

- **Expected:** Tools list does NOT include:
  - `request_team_plan` (lead-only)
  - `request_teammate_spawn` (lead-only)
  - `create_task_proposal` (lead-only)
  - `save_team_session_state` (lead-only)

- Attempt to call `request_team_plan` tool
- **Expected:** Error response:
  ```
  Tool "request_team_plan" is not available for agent type "ideation-team-member"
  ```

### Test 3: worker-team-member has implementation tools + team artifacts

**Setup:**
- Set `RALPHX_AGENT_TYPE=worker-team-member`
- Start MCP server

**Test:**
- Call `list_tools` MCP method
- **Expected:** Tools list includes:
  - `create_team_artifact` (document decisions)
  - `get_team_artifacts`
  - All worker step tools (start_step, complete_step, etc.)
  - All worker context tools (get_task_context, get_artifact, etc.)
  - Issue tools (get_task_issues, mark_issue_in_progress, etc.)

- **Expected:** Tools list does NOT include:
  - `request_team_plan` (lead-only)
  - `request_teammate_spawn` (lead-only)
  - `save_team_session_state` (lead-only)

### Test 4: Regular worker agent is denied team coordination tools

**Setup:**
- Set `RALPHX_AGENT_TYPE=ralphx-execution-worker`
- Start MCP server

**Test:**
- Call `list_tools` MCP method
- **Expected:** Tools list does NOT include any team coordination tools:
  - `request_team_plan`
  - `request_teammate_spawn`
  - `get_team_session_state`
  - `save_team_session_state`

- Attempt to call `request_team_plan` tool
- **Expected:** Error response:
  ```
  Tool "request_team_plan" is not available for agent type "ralphx-execution-worker"
  ```

### Test 5: ralphx-ideation agent is denied team coordination tools

**Setup:**
- Set `RALPHX_AGENT_TYPE=ralphx-ideation`
- Start MCP server

**Test:**
- Call `list_tools` MCP method
- **Expected:** Tools list does NOT include team coordination tools (team tools are for ralphx-ideation-team-lead only)

- Attempt to call `request_teammate_spawn` tool
- **Expected:** Error response:
  ```
  Tool "request_teammate_spawn" is not available for agent type "ralphx-ideation"
  ```

## Env Var Override Tests

### Test 6: RALPHX_ALLOWED_MCP_TOOLS overrides hardcoded allowlist

**Setup:**
- Set `RALPHX_AGENT_TYPE=ideation-team-member`
- Set `RALPHX_ALLOWED_MCP_TOOLS=get_session_plan,create_team_artifact`
- Start MCP server

**Test:**
- Call `list_tools` MCP method
- **Expected:** Tools list includes ONLY:
  - `get_session_plan`
  - `create_team_artifact`
  - `permission_request` (always included)

- **Expected:** Tools list does NOT include other team-member tools:
  - `get_team_artifacts`
  - `list_session_proposals`
  - `get_artifact`

- Attempt to call `get_team_artifacts` tool (not in env var list)
- **Expected:** Error response:
  ```
  Tool "get_team_artifacts" is not available for agent type "ideation-team-member"
  ```

### Test 7: Empty RALPHX_ALLOWED_MCP_TOOLS means no tools

**Setup:**
- Set `RALPHX_AGENT_TYPE=worker-team-member`
- Set `RALPHX_ALLOWED_MCP_TOOLS=` (empty string)
- Start MCP server

**Test:**
- Call `list_tools` MCP method
- **Expected:** Tools list includes ONLY:
  - `permission_request` (always included)

- Attempt to call `get_task_context` tool
- **Expected:** Error response:
  ```
  Tool "get_task_context" is not available for agent type "worker-team-member"
  ```

### Test 8: RALPHX_ALLOWED_MCP_TOOLS with whitespace handling

**Setup:**
- Set `RALPHX_AGENT_TYPE=ideation-team-member`
- Set `RALPHX_ALLOWED_MCP_TOOLS=  get_session_plan  ,  create_team_artifact  ,  ` (with extra spaces)
- Start MCP server

**Test:**
- Call `list_tools` MCP method
- **Expected:** Tools list includes (whitespace trimmed):
  - `get_session_plan`
  - `create_team_artifact`
  - `permission_request`

## Input Schema Validation Tests

### Test 9: request_team_plan requires valid process and teammates

**Setup:**
- Set `RALPHX_AGENT_TYPE=ralphx-ideation-team-lead`
- Start MCP server

**Test:**
- Call `request_team_plan` with missing `process` field
- **Expected:** Schema validation error (required field)

- Call `request_team_plan` with empty `teammates` array
- **Expected:** Forwarded to backend (backend handles business logic validation)

- Call `request_team_plan` with valid data:
  ```json
  {
    "process": "ideation-research",
    "teammates": [
      {
        "role": "frontend-researcher",
        "tools": ["Read", "Grep"],
        "mcp_tools": ["get_session_plan"],
        "model": "sonnet",
        "prompt_summary": "Research React patterns"
      }
    ]
  }
  ```
- **Expected:** Forwarded to Tauri backend via POST /api/team/plan

### Test 10: create_team_artifact requires valid artifact_type

**Setup:**
- Set `RALPHX_AGENT_TYPE=ideation-team-member`
- Start MCP server

**Test:**
- Call `create_team_artifact` with missing `artifact_type` field
- **Expected:** Schema validation error (required field)

- Call `create_team_artifact` with invalid artifact_type:
  ```json
  {
    "session_id": "sess-123",
    "title": "Research",
    "content": "...",
    "artifact_type": "InvalidType"
  }
  ```
- **Expected:** Schema validation error (must be TeamResearch|TeamAnalysis|TeamSummary)

- Call `create_team_artifact` with valid data:
  ```json
  {
    "session_id": "sess-123",
    "title": "Frontend Research Findings",
    "content": "## Patterns\n- Component structure...",
    "artifact_type": "TeamResearch"
  }
  ```
- **Expected:** Forwarded to Tauri backend via POST /api/team/artifact

### Test 11: request_teammate_spawn requires model within enum

**Setup:**
- Set `RALPHX_AGENT_TYPE=ralphx-ideation-team-lead`
- Start MCP server

**Test:**
- Call `request_teammate_spawn` with invalid model:
  ```json
  {
    "role": "coder-1",
    "prompt": "...",
    "model": "gpt-4",
    "tools": ["Read"],
    "mcp_tools": ["get_task_context"]
  }
  ```
- **Expected:** Schema validation error (model must be haiku|sonnet|opus)

- Call `request_teammate_spawn` with valid data:
  ```json
  {
    "role": "frontend-researcher",
    "prompt": "Research React patterns...",
    "model": "sonnet",
    "tools": ["Read", "Grep", "Glob"],
    "mcp_tools": ["get_session_plan", "create_team_artifact"]
  }
  ```
- **Expected:** Forwarded to Tauri backend via POST /api/team/spawn

### Test 12: get_team_artifacts requires session_id

**Setup:**
- Set `RALPHX_AGENT_TYPE=ideation-team-member`
- Start MCP server

**Test:**
- Call `get_team_artifacts` with missing `session_id` field
- **Expected:** Schema validation error (required field)

- Call `get_team_artifacts` with valid data:
  ```json
  {
    "session_id": "sess-123"
  }
  ```
- **Expected:** Forwarded to Tauri backend via GET /api/team/artifacts/sess-123

### Test 13: save_team_session_state requires all core fields

**Setup:**
- Set `RALPHX_AGENT_TYPE=ralphx-ideation-team-lead`
- Start MCP server

**Test:**
- Call `save_team_session_state` with missing `team_composition` field
- **Expected:** Schema validation error (required field)

- Call `save_team_session_state` with invalid team_composition item (missing required fields):
  ```json
  {
    "session_id": "sess-123",
    "team_composition": [
      {
        "name": "coder-1"
        // missing role, prompt, model
      }
    ],
    "phase": "EXPLORE"
  }
  ```
- **Expected:** Schema validation error (team_composition items require name, role, prompt, model)

- Call `save_team_session_state` with valid data:
  ```json
  {
    "session_id": "sess-123",
    "team_composition": [
      {
        "name": "frontend-researcher",
        "role": "Frontend Research Specialist",
        "prompt": "Research React patterns...",
        "model": "sonnet"
      }
    ],
    "phase": "EXPLORE",
    "artifact_ids": ["art-1", "art-2"]
  }
  ```
- **Expected:** Forwarded to Tauri backend via POST /api/team/session_state

## HTTP Endpoint Routing Tests

### Test 14: All 6 new tools route to correct endpoints

**Setup:**
- Set `RALPHX_AGENT_TYPE=ralphx-ideation-team-lead`
- Start MCP server
- Mock Tauri backend to log incoming requests

**Test:**
- Call `request_team_plan` → **Expected route:** POST /api/team/plan
- Call `request_teammate_spawn` → **Expected route:** POST /api/team/spawn
- Call `create_team_artifact` → **Expected route:** POST /api/team/artifact
- Call `get_team_artifacts` → **Expected route:** GET /api/team/artifacts/{session_id}
- Call `get_team_session_state` → **Expected route:** GET /api/team/session_state/{session_id}
- Call `save_team_session_state` → **Expected route:** POST /api/team/session_state

## Manual Test Instructions

### Test ralphx-ideation-team-lead access:
```bash
cd plugins/app/ralphx-mcp-server
RALPHX_AGENT_TYPE=ralphx-ideation-team-lead node build/index.js --agent-type=ralphx-ideation-team-lead
# Send list_tools request via stdio
# Verify all 6 team coordination tools are present
```

### Test ideation-team-member limited access:
```bash
cd plugins/app/ralphx-mcp-server
RALPHX_AGENT_TYPE=ideation-team-member node build/index.js --agent-type=ideation-team-member
# Send list_tools request via stdio
# Verify create_team_artifact and get_team_artifacts present
# Verify request_team_plan NOT present
```

### Test env var override:
```bash
cd plugins/app/ralphx-mcp-server
RALPHX_AGENT_TYPE=ideation-team-member \
RALPHX_ALLOWED_MCP_TOOLS=get_session_plan,create_team_artifact \
node build/index.js --agent-type=ideation-team-member
# Send list_tools request via stdio
# Verify ONLY get_session_plan and create_team_artifact present
```

### Test input schema validation:
```bash
cd plugins/app/ralphx-mcp-server
RALPHX_AGENT_TYPE=ralphx-ideation-team-lead node build/index.js --agent-type=ralphx-ideation-team-lead
# Send request_team_plan with missing process field
# Verify schema validation error
# Send request_team_plan with valid data
# Verify forwarded to backend
```

## Implementation Notes

Authorization is enforced at THREE layers (defense in depth):

1. **MCP Server TOOL_ALLOWLIST** (plugins/app/ralphx-mcp-server/src/tools.ts)
   - Hard-coded mapping of agent types to allowed tool names
   - Added 3 new agent types: ralphx-ideation-team-lead, ideation-team-member, worker-team-member
   - RALPHX_ALLOWED_MCP_TOOLS env var overrides hardcoded allowlist if set

2. **MCP Server Request Handler** (plugins/app/ralphx-mcp-server/src/index.ts)
   - Routes 6 new team tools to Tauri backend
   - Input schema validation handled by MCP SDK

3. **Backend Agent Config** (src-tauri/src/infrastructure/agents/claude/agent_config/mod.rs)
   - Rust-side agent definitions specify allowed skills/tools
   - Final enforcement layer before spawning agent

All three layers MUST agree for a tool call to succeed.

## Status

✅ Layer 1 (MCP TOOL_ALLOWLIST): Implemented in tools.ts (3 new agent types, env var override)
✅ Layer 2 (HTTP routing): Implemented in index.ts (6 new tool routes)
✅ Layer 3 (Input schemas): Defined in tools.ts (all 6 tools with clear inputSchema)
⏳ Backend endpoints (/api/team/*): Pending (task #2 - rust-services)

## Related Files

- `plugins/app/ralphx-mcp-server/src/tools.ts` - TOOL_ALLOWLIST + new tool definitions
- `plugins/app/ralphx-mcp-server/src/index.ts` - Request handler with new tool routing
- `plugins/app/ralphx-mcp-server/src/agentNames.ts` - New agent type constants
- `plugins/app/agents/ralphx-ideation-team-lead.md` - Team lead system prompt
- `plugins/app/agents/worker-team.md` - Worker team system prompt
- `plugins/app/agents/ideation-specialist-*.md` - Optional specialist templates
