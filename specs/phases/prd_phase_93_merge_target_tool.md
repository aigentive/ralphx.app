# RalphX - Phase 93: Add get_merge_target MCP Tool & Fix Hardcoded Branch Assumptions

## Overview

Tasks belonging to plans with feature branches should merge into the **plan branch**, not `main`. The backend function `resolve_merge_branches()` correctly computes the target, but this logic is only used during the **programmatic merge attempt**. When that fails (conflicts), the merger agent takes over — but it has no way to discover the correct target branch. Additionally, the `complete_merge` handler and auto-detection both hardcode `base_branch` verification, rejecting merges to plan branches.

**Bug evidence:** Task `7307dced` (plan `fb207fe2`) merged to `main` instead of `ralphx/ralphx/plan-fb207fe2`. The task is stuck in `merge_incomplete` because the system doesn't recognize the merge.

**Reference Plan:**
- `specs/plans/add_get_merge_target_mcp_tool_fix_hardcoded_branch_assumptions.md` - Detailed implementation plan with code snippets and compilation unit analysis

## Goals

1. Expose `resolve_merge_branches` so merger agent can discover the correct merge target
2. Add `get_merge_target` MCP tool + HTTP endpoint for the merger agent
3. Fix hardcoded `base_branch` verification in `complete_merge` handler and auto-completion
4. Update all three layers of the agent MCP tool allowlist (see `.claude/rules/agent-mcp-tools.md`)

## Dependencies

### Phase 85 (Feature Branch for Plan Groups) - Required

| Dependency | Why Needed |
|------------|------------|
| `resolve_merge_branches()` function | Exists since Phase 85, we're making it public |
| `PlanBranchRepository` | Needed to resolve plan-specific merge targets |
| Merger agent infrastructure | Phase 66 merger agent, Phase 76 hybrid detection |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/add_get_merge_target_mcp_tool_fix_hardcoded_branch_assumptions.md`
2. **Read `.claude/rules/agent-mcp-tools.md`** for the three-layer allowlist checklist
3. Understand the architecture and component structure
4. Then proceed with the specific task

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
2. **Read the ENTIRE implementation plan** at `specs/plans/add_get_merge_target_mcp_tool_fix_hardcoded_branch_assumptions.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Make resolve_merge_branches pub and re-export from state_machine module",
    "plan_section": "1. Extract resolve_merge_branches to shared location",
    "blocking": [2, 3, 5, 6],
    "blockedBy": [],
    "atomic_commit": "feat(state-machine): make resolve_merge_branches pub for cross-module use",
    "steps": [
      "Read specs/plans/add_get_merge_target_mcp_tool_fix_hardcoded_branch_assumptions.md section '1. Extract resolve_merge_branches'",
      "In side_effects.rs:244, change `async fn resolve_merge_branches` to `pub async fn resolve_merge_branches`",
      "In transition_handler/mod.rs, add `pub use side_effects::resolve_merge_branches;` (next to existing `complete_merge_internal` re-export at line 15)",
      "In domain/state_machine/mod.rs, add `resolve_merge_branches` to the `pub use transition_handler::` line (line 30)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(state-machine): make resolve_merge_branches pub for cross-module use"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add GET /api/git/tasks/:id/merge-target HTTP endpoint and route",
    "plan_section": "2. Add backend endpoint + route",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(git): add get_merge_target HTTP endpoint",
    "steps": [
      "Read specs/plans/add_get_merge_target_mcp_tool_fix_hardcoded_branch_assumptions.md section '2. Add backend endpoint + route'",
      "In git.rs, add `use crate::domain::state_machine::resolve_merge_branches;`",
      "Add MergeTargetResponse struct with source_branch and target_branch fields",
      "Add get_merge_target handler following get_task_commits pattern: get task, get project, call resolve_merge_branches, return response",
      "In http_server/mod.rs, add route: `.route(\"/api/git/tasks/:id/merge-target\", get(get_merge_target))` after line 91",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(git): add get_merge_target HTTP endpoint"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "mcp",
    "description": "Add get_merge_target MCP tool definition, handler, allowlists (all 3 layers), and update complete_merge description",
    "plan_section": "3. Add MCP tool definition + handler + allowlist + description update",
    "blocking": [4],
    "blockedBy": [2],
    "atomic_commit": "feat(mcp): add get_merge_target tool and update merge tool descriptions",
    "steps": [
      "Read specs/plans/add_get_merge_target_mcp_tool_fix_hardcoded_branch_assumptions.md section '3. Add MCP tool definition + handler + allowlist + description update'",
      "Read .claude/rules/agent-mcp-tools.md for the three-layer checklist",
      "LAYER 2 (MCP tools.ts): Add get_merge_target tool definition in MERGE TOOLS section after report_incomplete",
      "LAYER 2 (MCP tools.ts): Add 'get_merge_target' and 'report_incomplete' to ralphx-merger TOOL_ALLOWLIST",
      "LAYER 2 (MCP tools.ts): Update complete_merge description to remove hardcoded 'main branch' references",
      "LAYER 2 (MCP index.ts): Add GET handler dispatch for get_merge_target after report_incomplete handler",
      "LAYER 2 (MCP index.ts): Add 'get_merge_target' to taskScopedTools array",
      "LAYER 1 (Rust agent_config.rs): Add 'get_merge_target' and 'report_incomplete' to ralphx-merger allowed_mcp_tools",
      "LAYER 1 (Rust agent_config.rs): Update test_get_allowed_mcp_tools_merger_agent test to expect new tools",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Run cd ralphx-plugin/ralphx-mcp-server && npm run build",
      "Commit: feat(mcp): add get_merge_target tool and update merge tool descriptions"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "agent",
    "description": "Update merger agent prompt to use get_merge_target and fix frontmatter allowedTools",
    "plan_section": "4. Update merger agent prompt",
    "blocking": [],
    "blockedBy": [3],
    "atomic_commit": "docs(merger): update workflow to use get_merge_target",
    "steps": [
      "Read specs/plans/add_get_merge_target_mcp_tool_fix_hardcoded_branch_assumptions.md section '4. Update merger agent prompt'",
      "LAYER 3 (Agent frontmatter): Add mcp__ralphx__get_merge_target and mcp__ralphx__report_incomplete to allowedTools in merger.md frontmatter",
      "Update Step 1 to call get_merge_target first, then get_task_context",
      "Update Step 5 to merge into target_branch instead of hardcoded main",
      "Update MCP Tools Available table to include get_merge_target and report_incomplete",
      "Remove hardcoded 'ON the main branch' language throughout",
      "Commit: docs(merger): update workflow to use get_merge_target"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Fix complete_merge handler to use resolved merge target instead of hardcoded base_branch",
    "plan_section": "5. Fix complete_merge handler verification",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "fix(git): use resolved merge target in complete_merge handler",
    "steps": [
      "Read specs/plans/add_get_merge_target_mcp_tool_fix_hardcoded_branch_assumptions.md section '5. Fix complete_merge handler verification'",
      "In git.rs complete_merge handler (line 137-155), replace `let base_branch = project.base_branch.as_deref().unwrap_or(\"main\");` with `let (_, target_branch) = resolve_merge_branches(&task, &project, &state.app_state.plan_branch_repo).await;`",
      "Update is_commit_on_branch call to use &target_branch instead of base_branch",
      "Update error messages in the Err arm (lines 144-154) to reference target_branch",
      "Note: resolve_merge_branches import should already exist from Task 2 if done, otherwise add it",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(git): use resolved merge target in complete_merge handler"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "backend",
    "description": "Fix attempt_merge_auto_complete to use resolved merge target instead of hardcoded base_branch",
    "plan_section": "6. Fix attempt_merge_auto_complete verification",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "fix(merge): use resolved merge target in auto-complete verification",
    "steps": [
      "Read specs/plans/add_get_merge_target_mcp_tool_fix_hardcoded_branch_assumptions.md section '6. Fix attempt_merge_auto_complete verification'",
      "Add `use crate::domain::state_machine::resolve_merge_branches;` to imports in chat_service_send_background.rs",
      "In attempt_merge_auto_complete (lines 826-844), replace `let base_branch = project.base_branch.as_deref().unwrap_or(\"main\");` with `let (_, target_branch) = resolve_merge_branches(&task, &project, plan_branch_repo).await;`",
      "Update is_commit_on_branch call to use &target_branch",
      "Update log messages (lines 838-840, 844) to reference target_branch instead of base_branch",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(merge): use resolved merge target in auto-complete verification"
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
| **Make existing fn pub instead of duplicating** | `resolve_merge_branches` already has correct logic; duplicating it would create drift risk |
| **Merge MCP tasks 3+4+5+9 into one** | All modify same TS files; partial state (defined but not handled) causes runtime errors |
| **Three-layer allowlist update** | Rust spawn config, MCP server filter, and agent frontmatter all must agree (see `.claude/rules/agent-mcp-tools.md`) |
| **Fix verification in both handlers** | `complete_merge` (explicit) and `attempt_merge_auto_complete` (implicit) both hardcode `base_branch` |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] `resolve_merge_branches` is callable from `crate::domain::state_machine::resolve_merge_branches`
- [ ] `get_merge_target` endpoint returns correct branches for plan tasks
- [ ] `get_merge_target` endpoint returns base_branch for non-plan tasks
- [ ] `test_get_allowed_mcp_tools_merger_agent` passes with new tools

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] MCP: `cd ralphx-plugin/ralphx-mcp-server && npm run build` passes

### Manual Testing
- [ ] `curl http://127.0.0.1:3847/api/git/tasks/<task-id>/merge-target` returns correct branches
- [ ] Create a plan with feature branch, execute a task, trigger merge conflict → verify merger agent calls `get_merge_target` and merges to plan branch
- [ ] Verify `complete_merge` accepts commits on plan branch (not just main)

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `resolve_merge_branches` re-exported from `state_machine::mod.rs` and callable from `http_server::handlers::git`
- [ ] HTTP route `/api/git/tasks/:id/merge-target` registered in `mod.rs`
- [ ] MCP tool `get_merge_target` defined in `ALL_TOOLS` AND in `TOOL_ALLOWLIST["ralphx-merger"]`
- [ ] Tool handler dispatches GET request in `index.ts`
- [ ] Rust `AGENT_CONFIGS` includes `get_merge_target` and `report_incomplete` for `ralphx-merger`
- [ ] Agent frontmatter includes `mcp__ralphx__get_merge_target` and `mcp__ralphx__report_incomplete`

**Common failure modes to check:**
- [ ] No hardcoded `base_branch` remaining in merge verification paths
- [ ] No optional props defaulting to `false` or disabled
- [ ] No tools defined but not handled in dispatch

See `.claude/rules/gap-verification.md` for full verification workflow.
