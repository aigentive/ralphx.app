# RalphX - Phase 111: Fix Remaining plan_branch_repo Gaps on Scheduler Sites

## Overview

Phase 110 fixed `plan_branch_repo` propagation for 4 `TaskSchedulerService::new()` sites in `execution_commands.rs`. However, 6 additional scheduler construction sites across other files still create schedulers WITHOUT `.with_plan_branch_repo()`. When these schedulers fire delayed scheduling via `build_transition_service()`, the resulting `TaskTransitionService` has `plan_branch_repo: None`, causing plan merge tasks to fail with "Empty source branch resolved."

Additionally, `review_commands.rs:approve_task_for_review` creates a `TaskTransitionService` with `.with_plan_branch_repo()` but without a `TaskScheduler`, meaning `post_merge_cleanup()` can't trigger scheduling after programmatic merge from the HumanApprove path.

**Reference Plan:**
- `specs/plans/fix_remaining_plan_branch_repo_gaps_on_scheduler_construction_sites.md` - Full audit, root cause, and code changes

## Goals

1. Add `.with_plan_branch_repo()` to all 6 remaining `TaskSchedulerService::new()` sites
2. Add `TaskScheduler` to the `approve_task_for_review` transition service
3. Ensure no `TaskSchedulerService::new()` site (outside tests) lacks `.with_plan_branch_repo()`

## Dependencies

### Phase 110 (Fix Plan Merge Tasks Stuck in pending_merge) - Required

| Dependency | Why Needed |
|------------|------------|
| `TaskSchedulerService::with_plan_branch_repo()` method | Phase 110 added this method; Phase 111 calls it at 6 more sites |
| `build_transition_service()` propagation | Phase 110 wired propagation logic; Phase 111 ensures all callers provide the repo |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_remaining_plan_branch_repo_gaps_on_scheduler_construction_sites.md`
2. Understand the propagation gap pattern (scheduler → build_transition_service → child transition_service)
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Run linters for modified code only (backend: `cargo clippy`)
4. Commit with descriptive message

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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_remaining_plan_branch_repo_gaps_on_scheduler_construction_sites.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add plan_branch_repo to 6 remaining TaskSchedulerService::new() sites",
    "plan_section": "Task 1: Add plan_branch_repo to 6 remaining scheduler sites",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(scheduler): propagate plan_branch_repo to all scheduler construction sites",
    "steps": [
      "Read specs/plans/fix_remaining_plan_branch_repo_gaps_on_scheduler_construction_sites.md section 'Task 1'",
      "In chat_service_send_background.rs (~line 195): Extract scheduler from Arc::new(), add .with_plan_branch_repo() using if-let pattern (plan_branch_repo is Option), re-wrap in Arc",
      "In http_server/handlers/reviews.rs (~line 145): Add .with_plan_branch_repo(Arc::clone(&state.app_state.plan_branch_repo)) before Arc::new()",
      "In http_server/handlers/git.rs (~line 190): Add .with_plan_branch_repo(Arc::clone(&state.app_state.plan_branch_repo)) before Arc::new()",
      "In commands/task_commands/mutation.rs (~lines 170, 563, 667): Add .with_plan_branch_repo(Arc::clone(&state.plan_branch_repo)) before Arc::new() at all 3 sites",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test in src-tauri/",
      "Verify: grep all TaskSchedulerService::new sites outside tests have .with_plan_branch_repo()",
      "Commit: fix(scheduler): propagate plan_branch_repo to all scheduler construction sites"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add TaskScheduler to approve_task_for_review transition service for post-merge scheduling",
    "plan_section": "Task 2: Add task_scheduler to approve_task_for_review",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(review): add scheduler to approve_task_for_review for post-merge scheduling",
    "steps": [
      "Read specs/plans/fix_remaining_plan_branch_repo_gaps_on_scheduler_construction_sites.md section 'Task 2'",
      "In commands/review_commands.rs (~line 415): Create TaskSchedulerService with all repos from state + execution_state",
      "Chain .with_plan_branch_repo(Arc::clone(&state.plan_branch_repo)) before Arc::new()",
      "Call set_self_ref() and cast to Arc<dyn TaskScheduler>",
      "Add .with_task_scheduler(task_scheduler) to the existing TaskTransitionService",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test in src-tauri/",
      "Commit: fix(review): add scheduler to approve_task_for_review for post-merge scheduling"
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
| **Fix scheduler, not transition_service** | The transition_service already gets plan_branch_repo at most sites, but the scheduler stored inside it does NOT — the scheduler's `build_transition_service()` creates child services with `None` |
| **Both tasks are independent** | Task 1 (scheduler sites) and Task 2 (review approve path) touch completely different files and can be executed in parallel |
| **Use if-let pattern for Option** | In `chat_service_send_background.rs`, plan_branch_repo is `Option<Arc<dyn PlanBranchRepository>>` — must conditionally unwrap |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] All existing tests pass
- [ ] No new test failures introduced

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### Regression Check
- [ ] No `TaskSchedulerService::new()` site (outside tests) lacks `.with_plan_branch_repo()`:
  ```
  grep -rn "TaskSchedulerService::new" src-tauri/src/ | grep -v test | grep -v "_tests.rs"
  ```
  Then verify each has `.with_plan_branch_repo()` chained before `Arc::new()`.

### Manual Testing
- [ ] Reset stuck task: `sqlite3 src-tauri/ralphx.db "UPDATE tasks SET internal_status = 'ready' WHERE id = 'f3a81613-fdd9-4a7f-afc3-4a894747ab38';"`
- [ ] Plan merge task flows: `Ready → PendingMerge → Merged` (trivial merge, 5 commits ahead)
- [ ] HumanApprove path: approve a task → auto-transition through `Approved → PendingMerge → Merged` works with scheduling

### Wiring Verification

**For each modified scheduler site, verify the full propagation path:**

- [ ] Scheduler has `.with_plan_branch_repo()` before `Arc::new()`
- [ ] Scheduler has `set_self_ref()` after `Arc::new()`
- [ ] Transition service receives scheduler via `.with_task_scheduler()`
- [ ] Transition service also receives plan_branch_repo via `.with_plan_branch_repo()`

See `.claude/rules/gap-verification.md` for full verification workflow.
