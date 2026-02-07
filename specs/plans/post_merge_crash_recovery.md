# Plan: Post-Merge Crash Recovery for Task Dependencies

## Context

When a task reaches `Merged` state but the app crashes before `on_enter(Merged)` completes, dependent tasks that were `Blocked` should be unblocked on restart. Additionally, tasks stuck mid-merge (`PendingMerge`) need recovery.

**Good news:** A safety net already exists — `startup_jobs.rs::unblock_ready_tasks()` scans all `Blocked` tasks on startup and checks if their blockers are in terminal states. This DOES handle the `Merged` crash scenario.

**However, there are two bugs to fix and one gap to close:**

## Changes

### Task 1: Fix `Approved` inconsistency in `all_blockers_complete()` + test (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(startup): remove Approved from blocker terminal states`

> **Compilation unit note:** Tasks 1 & 2 from the original plan are merged. Removing `Approved` from the match arm changes the semantics — the existing test `test_blocked_task_unblocked_when_blocker_is_approved` would fail. Both changes must land atomically.

**Files:**
- `src-tauri/src/application/startup_jobs.rs:456-474` — Remove `InternalStatus::Approved` from match + update doc comment to list only `Merged, Failed, Cancelled`
- `src-tauri/src/application/startup_jobs/tests.rs:649-686` — Rename to `test_blocked_task_remains_blocked_when_blocker_is_approved`, add `execution_state.pause()` to isolate unblock logic from auto-transition recovery, assert task stays `Blocked`

### Task 2: Add `PendingMerge` to `AUTO_TRANSITION_STATES` + test
**Dependencies:** None (independent of Task 1)
**Atomic Commit:** `fix(startup): add PendingMerge crash recovery`

> **Compilation unit note:** The `AUTO_TRANSITION_STATES` constant change and new test are in separate files (`execution_commands.rs` and `startup_jobs/tests.rs`), but the test validates the constant's effect. They must land together.

**Files:**
- `src-tauri/src/commands/execution_commands.rs:41-46` — Add `InternalStatus::PendingMerge` with comment `// attempt_programmatic_merge() (→ Merged or → Merging)`
- `src-tauri/src/application/startup_jobs/tests.rs` (new test) — Test that a task stuck in `PendingMerge` has its entry actions re-triggered on startup (follows pattern of `test_approved_auto_transitions_on_startup`)

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/application/startup_jobs.rs` | Remove `Approved` from `all_blockers_complete()` + update doc comment |
| `src-tauri/src/application/startup_jobs/tests.rs` | Fix `Approved` test assertion + add `PendingMerge` recovery test |
| `src-tauri/src/commands/execution_commands.rs` | Add `PendingMerge` to `AUTO_TRANSITION_STATES` |

## Verification

1. `cargo test -p ralphx-lib -- startup_jobs` — all startup tests pass
2. `cargo test -p ralphx-lib -- execution_commands` — constant assertions pass
3. `cargo clippy --all-targets --all-features -- -D warnings` — no warnings

## Not in scope

- `MergeIncomplete`/`MergeConflict` recovery — these are human-intervention states with no `on_enter` handler; their stuck state IS the correct behavior (user decides next step)
- Cascade unblocking — when Task B is unblocked (Blocked→Ready), Task C blocked by Task B correctly stays blocked until Task B reaches Merged

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
