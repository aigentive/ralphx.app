# RalphX - Phase 37: Proposal Query Tools

## Overview

Add two MCP tools to enable the orchestrator-ideation agent to query proposals: `list_session_proposals` for lightweight listing and `get_proposal` for full details. The backend infrastructure already exists (repository methods `get_by_session` and `get_by_id`), so this phase primarily wires up the MCP layer.

These tools allow the ideation agent to query existing proposals before creating or modifying them, enabling smarter proposal management and dependency tracking.

**Reference Plan:**
- `specs/plans/add_proposal_query_tools.md` - Complete implementation plan with code snippets and response examples

## Goals

1. Add `list_session_proposals` MCP tool for lightweight proposal listing
2. Add `get_proposal` MCP tool for full proposal details retrieval
3. Wire HTTP handlers to existing repository methods
4. Enable orchestrator-ideation agent to query proposals

## Dependencies

### Phase 16 (Ideation Plan Artifacts) - Required

| Dependency | Why Needed |
|------------|------------|
| TaskProposal entity | Proposals to query must exist |
| ProposalDependency repository | Dependencies are included in responses |
| orchestrator-ideation agent | The agent that will use these tools |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/add_proposal_query_tools.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol:**

Reference: `.claude/rules/commit-lock.md`

1. Establish project root: `PROJECT_ROOT="$(git rev-parse --show-toplevel)"`
2. Acquire lock before `git add` (see commit-lock.md § Protocol)
3. Stage and commit using `git -C "$PROJECT_ROOT"`
4. Release lock after commit: `rm -f "$PROJECT_ROOT/.commit-lock"`

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/add_proposal_query_tools.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "mcp",
    "description": "Add tool definitions for list_session_proposals and get_proposal",
    "plan_section": "1. Add Tool Definitions (tools.ts)",
    "blocking": [5],
    "blockedBy": [],
    "atomic_commit": "feat(mcp): add list_session_proposals and get_proposal tool definitions",
    "steps": [
      "Read specs/plans/add_proposal_query_tools.md section '1. Add Tool Definitions'",
      "Add list_session_proposals tool definition to ALL_TOOLS in IDEATION TOOLS section",
      "Add get_proposal tool definition after list_session_proposals",
      "Update TOOL_ALLOWLIST['orchestrator-ideation'] to include both new tools",
      "Run npm run build in ralphx-plugin/ralphx-mcp-server to verify compilation",
      "Commit: feat(mcp): add list_session_proposals and get_proposal tool definitions"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add proposal query response types to HTTP server",
    "plan_section": "2. Add Response Types (types.rs)",
    "blocking": [3],
    "blockedBy": [],
    "atomic_commit": "feat(http_server): add proposal query response types",
    "steps": [
      "Read specs/plans/add_proposal_query_tools.md section '2. Add Response Types'",
      "Add ProposalSummary struct with id, title, category, priority, depends_on, plan_artifact_id",
      "Add ListProposalsResponse struct with proposals vec and count",
      "Add ProposalDetailResponse struct with full proposal fields including steps and acceptance_criteria",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: feat(http_server): add proposal query response types"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add HTTP handlers for list_session_proposals and get_proposal",
    "plan_section": "3. Add HTTP Handlers (ideation.rs)",
    "blocking": [4],
    "blockedBy": [2],
    "atomic_commit": "feat(http_server): add list_session_proposals and get_proposal handlers",
    "steps": [
      "Read specs/plans/add_proposal_query_tools.md section '3. Add HTTP Handlers'",
      "Add list_session_proposals handler: get proposals by session, build dependency map, return summaries",
      "Add get_proposal handler: get single proposal, parse JSON steps/acceptance_criteria, return full details",
      "Add necessary imports (HashMap, Path from axum)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(http_server): add list_session_proposals and get_proposal handlers"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Register proposal query routes in HTTP server",
    "plan_section": "4. Add Routes (mod.rs)",
    "blocking": [5],
    "blockedBy": [3],
    "atomic_commit": "feat(http_server): register proposal query routes",
    "steps": [
      "Read specs/plans/add_proposal_query_tools.md section '4. Add Routes'",
      "Add route /api/list_session_proposals/:session_id with GET handler",
      "Add route /api/proposal/:proposal_id with GET handler",
      "Import handlers from handlers::ideation module",
      "Run cargo build to verify compilation",
      "Commit: feat(http_server): register proposal query routes"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "mcp",
    "description": "Add GET dispatch handling for proposal query tools in MCP server",
    "plan_section": "5. Add MCP Dispatch (index.ts)",
    "blocking": [],
    "blockedBy": [1, 4],
    "atomic_commit": "feat(mcp): add GET dispatch for proposal query tools",
    "steps": [
      "Read specs/plans/add_proposal_query_tools.md section '5. Add MCP Dispatch'",
      "Add else-if branch for list_session_proposals calling callTauriGet with session_id",
      "Add else-if branch for get_proposal calling callTauriGet with proposal_id",
      "Run npm run build in ralphx-plugin/ralphx-mcp-server",
      "Commit: feat(mcp): add GET dispatch for proposal query tools"
    ],
    "passes": false
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start (inverse of blocking)
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Use GET requests for query tools** | Read-only operations should use HTTP GET, consistent with existing get_* tools |
| **Return lightweight summaries for list** | Prevents large payloads when session has many proposals; get_proposal provides full details |
| **Include dependencies in responses** | Agent needs dependency info to understand proposal relationships |
| **Parse JSON strings in handler** | Steps and acceptance_criteria are stored as JSON strings in DB; deserialize at response boundary |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] ProposalSummary and ProposalDetailResponse types compile
- [ ] Handlers access repositories correctly
- [ ] Routes are registered without conflicts

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] MCP Server: `npm run build` succeeds in ralphx-plugin/ralphx-mcp-server

### Manual Testing
- [ ] Start app, verify tools appear for debug agent type
- [ ] Call `list_session_proposals` with valid session ID, verify response format
- [ ] Call `get_proposal` with proposal ID from list, verify full details returned
- [ ] Verify 404 returned for non-existent proposal ID

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Tool definitions exist in tools.ts with correct inputSchema
- [ ] TOOL_ALLOWLIST includes tools for orchestrator-ideation agent
- [ ] GET dispatch routes tool calls to callTauriGet with correct paths
- [ ] HTTP routes registered in mod.rs with correct handlers
- [ ] Handlers return correct response types

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
