# Fix: Plan Merge Tasks Stuck in `pending_merge`

## Context

Plan merge task `ed5ed4b3` (session `d2b097e2`) is stuck in `pending_merge`. Both sibling tasks merged successfully, the git branch `ralphx/ralphx/plan-d2b097e2` exists, and the plan_branch DB record has the correct `merge_task_id`. The merge should succeed but `attempt_programmatic_merge()` silently returns.

## Root Cause

**`plan_branch_repo` is `None` in the services context when `attempt_programmatic_merge()` runs.**

Evidence:
- Task transition history: `blocked → ready → pending_merge` (system event)
- All data correct in DB (plan_branch exists, merge_task_id linked)
- Git branch exists on disk
- Only explanation for silent return: `resolve_merge_branches()` gets `plan_branch_repo = None` → returns `("", "main")` → empty source check → silent return at line 923

**How `plan_branch_repo` becomes `None`:**

1. Sibling task merges via programmatic fast path
2. `post_merge_cleanup` unblocks plan_merge task → now `Ready`
3. `post_merge_cleanup` tries to schedule via `services.task_scheduler` — but it's **`None`** because `TaskSchedulerService::build_transition_service()` (line 214) never calls `.with_task_scheduler()` on the transition service it creates
4. Plan_merge task stays in `Ready` with no scheduling
5. Eventually a different scheduler picks it up (e.g. `resume_execution`, settings change)
6. Those schedulers in `execution_commands.rs` are constructed **without `.with_plan_branch_repo()`**
7. Scheduler transitions plan_merge to `PendingMerge` → `attempt_programmatic_merge()` runs with `plan_branch_repo = None` → silent failure

**Why intermittent:** When the last sibling goes through the agent merge path (`chat_service_send_background.rs:983`), that path creates a scheduler WITH `plan_branch_repo` → works. Only the programmatic-merge + secondary-scheduler path hits the bug.

## Changes

### Task 1: Add `plan_branch_repo` to missing scheduler sites (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(execution): add plan_branch_repo to scheduler construction sites`

**File:** `src-tauri/src/commands/execution_commands.rs`

Add `.with_plan_branch_repo(Arc::clone(&app_state.plan_branch_repo))` after each `TaskSchedulerService::new(...)`:

| Line | Command | Currently Missing |
|------|---------|-------------------|
| 669 | `resume_execution` | `.with_plan_branch_repo()` |
| 946 | `set_max_concurrent` | `.with_plan_branch_repo()` |
| 1030 | `update_execution_settings` | `.with_plan_branch_repo()` |
| 1179 | (another scheduler site) | `.with_plan_branch_repo()` |

**Compilation unit:** Additive only — `.with_plan_branch_repo()` already exists on `TaskSchedulerService` (line 100). No signature changes.

### Task 2: Fix `build_transition_service()` to propagate scheduler
**Dependencies:** None
**Atomic Commit:** `fix(scheduler): propagate task_scheduler through build_transition_service`

**File:** `src-tauri/src/application/task_scheduler_service.rs`

`build_transition_service()` (line 214-237) builds a `TaskTransitionService` but never calls `.with_task_scheduler()`. This means `on_enter(Merged)` → `post_merge_cleanup` → scheduling is silently skipped because `services.task_scheduler` is `None`.

**Fix:** Add an `Arc<dyn TaskScheduler>` self-reference field to `TaskSchedulerService`. Set it after wrapping in `Arc` at construction sites. Use it in `build_transition_service()`:

```rust
fn build_transition_service(&self) -> TaskTransitionService<R> {
    let mut service = TaskTransitionService::new(...);
    if let Some(ref repo) = self.plan_branch_repo {
        service = service.with_plan_branch_repo(Arc::clone(repo));
    }
    if let Some(ref sched) = self.self_ref {
        service = service.with_task_scheduler(Arc::clone(sched));
    }
    service
}
```

**Compilation unit:** Self-contained within one file. `self_ref` is set via a builder method at existing `Arc`-wrapping sites (e.g. `lib.rs:267`). Those call sites must also be updated in this task — find all sites that wrap `TaskSchedulerService` in `Arc` and call `set_self_ref()` (or equivalent).

**Callers to update:** Grep for `Arc::new(.*TaskSchedulerService` and `Arc<dyn TaskScheduler>` construction. Key sites: `lib.rs`, `chat_service_send_background.rs`.

### Task 3: Convert silent failures to `MergeIncomplete` + diagnostic logging
**Dependencies:** None
**Atomic Commit:** `fix(merge): convert silent merge failures to MergeIncomplete with diagnostics`

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`

**Silent failures → `MergeIncomplete`:**

| Location | Current Behavior | Fix |
|----------|-----------------|-----|
| Line 916-925 (empty source branch) | Silent `return` | Transition to `MergeIncomplete` with error metadata |
| Lines 999-1009 (`complete_merge_internal` failure) | Log error, return | Fall back to `MergeIncomplete` |
| Lines 1148-1158 (same, worktree path) | Log error, return | Fall back to `MergeIncomplete` |
| Lines 1277-1287 (same, local mode) | Log error, return | Fall back to `MergeIncomplete` |
| Line 884-894 (repos unavailable) | `warn!` + return | `error!` with context |
| Line 903-908 (fetch failure) | `warn!` + return | `error!` with context |

**Diagnostic logging in `resolve_merge_branches`:**

- At entry (line 267): `tracing::debug!` with category, plan_branch_repo availability, ideation_session_id
- At `plan_branch_repo is None` fallback (line 275-276): `tracing::warn!` when category is `plan_merge`

**Compilation unit:** All changes in one file. Transitioning to `MergeIncomplete` uses existing `TaskEvent::MergeAgentError` which is already in the valid transitions table (`pending_merge` → `merge_incomplete`). No new types or signatures needed.

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/commands/execution_commands.rs` | Add `with_plan_branch_repo` to 4 scheduler sites |
| `src-tauri/src/application/task_scheduler_service.rs` | Add `self_ref` field; use it in `build_transition_service` |
| `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` | Silent failures → `MergeIncomplete`; diagnostic logging |

## Verification

1. **Unstick current task:** `sqlite3 src-tauri/ralphx.db "UPDATE tasks SET internal_status = 'ready' WHERE id = 'ed5ed4b3-ea23-41a8-9af8-a3541c3e01e1';"`
2. **Build:** `cargo check`
3. **Lint:** `cargo clippy --all-targets --all-features -- -D warnings`
4. **Test:** `cargo test`
5. **Runtime:** Resume execution → plan_merge task should go `Ready → PendingMerge → Merged`
6. **Logs:** Verify `resolve_merge_branches` shows `plan_branch_repo_available = true`
7. **Regression:** If merge fails for any reason, task should be in `MergeIncomplete` (not stuck in `pending_merge`)

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Execution Order

Tasks 1, 2, and 3 are **independent** — no cross-task compilation dependencies. They can be executed in any order or in parallel. Each modifies a different file and compiles independently.

| Task | File | Blocks | Blocked By |
|------|------|--------|------------|
| Task 1 | `execution_commands.rs` | — | — |
| Task 2 | `task_scheduler_service.rs` (+caller sites) | — | — |
| Task 3 | `side_effects.rs` | — | — |
