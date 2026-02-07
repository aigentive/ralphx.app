# RalphX - Phase 97: Plan Branch Session-Based Lookup

## Overview

Phase 96 added `ideation_session_id` to tasks and fixed the FK constraint, but never updated the plan branch **lookup logic** to use it. All plan branch lookups still go through `task.plan_artifact_id` → `get_by_plan_artifact_id()`, which returns nothing when `plan_artifact_id` is NULL (common case for sessions without a plan artifact).

This phase completes the session-based lookup migration: adds `get_by_session_id()` to the repository, updates all callers (merge resolution, apply flow, branch commands), and fixes stale UI after toggle.

**Reference Plan:**
- `specs/plans/migrate_plan_branches_session_id.md` - Detailed implementation plan with line-level code changes

## Goals

1. Add `get_by_session_id()` as primary plan branch lookup (with UNIQUE index)
2. Fix merge resolution to find feature branches via `ideation_session_id`
3. Fix feature branch auto-creation on Accept Plan (remove `plan_artifact_id` gate)
4. Fix stale toggle UI after enable/disable

## Dependencies

### Phase 96 (Fix FK Constraint — Add ideation_session_id to Tasks) - Required

| Dependency | Why Needed |
|------------|------------|
| `ideation_session_id` column on tasks | Tasks must have session_id to look up plan branches |
| `plan_branches.session_id` column | Exists since Phase 85 (v13 migration) |

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/migrate_plan_branches_session_id.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/migrate_plan_branches_session_id.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add UNIQUE index on plan_branches.session_id (v16 migration)",
    "plan_section": "Step 1: Migration — Add UNIQUE index on session_id",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(db): add UNIQUE index on plan_branches.session_id",
    "steps": [
      "Read specs/plans/migrate_plan_branches_session_id.md section 'Step 1'",
      "Create src-tauri/src/infrastructure/sqlite/migrations/v16_plan_branch_session_index.rs with CREATE UNIQUE INDEX IF NOT EXISTS",
      "Register v16 migration in mod.rs, bump SCHEMA_VERSION to 16",
      "Create v16_plan_branch_session_index_tests.rs with migration test",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(db): add UNIQUE index on plan_branches.session_id"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add get_by_session_id() to PlanBranchRepository trait and all implementations",
    "plan_section": "Step 2: Repository — Add get_by_session_id()",
    "blocking": [3, 4, 5],
    "blockedBy": [1],
    "atomic_commit": "feat(plan-branch): add get_by_session_id to PlanBranchRepository trait",
    "steps": [
      "Read specs/plans/migrate_plan_branches_session_id.md section 'Step 2'",
      "Add get_by_session_id(&self, session_id: &IdeationSessionId) -> AppResult<Option<PlanBranch>> to PlanBranchRepository trait",
      "Implement in SqlitePlanBranchRepository: SELECT * FROM plan_branches WHERE session_id = ?1",
      "Implement in MemoryPlanBranchRepository: find by b.session_id == *session_id",
      "Add tests for get_by_session_id in sqlite_plan_branch_repo.rs (found, not_found cases)",
      "NOTE: Only 2 trait impls exist (SQLite, Memory) — both covered here. No other mocks to update.",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(plan-branch): add get_by_session_id to PlanBranchRepository trait"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Fix merge resolution to use ideation_session_id → get_by_session_id()",
    "plan_section": "Step 3: Fix merge resolution — side_effects.rs",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "fix(plan-branch): resolve merge branches via session_id instead of plan_artifact_id",
    "steps": [
      "Read specs/plans/migrate_plan_branches_session_id.md section 'Step 3'",
      "In resolve_task_base_branch(): change task.plan_artifact_id → task.ideation_session_id, get_by_plan_artifact_id → get_by_session_id",
      "In resolve_merge_branches() Path 2: change task.plan_artifact_id → task.ideation_session_id, get_by_plan_artifact_id → get_by_session_id",
      "Update make_task() test helper to accept ideation_session_id parameter",
      "Update make_plan_branch() to use consistent session IDs matching test tasks",
      "Update all test assertions that exercise plan branch lookup to set task.ideation_session_id",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(plan-branch): resolve merge branches via session_id instead of plan_artifact_id"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Remove plan_artifact_id gate from feature branch creation in apply_proposals_to_kanban",
    "plan_section": "Step 4: Fix auto-create on Accept Plan — ideation_commands_apply.rs",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "fix(plan-branch): remove plan_artifact_id gate from apply_proposals_to_kanban",
    "steps": [
      "Read specs/plans/migrate_plan_branches_session_id.md section 'Step 4'",
      "CRITICAL FK SAFETY: tasks.plan_artifact_id has FK to artifacts table. plan_branches.plan_artifact_id does NOT. Only use session_id fallback for plan_branches, NEVER for tasks.",
      "Keep Phase 96 behavior for tasks: only set tasks.plan_artifact_id when real artifact_id exists (preserve the if let Some gate for task updates at line 171)",
      "Remove the if let Some gate around FEATURE BRANCH CREATION (line 208): gate on session_id availability instead",
      "Compute effective_plan_id (real artifact_id or session_id fallback) ONLY for plan_branches.plan_artifact_id column (no FK constraint)",
      "Change feature branch existence check: get_by_session_id(&session_id) instead of get_by_plan_artifact_id",
      "Merge task: set ideation_session_id always, plan_artifact_id only if real artifact exists",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(plan-branch): remove plan_artifact_id gate from apply_proposals_to_kanban"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Session-first lookup in get/enable/disable plan branch commands",
    "plan_section": "Step 5: Fix plan branch commands — plan_branch_commands.rs",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "fix(plan-branch): session-first lookup in get/enable/disable commands",
    "steps": [
      "Read specs/plans/migrate_plan_branches_session_id.md section 'Step 5'",
      "get_plan_branch: try get_by_session_id() first (treat plan_artifact_id param as possible session_id), fallback to get_by_plan_artifact_id()",
      "enable_feature_branch: use get_by_session_id(&session_id) for existence check and re-fetch after creation",
      "disable_feature_branch: try get_by_session_id() first, fallback to get_by_plan_artifact_id()",
      "Keep existing param names for frontend compatibility",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(plan-branch): session-first lookup in get/enable/disable commands"
    ],
    "passes": true
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Fix stale toggle after feature branch enable — always invalidate cache",
    "plan_section": "Step 6: Fix frontend error handling — PlanGroupSettings.tsx",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(graph): fix stale toggle after feature branch enable",
    "steps": [
      "Read specs/plans/migrate_plan_branches_session_id.md section 'Step 6'",
      "Move onBranchChange?.() from try block to finally block (always invalidate cache)",
      "Handle Tauri string errors in catch: typeof err === 'string' ? err : err instanceof Error ? err.message : 'Failed to update'",
      "Silence 'already exists' error (just stale UI state, refetch will correct it)",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(graph): fix stale toggle after feature branch enable"
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
| **Add `get_by_session_id()` vs modify `get_by_plan_artifact_id()`** | Keep backward compat — existing callers still work, new callers use session_id |
| **UNIQUE index on session_id** | Enforces 1:1 session-to-branch mapping at DB level, enables efficient lookup |
| **Session-first fallback in commands** | Frontend sends `plan_artifact_id` param which may actually be a session_id (graph uses session_id as fallback) |
| **`effective_plan_id` only for `plan_branches` table** | `plan_branches.plan_artifact_id` is NOT NULL (no FK) — session_id fallback OK. `tasks.plan_artifact_id` HAS FK to artifacts — only set real artifact_id (Phase 96 fix) |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] v16 migration creates index successfully on fresh and existing DBs
- [ ] `get_by_session_id` returns correct plan branch
- [ ] `get_by_session_id` returns None for unknown session
- [ ] `resolve_task_base_branch` finds feature branch via `ideation_session_id`
- [ ] `resolve_merge_branches` routes plan task to feature branch via `ideation_session_id`
- [ ] All existing tests still pass (backward compat)

### Frontend - Run `npm run lint && npm run typecheck`
- [ ] PlanGroupSettings toggle always invalidates cache (even on error)
- [ ] String errors from Tauri are handled correctly

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes

### Manual Testing
- [ ] Create ideation session (no plan artifact) → Accept Plan → feature branch auto-created
- [ ] Verify merge task exists and is blocked by plan tasks
- [ ] Execute a task → verify merge target is feature branch (not main)
- [ ] Toggle feature branch OFF/ON in settings → UI updates immediately
- [ ] DB verification: `sqlite3 src-tauri/ralphx.db "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_plan_branches_session_id';"`

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `get_by_session_id()` is called from `resolve_task_base_branch()`, `resolve_merge_branches()`, `get_plan_branch`, `enable_feature_branch`, `disable_feature_branch`
- [ ] `apply_proposals_to_kanban` creates feature branch even when `plan_artifact_id` is NULL
- [ ] `onBranchChange` fires in finally block (not just try block)

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No functions exported but never called
- [ ] `get_by_plan_artifact_id` still works for backward compat

See `.claude/rules/gap-verification.md` for full verification workflow.
