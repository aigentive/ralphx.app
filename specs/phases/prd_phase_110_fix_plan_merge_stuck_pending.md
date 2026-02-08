# RalphX - Phase 110: Fix Plan Merge Tasks Stuck in `pending_merge`

## Overview

Plan merge tasks intermittently get stuck in `pending_merge` because `plan_branch_repo` is `None` in the services context when `attempt_programmatic_merge()` runs. This happens when secondary schedulers (in `execution_commands.rs`) pick up plan merge tasks without propagating the plan branch repository, and when `build_transition_service()` fails to propagate the task scheduler for post-merge scheduling.

Additionally, when merge failures occur due to missing context, the system silently returns instead of transitioning to `MergeIncomplete`, leaving tasks permanently stuck with no user-visible error.

**Reference Plan:**
- `specs/plans/fix_plan_merge_tasks_stuck_in_pending_merge.md` - Root cause analysis and fix strategy for plan_branch_repo propagation gaps

## Goals

1. Fix all `TaskSchedulerService` construction sites to include `plan_branch_repo`
2. Fix `build_transition_service()` to propagate `task_scheduler` for post-merge scheduling
3. Convert silent merge failures to `MergeIncomplete` so tasks never get permanently stuck
4. Add diagnostic logging for merge branch resolution debugging

## Dependencies

### Phase 109 (Fix Worktree-Mode Plan Merge Base Branch) - Required

| Dependency | Why Needed |
|------------|------------|
| Worktree merge fix | Plan merge flow must work correctly before fixing propagation gaps |
| MergeIncomplete state | Phase 83 introduced MergeIncomplete; this phase uses it as fallback |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_plan_merge_tasks_stuck_in_pending_merge.md`
2. Understand the root cause and propagation gaps
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
- All 3 tasks have `"blockedBy": []` — they can start immediately and in any order
- Each task modifies a different file and compiles independently

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_plan_merge_tasks_stuck_in_pending_merge.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add plan_branch_repo to all TaskSchedulerService construction sites in execution_commands.rs",
    "plan_section": "Task 1: Add plan_branch_repo to missing scheduler sites",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(execution): add plan_branch_repo to scheduler construction sites",
    "steps": [
      "Read specs/plans/fix_plan_merge_tasks_stuck_in_pending_merge.md section 'Task 1'",
      "Open src-tauri/src/commands/execution_commands.rs",
      "Find all 4 TaskSchedulerService::new(...) call sites (lines ~669, ~946, ~1030, ~1179)",
      "Add .with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo)) after each construction",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(execution): add plan_branch_repo to scheduler construction sites"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Fix build_transition_service() to propagate task_scheduler via self-reference field",
    "plan_section": "Task 2: Fix build_transition_service() to propagate scheduler",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(scheduler): propagate task_scheduler through build_transition_service",
    "steps": [
      "Read specs/plans/fix_plan_merge_tasks_stuck_in_pending_merge.md section 'Task 2'",
      "Open src-tauri/src/application/task_scheduler_service.rs",
      "Add self_ref: Option<Arc<dyn TaskScheduler>> field to TaskSchedulerService struct",
      "Add pub fn set_self_ref(&self, ...) or with_self_ref() builder method",
      "Update build_transition_service() to call .with_task_scheduler(Arc::clone(sched)) when self_ref is Some",
      "Find all Arc-wrapping construction sites (grep Arc::new.*TaskSchedulerService and Arc<dyn TaskScheduler>) in lib.rs, chat_service_send_background.rs",
      "Call set_self_ref() after wrapping in Arc at each site",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(scheduler): propagate task_scheduler through build_transition_service"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Convert silent merge failures to MergeIncomplete transitions and add diagnostic logging",
    "plan_section": "Task 3: Convert silent failures to MergeIncomplete + diagnostic logging",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(merge): convert silent merge failures to MergeIncomplete with diagnostics",
    "steps": [
      "Read specs/plans/fix_plan_merge_tasks_stuck_in_pending_merge.md section 'Task 3'",
      "Open src-tauri/src/domain/state_machine/transition_handler/side_effects.rs",
      "Add tracing::debug! at resolve_merge_branches entry with category, plan_branch_repo availability, ideation_session_id",
      "Add tracing::warn! at plan_branch_repo is None fallback when task category is plan_merge",
      "Convert empty source branch silent return (~line 916-925) to MergeIncomplete transition with error metadata",
      "Convert complete_merge_internal failure returns (~lines 999-1009, 1148-1158, 1277-1287) to MergeIncomplete fallback",
      "Upgrade warn! to error! for repos unavailable (~line 884-894) and fetch failure (~line 903-908) with more context",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(merge): convert silent merge failures to MergeIncomplete with diagnostics"
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
| **Self-reference via `Arc` for scheduler propagation** | Avoids circular dependency — scheduler creates transition services that need scheduler reference for post-merge scheduling |
| **`MergeIncomplete` as universal fallback** | Existing state with retry/resolve UI already built (Phase 99/108) — user can always retry |
| **All 3 tasks independent** | Each modifies a different file, no cross-task compilation dependencies — enables parallel execution |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] All existing tests pass
- [ ] No new warnings from clippy

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### Manual Testing
- [ ] Unstick task: `sqlite3 src-tauri/ralphx.db "UPDATE tasks SET internal_status = 'ready' WHERE id = 'ed5ed4b3-ea23-41a8-9af8-a3541c3e01e1';"`
- [ ] Resume execution → plan_merge task should go `Ready → PendingMerge → Merged`
- [ ] Logs show `resolve_merge_branches` with `plan_branch_repo_available = true`
- [ ] If merge fails for any reason, task ends in `MergeIncomplete` (not stuck in `pending_merge`)

### Wiring Verification

**For each fix, verify the full path from trigger to resolution:**

- [ ] `resume_execution` scheduler has `plan_branch_repo` → plan merge tasks resolve correctly
- [ ] `set_max_concurrent` scheduler has `plan_branch_repo` → capacity increase triggers plan merge
- [ ] `build_transition_service()` propagates `task_scheduler` → post-merge cleanup can schedule next tasks
- [ ] Silent merge failures surface as `MergeIncomplete` with error context visible in UI

**Common failure modes to check:**
- [ ] No silent returns in `attempt_programmatic_merge` — all paths either succeed or transition to error state
- [ ] `plan_branch_repo` is never `None` when a plan merge task reaches `pending_merge`

See `.claude/rules/gap-verification.md` for full verification workflow.
