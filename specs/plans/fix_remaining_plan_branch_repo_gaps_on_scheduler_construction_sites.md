# Fix: Remaining `plan_branch_repo` Gaps on Scheduler Construction Sites

## Context

Phase 110 fixed 4 `TaskSchedulerService::new()` sites in `execution_commands.rs` and added `self_ref`/`build_transition_service()` propagation. It also converted silent merge failures to `MergeIncomplete` (which IS working — the task now surfaces the error instead of getting stuck in `pending_merge`).

However, plan merge task `f3a81613` (plan branch `ralphx/ralphx/plan-a5020a26`) is stuck in `merge_incomplete` with error: **"Empty source branch resolved. This typically means plan_branch_repo was unavailable when resolving merge branches for a plan_merge task."**

Phase 110 only fixed schedulers in `execution_commands.rs`. There are **6 more `TaskSchedulerService::new()` sites** across other files that create schedulers WITHOUT `.with_plan_branch_repo()`. When these schedulers later fire `try_schedule_ready_tasks()` and call `build_transition_service()`, the resulting `TaskTransitionService` has `plan_branch_repo: None`.

## Evidence

- Task ID: `f3a81613-fdd9-4a7f-afc3-4a894747ab38` (plan_merge, merge_incomplete)
- Plan branch: `ralphx/ralphx/plan-a5020a26` (exists, status=active, merge_task_id matches)
- Session: `c6340ab5-b5eb-493d-be6c-1bc7813a3eef`
- All 3 sibling tasks: `merged`
- State history: `blocked → ready` (06:42:55.829) → `pending_merge` (06:42:56.434, +600ms) → `merge_incomplete` (06:42:56.436, +2ms)
- The 600ms gap = delayed `try_schedule_ready_tasks()` from `on_enter(Ready)` side effect
- Git branch has 5 commits ahead of main — merge should be trivial

## Root Cause: The Propagation Gap

**Pattern at all 6 unfixed sites:** The `TaskTransitionService` gets `.with_plan_branch_repo()`, but the `TaskSchedulerService` passed into it does NOT. When the scheduler fires later (delayed scheduling, slot freed), `build_transition_service()` propagates `self.plan_branch_repo` which is `None`.

```
transition_service.with_plan_branch_repo(✅)     ← immediate transition works
  └── .with_task_scheduler(scheduler)             ← scheduler stored for later
        └── scheduler.plan_branch_repo = None ❌   ← scheduler has no plan_branch_repo
              └── build_transition_service()       ← builds child transition service
                    └── plan_branch_repo: None ❌   ← propagates None to child
```

## Audit: All `TaskSchedulerService::new()` Sites

| # | File | Line | `.with_plan_branch_repo()` | Status |
|---|------|------|---------------------------|--------|
| 1 | `commands/execution_commands.rs` | 656 | ✅ (Phase 110) | Fixed |
| 2 | `commands/execution_commands.rs` | 935 | ✅ (Phase 110) | Fixed |
| 3 | `commands/execution_commands.rs` | 1021 | ✅ (Phase 110) | Fixed |
| 4 | `commands/execution_commands.rs` | 1172 | ✅ (Phase 110) | Fixed |
| 5 | `chat_service/chat_service_send_background.rs` | 195 | ❌ **MISSING** | **FIX** |
| 6 | `http_server/handlers/reviews.rs` | 145 | ❌ **MISSING** | **FIX** |
| 7 | `http_server/handlers/git.rs` | 190 | ❌ **MISSING** | **FIX** |
| 8 | `commands/task_commands/mutation.rs` | 170 | ❌ **MISSING** | **FIX** |
| 9 | `commands/task_commands/mutation.rs` | 563 | ❌ **MISSING** | **FIX** |
| 10 | `commands/task_commands/mutation.rs` | 667 | ❌ **MISSING** | **FIX** |
| 11 | `chat_service/chat_service_send_background.rs` | 985 | ✅ (conditional) | OK |
| 12 | `task_scheduler_service.rs` | 341 | N/A (test code) | Skip |

**Secondary gap:** `review_commands.rs:415` creates a `TaskTransitionService` with `.with_plan_branch_repo()` but NO `.with_task_scheduler()`. This means `post_merge_cleanup()` can't trigger scheduling after a successful programmatic merge from the `HumanApprove` path.

## Immediate Workaround

The user can click **"Retry Merge"** right now — the `retry_merge` command in `git_commands.rs` uses `create_transition_service()` which DOES include `.with_plan_branch_repo()`. The merge should succeed on retry.

## Changes

### Task 1: Add `plan_branch_repo` to 6 remaining scheduler sites (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(scheduler): propagate plan_branch_repo to all scheduler construction sites`

Add `.with_plan_branch_repo(...)` to the `TaskSchedulerService` construction BEFORE `Arc::new()` wrapping at each site:

**File: `src-tauri/src/application/chat_service/chat_service_send_background.rs`**
- Line 195: Add after `TaskSchedulerService::new(...)` and before `Arc::new()`:
  ```rust
  let scheduler_concrete = TaskSchedulerService::new(...);
  scheduler_concrete = scheduler_concrete.with_plan_branch_repo(Arc::clone(&plan_branch_repo_ref));
  let scheduler_concrete = Arc::new(scheduler_concrete);
  ```
  Note: `plan_branch_repo` is available as a captured variable in this closure (line ~130).

**File: `src-tauri/src/http_server/handlers/reviews.rs`**
- Line 145: Add `.with_plan_branch_repo(Arc::clone(&state.app_state.plan_branch_repo))` between `TaskSchedulerService::new(...)` and `Arc::new()`.

**File: `src-tauri/src/http_server/handlers/git.rs`**
- Line 190: Add `.with_plan_branch_repo(Arc::clone(&state.app_state.plan_branch_repo))` between `TaskSchedulerService::new(...)` and `Arc::new()`.

**File: `src-tauri/src/commands/task_commands/mutation.rs`**
- Lines 170, 563, 667: Add `.with_plan_branch_repo(Arc::clone(&state.plan_branch_repo))` between `TaskSchedulerService::new(...)` and `Arc::new()`.

Each site currently wraps in `Arc::new(TaskSchedulerService::new(...))` — restructure to:
```rust
let scheduler_concrete = TaskSchedulerService::new(...);
let scheduler_concrete = scheduler_concrete.with_plan_branch_repo(Arc::clone(&...));
let scheduler_concrete = Arc::new(scheduler_concrete);
scheduler_concrete.set_self_ref(Arc::clone(&scheduler_concrete) as Arc<dyn TaskScheduler>);
```

### Task 2: Add `task_scheduler` to `approve_task_for_review`
**Dependencies:** None (independent of Task 1)
**Atomic Commit:** `fix(review): add scheduler to approve_task_for_review for post-merge scheduling`

**File: `src-tauri/src/commands/review_commands.rs`**
- Line 415-429: Create a scheduler and add `.with_task_scheduler()` to the transition service.

Current:
```rust
let transition_service = TaskTransitionService::new(...)
    .with_plan_branch_repo(Arc::clone(&state.plan_branch_repo));
```

Fix:
```rust
let scheduler_concrete = Arc::new(TaskSchedulerService::new(
    Arc::clone(&execution_state),
    Arc::clone(&state.project_repo),
    // ... all repos ...
    Some(app.clone()),
).with_plan_branch_repo(Arc::clone(&state.plan_branch_repo)));
scheduler_concrete.set_self_ref(Arc::clone(&scheduler_concrete) as Arc<dyn TaskScheduler>);
let task_scheduler: Arc<dyn TaskScheduler> = scheduler_concrete;

let transition_service = TaskTransitionService::new(...)
    .with_task_scheduler(task_scheduler)
    .with_plan_branch_repo(Arc::clone(&state.plan_branch_repo));
```

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/application/chat_service/chat_service_send_background.rs` | Add `.with_plan_branch_repo()` to scheduler at line 195 |
| `src-tauri/src/http_server/handlers/reviews.rs` | Add `.with_plan_branch_repo()` to scheduler at line 145 |
| `src-tauri/src/http_server/handlers/git.rs` | Add `.with_plan_branch_repo()` to scheduler at line 190 |
| `src-tauri/src/commands/task_commands/mutation.rs` | Add `.with_plan_branch_repo()` to schedulers at lines 170, 563, 667 |
| `src-tauri/src/commands/review_commands.rs` | Add scheduler + `.with_task_scheduler()` to approve path at line 415 |

## Verification

1. **Build:** `cargo check` in `src-tauri/`
2. **Lint:** `cargo clippy --all-targets --all-features -- -D warnings`
3. **Test:** `cargo test`
4. **Unstick task:** `sqlite3 src-tauri/ralphx.db "UPDATE tasks SET internal_status = 'ready' WHERE id = 'f3a81613-fdd9-4a7f-afc3-4a894747ab38';"`
5. **Runtime:** Resume execution → plan_merge task should go `Ready → PendingMerge → Merged`
6. **Regression:** Verify no `TaskSchedulerService::new()` site (outside tests) lacks `.with_plan_branch_repo()`:
   ```
   grep -n "TaskSchedulerService::new" src-tauri/src/**/*.rs | grep -v test | grep -v "_tests.rs"
   ```
   Then verify each has `.with_plan_branch_repo()` chained before `Arc::new()`.

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
- Task 1 and Task 2 are independent and can be executed in parallel (no shared files)
