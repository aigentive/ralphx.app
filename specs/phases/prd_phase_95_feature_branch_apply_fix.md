# RalphX - Phase 95: Fix Feature Branch Not Created on Accept Plan

## Overview

When clicking "Accept Plan" from ideation, the plan group's feature branch was not created when `session.plan_artifact_id` is NULL (no plan artifact created during ideation). The graph query uses `session_id` as a fallback for grouping, but the apply flow did not — causing tasks to lack `plan_artifact_id`, feature branches to not be created, and tasks to merge to MAIN instead of the feature branch. Additionally, the frontend did not invalidate plan-branch queries after apply, causing stale UI.

**Reference Plan:**
- `specs/plans/fix_feature_branch_not_created_on_accept_plan.md` - Root cause analysis and fix plan for plan_artifact_id fallback inconsistency

## Goals

1. Ensure `plan_artifact_id` is always set on tasks created via apply flow (using session_id fallback)
2. Ensure feature branches are always created when `use_feature_branches = true`
3. Fix `enable_feature_branch` to find tasks even when `plan_artifact_id` was NULL
4. Invalidate plan-branch queries immediately after apply for instant UI feedback

## Dependencies

### Phase 85 (Feature Branch for Plan Groups) - Required

| Dependency | Why Needed |
|------------|------------|
| Plan branch system | This phase fixes bugs in the feature branch creation flow introduced in Phase 85 |
| Merge task workflow | Tasks merging to wrong branch is a consequence of missing plan_artifact_id |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_feature_branch_not_created_on_accept_plan.md`
2. Understand the root cause analysis and how the graph query fallback differs from the apply flow
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
- All 4 tasks are independent compilation units — execute in any order
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_feature_branch_not_created_on_accept_plan.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add session_id fallback for plan_artifact_id in apply_proposals_to_kanban",
    "plan_section": "1. Backend: Apply consistent plan_artifact_id resolution in apply flow",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(ideation): use session_id fallback for plan_artifact_id in apply flow",
    "steps": [
      "Read specs/plans/fix_feature_branch_not_created_on_accept_plan.md section '1. Backend'",
      "In ideation_commands_apply.rs line 167, replace `session.plan_artifact_id.clone()` with fallback to session.id",
      "Verify: plan_artifact_id is always Some after this change",
      "Verify: feature branch creation block (line 207) now always executes when use_feature_branch=true",
      "Verify: task plan_artifact_id assignment (line 170) now always executes",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(ideation): use session_id fallback for plan_artifact_id in apply flow"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add session-based task lookup fallback in enable_feature_branch for tasks with NULL plan_artifact_id",
    "plan_section": "2. Backend: Apply same fallback in enable_feature_branch task lookup",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(plan-branch): add session-based task lookup fallback in enable_feature_branch",
    "steps": [
      "Read specs/plans/fix_feature_branch_not_created_on_accept_plan.md section '2. Backend'",
      "In plan_branch_commands.rs lines 160-173, extend the task filter to also match tasks linked via task_proposals where session_id matches",
      "Alternative simpler approach: backfill plan_artifact_id on matched tasks that have it NULL",
      "Ensure merge task gets correct blockedBy dependencies for all plan tasks",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(plan-branch): add session-based task lookup fallback in enable_feature_branch"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Invalidate plan-branch queries in useApplyProposals onSuccess callback",
    "plan_section": "3. Frontend: Invalidate plan-branch queries after apply",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(ideation): invalidate plan-branch queries after apply",
    "steps": [
      "Read specs/plans/fix_feature_branch_not_created_on_accept_plan.md section '3. Frontend'",
      "In useApplyProposals.ts onSuccess callback, add queryClient.invalidateQueries for plan-branch queries",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(ideation): invalidate plan-branch queries after apply"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Reduce plan-branch staleTime from 30s to 5s for faster UI state indicator updates",
    "plan_section": "4. Frontend: Reduce plan-branch staleTime",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(graph): reduce plan-branch staleTime for faster UI updates",
    "steps": [
      "Read specs/plans/fix_feature_branch_not_created_on_accept_plan.md section '4. Frontend'",
      "In PlanGroupHeader.tsx line 219, change staleTime from 30_000 to 5_000",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(graph): reduce plan-branch staleTime for faster UI updates"
    ],
    "passes": true
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
| **Use session_id as fallback for plan_artifact_id** | Matches the graph query logic (query.rs:551-554) that already uses this pattern, ensuring consistency across the codebase |
| **All tasks independent (no blockedBy)** | Each modifies a separate file with no shared type/signature changes — all are complete compilation units |
| **Backfill approach for enable_feature_branch** | Simpler than proposal-tracing: just find tasks via session and set their plan_artifact_id if NULL |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] All existing tests pass
- [ ] plan_artifact_id is always set on tasks after apply (even when session.plan_artifact_id is NULL)

### Frontend - Run `npm run test`
- [ ] All existing tests pass

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes

### Manual Testing
- [ ] Create new ideation session (no plan artifact), add proposals, click "Accept Plan"
- [ ] Check DB: `SELECT plan_artifact_id FROM tasks WHERE ...` — should have plan_artifact_id set
- [ ] Check DB: `SELECT * FROM plan_branches WHERE ...` — should have active branch with merge_task_id
- [ ] Check UI: Navigate to graph immediately after accept — toggle should be ON
- [ ] Execute a task — should merge to feature branch, not main

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Accept Plan button → apply_proposals_to_kanban → plan_artifact_id always resolved
- [ ] Feature branch toggle → enable_feature_branch → finds tasks even with NULL plan_artifact_id
- [ ] Apply success → plan-branch query invalidated → UI refreshes immediately

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
