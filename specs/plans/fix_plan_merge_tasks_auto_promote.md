# Fix: Plan Merge Tasks Auto-Promote to Merge Workflow

## Context

When all tasks in a plan complete (reach `Merged`), the auto-created plan merge task gets unblocked (Blocked → Ready) but then sits idle. It should automatically proceed to the merge workflow (`PendingMerge` → programmatic merge attempt). Two root causes:

1. **`unblock_dependents()` bypasses the state machine** — does a direct DB update to `Ready` without calling `on_enter(Ready)`, so `try_schedule_ready_tasks()` is never triggered for newly-unblocked tasks.
2. **Scheduler has no plan_merge awareness** — it always routes Ready tasks to `Executing`, but plan_merge tasks should skip execution entirely and go to `PendingMerge`.

## Changes

### 1. Trigger scheduling after unblocking dependents (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(state-machine): trigger scheduling after unblock_dependents in on_enter(Merged)`

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` (~line 774)

In `on_enter(Merged)`, after `unblock_dependents()`, add a call to `self.try_schedule_ready_tasks().await`. This ensures any tasks that just became Ready get picked up by the scheduler.

```rust
State::Merged => {
    self.machine.context.services.dependency_manager
        .unblock_dependents(&self.machine.context.task_id).await;
    // NEW: Schedule newly-unblocked tasks
    self.try_schedule_ready_tasks().await;
}
```

### 2. Add PendingMerge to Ready's valid transitions (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(status): allow Ready → PendingMerge transition for plan_merge tasks`

**File:** `src-tauri/src/domain/entities/status.rs` (~line 74)

```rust
Ready => &[Executing, Blocked, Cancelled, PendingMerge],
```

This allows the scheduler to transition plan_merge tasks directly from Ready to PendingMerge.

### 3. Route plan_merge tasks to PendingMerge in scheduler
**Dependencies:** Task 1, Task 2
**Atomic Commit:** `fix(scheduler): route plan_merge tasks to PendingMerge instead of Executing`

**File:** `src-tauri/src/application/task_scheduler_service.rs` (~line 275-282)

After finding a schedulable task, check its category. If `plan_merge`, transition to `PendingMerge` instead of `Executing`:

```rust
let target_status = if task.category == "plan_merge" {
    tracing::info!(
        task_id = task.id.as_str(),
        "Plan merge task: routing to PendingMerge (skip execution)"
    );
    InternalStatus::PendingMerge
} else {
    InternalStatus::Executing
};

if let Err(e) = transition_service
    .transition_task(&task.id, target_status)
    .await
{ ... }
```

Note: `PendingMerge` entry triggers `attempt_programmatic_merge()` which correctly resolves merge branches for plan merge tasks via `resolve_merge_branches()` (uses `plan_branches.merge_task_id` to find the feature branch).

## Compilation Unit Analysis

**All three changes are a single compilation unit.** They can be applied independently without breaking compilation:

- Change 1 (add `try_schedule_ready_tasks()` call): Compiles alone — just adds a method call that already exists.
- Change 2 (add `PendingMerge` to valid transitions): Compiles alone — just adds a variant to an array.
- Change 3 (scheduler routing): Compiles alone — just adds an `if` check using existing `InternalStatus::PendingMerge`.

However, changes 1 and 2 are prerequisites for change 3 to be *functionally correct*:
- Without change 1, the scheduler is never invoked after unblocking.
- Without change 2, the transition to `PendingMerge` would fail validation.

**Recommendation:** Apply all three as a single commit since the fix is only meaningful with all three together.

## Flow After Fix

```
Last plan task → Merged
  → on_enter(Merged): unblock_dependents() → merge task: Blocked → Ready (DB)
  → on_enter(Merged): try_schedule_ready_tasks()
    → scheduler finds merge task (Ready, category=plan_merge)
    → transition to PendingMerge (not Executing)
      → attempt_programmatic_merge()
        → resolve_merge_branches: feature branch → main
        → Success → Merged | Conflict → Merging (agent)
```

## Startup Recovery Analysis (No Additional Changes Needed)

The existing startup path already covers stuck plan_merge tasks:

| Scenario | Recovery Mechanism | After Fix? |
|----------|-------------------|------------|
| App crash while merge task is `Blocked` (blockers done) | `unblock_ready_tasks()` → sets to Ready | Scheduler picks up via fix #3 |
| App crash while merge task is `Ready` (already unblocked) | `try_schedule_ready_tasks()` at end of startup (line 340) | Scheduler routes to PendingMerge via fix #3 |
| App crash while merge task is `PendingMerge` | `AUTO_TRANSITION_STATES` recovery re-executes `on_enter(PendingMerge)` → `attempt_programmatic_merge()` | Already works |
| App crash while merge task is `Merging` (agent) | `AGENT_ACTIVE_STATUSES` recovery re-spawns merger agent | Already works |
| **Pre-fix stuck tasks** (currently in Ready) | Next `try_schedule_ready_tasks()` call (startup or task completion) | Auto-resolved by fix #3 |

## Verification

1. `cargo clippy --all-targets --all-features -- -D warnings`
2. `cargo test` — existing + new tests pass
3. Manual test: create a plan with feature branch, complete all tasks, verify merge task auto-proceeds to merge workflow

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
