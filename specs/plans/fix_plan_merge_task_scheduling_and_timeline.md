# Fix: Plan Merge Task Not Auto-Scheduled + Empty Timeline

## Context

After all tasks in a plan/ideation session merge, the auto-created plan merge task transitions to `Ready` but is never picked up by the scheduler. It only works after app restart. Additionally, the plan merge task's timeline shows no events.

**Root cause (both bugs):** `unblock_dependents()` in `RepoBackedDependencyManager` performs a raw DB update from `Blocked` → `Ready`, bypassing the state machine entirely. This means:
1. `on_enter(Ready)` never fires → no `try_schedule_ready_tasks()` → task sits idle
2. `persist_status_change()` is never called → transition invisible to timeline

Compounding this, there are **3 code paths** that call `unblock_dependents()` after a merge, but only 1 (the state machine `on_enter(Merged)` path) has a subsequent `try_schedule_ready_tasks()` call. The other 2 paths bypass the state machine entirely:
- `post_merge_cleanup()` — programmatic merge success (the most common path)
- `attempt_merge_auto_complete()` — agent auto-complete merge detection

## Changes

### 1. Add `persist_status_change()` in `unblock_dependents()` (fixes timeline) (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(task-transition): record timeline entry when unblocking dependents`

**File:** `src-tauri/src/application/task_transition_service.rs:214`

After the successful `task_repo.update()` at line 211, add:

```rust
// Record state transition history for timeline
if let Err(e) = self.task_repo.persist_status_change(
    &dependent_id,
    InternalStatus::Blocked,
    InternalStatus::Ready,
    "blockers_resolved",
).await {
    tracing::warn!(error = %e, task_id = %dependent_id, "Failed to record unblock transition (non-fatal)");
}
```

This fixes timeline for ALL tasks unblocked via the dependency system, not just plan merge tasks.

### 2. Add `try_schedule_ready_tasks()` in `post_merge_cleanup()` (fixes scheduling - programmatic merge path)
**Dependencies:** None
**Atomic Commit:** `fix(merge): schedule unblocked tasks after programmatic merge`

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs:1297`

After the `unblock_dependents()` call at line 1296, add (same pattern as `on_enter(Merged)` at lines 792-798):

```rust
if let Some(ref scheduler) = self.machine.context.services.task_scheduler {
    let scheduler = Arc::clone(scheduler);
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
        scheduler.try_schedule_ready_tasks().await;
    });
}
```

### 3. Add `try_schedule_ready_tasks()` in `attempt_merge_auto_complete()` (fixes scheduling - agent auto-complete path)
**Dependencies:** None
**Atomic Commit:** `fix(merge): schedule unblocked tasks after agent auto-complete merge`

**File:** `src-tauri/src/application/chat_service/chat_service_send_background.rs:980`

After the `unblock_dependents()` call at line 980, construct a `TaskSchedulerService` and trigger scheduling. All required parameters are already available in the function signature (lines 633-646):

```rust
// Schedule newly-unblocked tasks (e.g. plan_merge tasks that just became Ready)
let scheduler = TaskSchedulerService::new(
    Arc::clone(execution_state),
    Arc::clone(project_repo),
    Arc::clone(task_repo),
    Arc::clone(task_dependency_repo),
    Arc::clone(chat_message_repo),
    Arc::clone(conversation_repo),
    Arc::clone(agent_run_repo),
    Arc::clone(ideation_session_repo),
    Arc::clone(activity_event_repo),
    Arc::clone(message_queue),
    Arc::clone(running_agent_registry),
    app_handle.cloned(),
);
let scheduler = if let Some(ref repo) = plan_branch_repo {
    scheduler.with_plan_branch_repo(Arc::clone(repo))
} else {
    scheduler
};
let scheduler = Arc::new(scheduler);
tokio::spawn(async move {
    tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
    scheduler.try_schedule_ready_tasks().await;
});
```

Add import at top of file if not present: `use crate::application::task_scheduler_service::TaskSchedulerService;`

## Files Modified

| File | Change |
|------|--------|
| `src-tauri/src/application/task_transition_service.rs` | Add `persist_status_change()` in `unblock_dependents()` |
| `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` | Add scheduler trigger in `post_merge_cleanup()` |
| `src-tauri/src/application/chat_service/chat_service_send_background.rs` | Add scheduler construction + trigger in `attempt_merge_auto_complete()` |

## Verification

1. `cargo clippy --all-targets --all-features -- -D warnings` — must pass
2. `cargo test` — must pass
3. **Manual test:** Create ideation session with 2+ tasks in a plan with feature branches enabled → execute all tasks → let them merge → verify the plan merge task is automatically picked up and merges the feature branch into main without restart
4. **Timeline check:** After the plan merge task completes, open its detail view → verify timeline shows `Blocked → Ready → PendingMerge → Merged` transitions

## Compilation Unit Analysis

All 3 changes are **purely additive** — inserting new code after existing lines in separate files. No renames, no signature changes, no removed exports. Each change compiles independently and can be committed separately or together.

| Change | Files | Additive? | Compiles Alone? |
|--------|-------|-----------|-----------------|
| 1 | `task_transition_service.rs` | ✅ | ✅ (uses existing `persist_status_change` API) |
| 2 | `side_effects.rs` | ✅ | ✅ (uses existing `task_scheduler` pattern) |
| 3 | `chat_service_send_background.rs` | ✅ | ✅ (import already present, all params available) |

**Recommendation:** Commit all 3 together as a single atomic fix since they address the same root cause.

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
