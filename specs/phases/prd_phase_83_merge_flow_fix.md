# RalphX - Phase 83: Merge Flow Fix

## Overview

Tasks in worktree mode end up stuck in `merge_conflict` status despite having no actual git conflicts. The root cause is that programmatic merge fails when the task branch is locked to a worktree, the merger agent receives confusing prompts, and error messages are lost between backend and MCP client. This phase fixes the merge flow to work correctly in worktree mode and adds a new `MergeIncomplete` status for non-conflict failures.

**Reference Plan:**
- `specs/plans/merge_flow_issues.md` - Detailed investigation and implementation plan for merge flow issues

## Goals

1. Fix programmatic merge to work in worktree mode by deleting worktree before merge
2. Add `MergeIncomplete` status to distinguish non-conflict failures from actual conflicts
3. Standardize error response format between backend and MCP client
4. Add diagnostic logging and `report_incomplete` endpoint for better debugging

## Dependencies

### Phase 82 (Project-Scoped Execution Control) - Required

| Dependency | Why Needed |
|------------|------------|
| Per-project execution state | Merge flow operates within project scope |

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

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/merge_flow_issues.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/merge_flow_issues.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Fix programmatic merge for worktree mode by deleting worktree before merge",
    "plan_section": "Task 0 (CRITICAL): Fix Programmatic Merge for Worktree Mode",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(git): delete worktree before programmatic merge to unlock branch",
    "steps": [
      "Read specs/plans/merge_flow_issues.md section 'Task 0 (CRITICAL)'",
      "In side_effects.rs attempt_programmatic_merge, before try_rebase_and_merge:",
      "  - Check if project.git_mode == GitMode::Worktree",
      "  - If task.worktree_path exists and path exists, delete worktree using GitService::delete_worktree",
      "  - This unlocks the branch for checkout in the main repo",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(git): delete worktree before programmatic merge to unlock branch"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add MergeIncomplete status with transitions for non-conflict failures",
    "plan_section": "Task 1: Add MergeIncomplete Status",
    "blocking": [5, 8],
    "blockedBy": [],
    "atomic_commit": "feat(state-machine): add MergeIncomplete status with transitions",
    "steps": [
      "Read specs/plans/merge_flow_issues.md section 'Task 1: Add MergeIncomplete Status'",
      "In status.rs: Add MergeIncomplete variant to InternalStatus enum",
      "In status.rs: Add 'merge_incomplete' to as_str(), FromStr, serialization tests",
      "In status.rs: Update valid_transitions for Merging to include MergeIncomplete",
      "In status.rs: Add MergeIncomplete valid_transitions -> [Merging, Merged]",
      "In types.rs: Add State::MergeIncomplete variant",
      "In types.rs: Update dispatch, name(), as_str(), FromStr for MergeIncomplete",
      "In transitions.rs: Add merge_incomplete handler function",
      "In events.rs: Add MergeAgentError event if needed for transition trigger",
      "Update all_variants() count tests (now 24 variants)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(state-machine): add MergeIncomplete status with transitions"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Standardize HTTP error responses to JSON format",
    "plan_section": "Task 2: Fix MCP Error Response Format",
    "blocking": [8],
    "blockedBy": [],
    "atomic_commit": "fix(http): standardize error responses to JSON format",
    "steps": [
      "Read specs/plans/merge_flow_issues.md section 'Task 2: Fix MCP Error Response Format'",
      "In git.rs: Create json_error helper function that returns (StatusCode, Json<serde_json::Value>)",
      "Replace all Err((StatusCode, String)) returns with json_error calls",
      "Affected functions: complete_merge (lines 79-84, 107-115, 130-140), report_conflict (260-268)",
      "Error format: { \"error\": \"message\", \"details\": \"optional\" }",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(http): standardize error responses to JSON format"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "mcp",
    "description": "Update MCP client to gracefully handle both plain text and JSON error responses",
    "plan_section": "Task 2: Fix MCP Error Response Format",
    "blocking": [],
    "blockedBy": [3],
    "atomic_commit": "fix(mcp): handle both JSON and plain text error responses",
    "steps": [
      "Read specs/plans/merge_flow_issues.md section 'Task 2: Fix MCP Error Response Format'",
      "In tauri-client.ts: Update error handling in callTauri and callTauriGet",
      "First try to parse response.json(), extract errorData.error",
      "If JSON parse fails, fall back to response.text() || response.statusText",
      "Ensure TauriClientError gets the detailed message",
      "Run npm run build in ralphx-plugin/ralphx-mcp-server to verify",
      "Commit: fix(mcp): handle both JSON and plain text error responses"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Add diagnostic logging for merge failures",
    "plan_section": "Task 3: Add Diagnostic Logging for Merge Failures",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "chore(git): add diagnostic logging for merge failures",
    "steps": [
      "Read specs/plans/merge_flow_issues.md section 'Task 3: Add Diagnostic Logging'",
      "In side_effects.rs attempt_programmatic_merge error path (lines 932-957):",
      "  - Add tracing::error with task_id, error, worktree_path, task_branch, base_branch",
      "In git_service.rs try_rebase_and_merge: Add tracing::debug for each step:",
      "  - Worktree validation",
      "  - Branch checkout attempt",
      "  - Fetch result",
      "  - Rebase result",
      "  - Merge result",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: chore(git): add diagnostic logging for merge failures"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "agent",
    "description": "Improve complete_merge tool description to clarify SHA requirements",
    "plan_section": "Task 5: Improve complete_merge Tool Description",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "docs(mcp): clarify complete_merge tool requirements",
    "steps": [
      "Read specs/plans/merge_flow_issues.md section 'Task 5: Improve complete_merge Tool Description'",
      "In tools.ts: Find complete_merge tool definition",
      "Update description to clarify:",
      "  - Call AFTER merging task branch INTO main",
      "  - Use: git checkout main && git merge <task-branch>",
      "  - Get SHA from main: git rev-parse HEAD (on main branch)",
      "  - commit_sha MUST be a commit ON the main branch, not the task branch",
      "Run npm run build in ralphx-plugin/ralphx-mcp-server to verify",
      "Commit: docs(mcp): clarify complete_merge tool requirements"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "backend",
    "description": "Fix project settings with fallback defaults and migration",
    "plan_section": "Task 6: Fix Project Settings and Add Fallback Defaults",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(project): add fallback defaults for git mode settings and migration",
    "steps": [
      "Read specs/plans/merge_flow_issues.md section 'Task 6: Fix Project Settings'",
      "In project.rs: Add base_branch_or_default() -> &str method (returns 'main' if empty)",
      "In project.rs: Add worktree_parent_or_default() -> &str method (returns '~/ralphx-worktrees' if empty)",
      "Create migrations/v24_fix_worktree_project_settings.rs:",
      "  - UPDATE projects SET base_branch = 'main' WHERE git_mode = 'worktree' AND (base_branch IS NULL OR base_branch = '')",
      "  - UPDATE projects SET worktree_parent_directory = '~/ralphx-worktrees' WHERE git_mode = 'worktree' AND (worktree_parent_directory IS NULL OR worktree_parent_directory = '')",
      "Register migration in MIGRATIONS array, bump SCHEMA_VERSION",
      "In project_service.rs: When setting git_mode to worktree, ensure base_branch and worktree_parent_directory are set",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(project): add fallback defaults for git mode settings and migration"
    ],
    "passes": false
  },
  {
    "id": 8,
    "category": "backend",
    "description": "Use MergeIncomplete status for non-conflict failures in side_effects and auto_complete",
    "plan_section": "Task 4: Use MergeIncomplete for Non-Conflict Failures",
    "blocking": [],
    "blockedBy": [2, 3],
    "atomic_commit": "feat(merge): use MergeIncomplete status for non-conflict failures",
    "steps": [
      "Read specs/plans/merge_flow_issues.md section 'Task 4: Use MergeIncomplete'",
      "In side_effects.rs attempt_programmatic_merge Err path:",
      "  - Change task.internal_status = InternalStatus::Merging to InternalStatus::MergeIncomplete",
      "  - Change persist_status_change reason from 'merge_error' to 'merge_incomplete'",
      "  - Update agent prompt: 'Merge failed for task: {task_id}. Error: {e}. Diagnose and fix.'",
      "In chat_service_send_background.rs attempt_merge_auto_complete:",
      "  - When branch not merged to main, transition to MergeIncomplete instead of MergeConflict",
      "  - Update reason message to 'merge_incomplete'",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(merge): use MergeIncomplete status for non-conflict failures"
    ],
    "passes": false
  },
  {
    "id": 9,
    "category": "backend",
    "description": "Add report_incomplete HTTP endpoint for merger agent",
    "plan_section": "Task 7: Add report_incomplete Endpoint",
    "blocking": [],
    "blockedBy": [2, 3],
    "atomic_commit": "feat(http): add report_incomplete endpoint for merge failures",
    "steps": [
      "Read specs/plans/merge_flow_issues.md section 'Task 7: Add report_incomplete Endpoint'",
      "In git.rs: Add ReportIncompleteRequest struct with reason: String, diagnostic_info: Option<String>",
      "In git.rs: Add report_incomplete handler:",
      "  - POST /api/git/tasks/{id}/report-incomplete",
      "  - Validate task is in Merging status",
      "  - Transition to MergeIncomplete using TaskTransitionService",
      "  - Return MergeOperationResponse with success and new_status",
      "Register route in http_server router",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(http): add report_incomplete endpoint for merge failures"
    ],
    "passes": false
  },
  {
    "id": 10,
    "category": "mcp",
    "description": "Add report_incomplete MCP tool for merger agent",
    "plan_section": "Task 7: Add report_incomplete Endpoint",
    "blocking": [],
    "blockedBy": [9],
    "atomic_commit": "feat(mcp): add report_incomplete tool for merge failures",
    "steps": [
      "Read specs/plans/merge_flow_issues.md section 'Task 7: Add report_incomplete Endpoint'",
      "In tools.ts: Add report_incomplete tool definition:",
      "  - name: 'report_incomplete'",
      "  - description: 'Report that merge cannot be completed due to non-conflict errors'",
      "  - inputSchema: task_id (required), reason (required), diagnostic_info (optional)",
      "In index.ts: Add handler for report_incomplete tool:",
      "  - Call POST /api/git/tasks/{task_id}/report-incomplete",
      "  - Return result from backend",
      "Run npm run build in ralphx-plugin/ralphx-mcp-server to verify",
      "Commit: feat(mcp): add report_incomplete tool for merge failures"
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
| **Delete worktree before merge** | Simplest fix - worktree will be deleted after merge anyway, unlocking branch early allows normal checkout flow |
| **New MergeIncomplete status** | Distinguishes "agent failed" from "actual conflict" - better UX and debugging |
| **JSON error responses** | Enables detailed error messages to reach agent for better diagnostics |
| **Fallback defaults in getters** | Non-breaking change - existing code continues to work, new code gets reliable defaults |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] MergeIncomplete status transitions work correctly
- [ ] Migration v24 runs without errors
- [ ] report_incomplete endpoint responds correctly

### MCP - Run `npm run build` in ralphx-mcp-server
- [ ] report_incomplete tool builds without TypeScript errors
- [ ] complete_merge tool description is updated

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] MCP Server: `npm run build` passes

### Manual Testing
- [ ] Create task in worktree mode project
- [ ] Execute task (creates commit in worktree)
- [ ] Approve task
- [ ] Verify programmatic merge succeeds (task → `merged`)
- [ ] If conflict occurs: task → `merging` with agent spawned
- [ ] If error occurs: task → `merge_incomplete` with diagnostic info

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] report_incomplete endpoint registered in router
- [ ] report_incomplete MCP tool calls correct endpoint
- [ ] MergeIncomplete status appears in Kanban UI (if visible)
- [ ] Error messages propagate from backend to agent

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
