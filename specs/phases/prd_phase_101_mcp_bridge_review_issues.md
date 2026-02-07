# RalphX - Phase 101: MCP Bridge for Review Issue Tools

## Overview

The review issue lifecycle is fully implemented in the backend (Tauri commands, domain entities, repo, DB migration) and frontend (API layer, 5 detail view components, IssueList component). But the **MCP bridge is completely missing** — no HTTP endpoints, no MCP tool definitions, no MCP handlers. This means agents cannot interact with review issues during execution or review, leaving the UI's issue progress tracking non-functional.

This phase adds the HTTP endpoints, MCP tool definitions, handler dispatch, and three-layer agent allowlist updates to complete the MCP bridge. It also enhances the reviewer agent with a re-review workflow for checking issue resolution status.

**Reference Plan:**
- `specs/plans/build_mcp_bridge_for_review_issue_tools.md` - Complete implementation plan with compilation unit analysis and dependency graph

## Goals

1. Enable worker agent to fetch, start, and resolve review issues during re-execution via MCP
2. Enable reviewer agent to check issue resolution status and step progress during re-reviews
3. Complete the three-layer agent allowlist for all review issue tools
4. Remove unimplemented UI placeholder to avoid misleading users

## Dependencies

### Phase 60 (Review Issues as First-Class Entities) - Required

| Dependency | Why Needed |
|------------|------------|
| ReviewIssue entity + repo | Backend data layer this phase bridges to MCP |
| Review issue Tauri commands | HTTP handlers mirror these commands |
| Frontend API + IssueList component | UI already wired, just needs agent data flow |

### Phase 66 (Per-Task Git Branch Isolation) - Required

| Dependency | Why Needed |
|------------|------------|
| Merger agent config pattern | Three-layer allowlist pattern used as reference |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/build_mcp_bridge_for_review_issue_tools.md`
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

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

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
2. **Read the ENTIRE implementation plan** at `specs/plans/build_mcp_bridge_for_review_issue_tools.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add HTTP handlers, request types, routes, and module registration for review issue endpoints",
    "plan_section": "1. HTTP Layer — Handlers, Types, Routes, Module Registration",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(http_server): add review issue HTTP endpoints for MCP bridge",
    "steps": [
      "Read specs/plans/build_mcp_bridge_for_review_issue_tools.md section '1. HTTP Layer'",
      "Create src-tauri/src/http_server/handlers/issues.rs with 4 HTTP handlers following handlers/steps.rs pattern",
      "Add MarkIssueInProgressRequest and MarkIssueAddressedRequest to src-tauri/src/http_server/types.rs",
      "Add 4 routes to src-tauri/src/http_server/mod.rs after review tools section",
      "Register module in src-tauri/src/http_server/handlers/mod.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(http_server): add review issue HTTP endpoints for MCP bridge"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "mcp",
    "description": "Add MCP tool definitions, registration in tools.ts allowlist, and dispatch handlers in index.ts",
    "plan_section": "2. MCP Layer — Tool Definitions, Registration, Dispatch",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(mcp): add review issue tool definitions and dispatch handlers",
    "steps": [
      "Read specs/plans/build_mcp_bridge_for_review_issue_tools.md section '2. MCP Layer'",
      "Create ralphx-plugin/ralphx-mcp-server/src/issue-tools.ts with 4 tool definitions following step-tools.ts pattern",
      "Update ralphx-plugin/ralphx-mcp-server/src/tools.ts: import ISSUE_TOOLS, add to ALL_TOOLS, update TOOL_ALLOWLIST for worker and reviewer",
      "Update ralphx-plugin/ralphx-mcp-server/src/index.ts: add 4 handler dispatch cases and taskScopedTools entries",
      "Run cd ralphx-plugin/ralphx-mcp-server && npm run typecheck",
      "Commit: feat(mcp): add review issue tool definitions and dispatch handlers"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add review issue tools to worker and reviewer allowed_mcp_tools in agent_config.rs with test updates",
    "plan_section": "3. Agent Config Layer 1 (Rust)",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(agents): add review issue tools to worker and reviewer allowlists",
    "steps": [
      "Read specs/plans/build_mcp_bridge_for_review_issue_tools.md section '3. Agent Config Layer 1'",
      "Add get_task_issues, mark_issue_in_progress, mark_issue_addressed to worker's allowed_mcp_tools",
      "Add get_task_issues, get_step_progress, get_issue_progress to reviewer's allowed_mcp_tools",
      "Update corresponding test_get_allowed_mcp_tools_* tests",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(agents): add review issue tools to worker and reviewer allowlists"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "agent",
    "description": "Add issue tools to reviewer frontmatter and add Re-Review Workflow section to system prompt",
    "plan_section": "4. Agent Config Layer 3 + Reviewer Prompt",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(agents): add issue tools to reviewer frontmatter and re-review workflow",
    "steps": [
      "Read specs/plans/build_mcp_bridge_for_review_issue_tools.md section '4. Agent Config Layer 3 + Reviewer Prompt'",
      "Add mcp__ralphx__get_task_issues, mcp__ralphx__get_step_progress, mcp__ralphx__get_issue_progress to reviewer.md frontmatter tools list",
      "Add Re-Review Workflow section to reviewer.md system prompt body",
      "Verify worker.md already has issue tools listed (no changes needed)",
      "Commit: feat(agents): add issue tools to reviewer frontmatter and re-review workflow"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Remove unimplemented 'Files Under Review' placeholder from ReviewingTaskDetail",
    "plan_section": "5. Remove 'Files Under Review' placeholder",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(ui): remove unimplemented Files Under Review placeholder",
    "steps": [
      "Read specs/plans/build_mcp_bridge_for_review_issue_tools.md section '5. Remove Files Under Review placeholder'",
      "Delete the placeholder section (data-testid='files-under-review-empty') from ReviewingTaskDetail.tsx",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(ui): remove unimplemented Files Under Review placeholder"
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
| **Reuse existing Tauri command types for HTTP responses** | `ReviewIssueResponse` and `IssueProgressResponse` already derive `Serialize` — no duplicate types needed |
| **Follow handlers/steps.rs pattern exactly** | Consistent HTTP handler structure with direct repo access and frontend event emission |
| **Three-layer allowlist for all 4 tools** | Agent MCP tool access requires Rust config + TS allowlist + agent frontmatter (see `.claude/rules/agent-mcp-tools.md`) |
| **Merge compilation units into single tasks** | HTTP handlers + types + routes + module registration must compile together; same for MCP tool definition + registration + dispatch |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] Agent config tests pass with new allowed_mcp_tools entries
- [ ] HTTP handlers compile and are registered

### MCP Server - Run `npm run typecheck`
- [ ] issue-tools.ts type-checks
- [ ] tools.ts imports resolve
- [ ] index.ts dispatch handlers type-check

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] MCP Server: `npm run typecheck` passes in `ralphx-plugin/ralphx-mcp-server/`

### Manual Testing
- [ ] `curl http://localhost:3847/api/task_issues/{task_id}` returns issues
- [ ] `curl -X POST http://localhost:3847/api/mark_issue_in_progress -d '{"issue_id":"..."}' ` updates status
- [ ] Worker agent can fetch issues via MCP during re-execution
- [ ] Worker agent can mark issues in_progress/addressed via MCP
- [ ] Reviewer agent can see issue resolution status during re-review
- [ ] UI reflects issue status changes in real-time

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] HTTP handlers are registered in mod.rs AND routed in mod.rs
- [ ] MCP tools are defined in issue-tools.ts AND registered in ALL_TOOLS AND added to TOOL_ALLOWLIST
- [ ] Dispatch handlers in index.ts call correct HTTP endpoints
- [ ] Agent configs (Rust + frontmatter) include all new tools
- [ ] "Files Under Review" placeholder is fully removed (no residual markup)

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
