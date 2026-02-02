# Plan: Fix Task Crash Recovery Gaps

## Root Cause

Two gaps in crash recovery:

### Gap 1: Auto-Transition States Not Recovered

Task crashed **AFTER** entering a state with auto-transition but **BEFORE** the auto-transition completed.

| State | In AGENT_ACTIVE_STATUSES | Auto-transitions to | Spawns Agent |
|-------|--------------------------|---------------------|--------------|
| `QaPassed` | ❌ No | `PendingReview` | No (chains) |
| `PendingReview` | ❌ No | `Reviewing` | Reviewer |
| `RevisionNeeded` | ❌ No | `ReExecuting` | Worker |
| `Approved` | ❌ No | `PendingMerge` | No (programmatic) |

### Gap 2: Merging State Missing from Agent-Active

`Merging` spawns a merger agent (side_effects.rs:465-478) but is NOT in `AGENT_ACTIVE_STATUSES`.

| State | Spawns Agent | In AGENT_ACTIVE_STATUSES |
|-------|--------------|--------------------------|
| `Merging` | ✅ Merger | ❌ No (BUG) |

### What's Already Covered

| Category | States | Recovery |
|----------|--------|----------|
| Agent-Active | `Executing`, `QaRefining`, `QaTesting`, `Reviewing`, `ReExecuting` | ✅ `AGENT_ACTIVE_STATUSES` |
| Scheduler | `Ready` | ✅ `try_schedule_ready_tasks()` |
| Human-Waiting | `ReviewPassed`, `Escalated`, `MergeConflict`, `QaFailed` | ✅ No action needed |
| Terminal | `Merged`, `Failed`, `Cancelled` | ✅ No action needed |

## Solution

Two changes:
1. Add `Merging` to `AGENT_ACTIVE_STATUSES` (spawns merger agent)
2. Create `AUTO_TRANSITION_STATES` for states with auto-transitions

## Implementation Plan

### Task 1: Add Merging to AGENT_ACTIVE_STATUSES (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(startup): add Merging to AGENT_ACTIVE_STATUSES for crash recovery`

**File:** `src-tauri/src/commands/execution_commands.rs`

```rust
pub const AGENT_ACTIVE_STATUSES: &[InternalStatus] = &[
    InternalStatus::Executing,
    InternalStatus::QaRefining,
    InternalStatus::QaTesting,
    InternalStatus::Reviewing,
    InternalStatus::ReExecuting,
    InternalStatus::Merging,  // ADD THIS - spawns merger agent
];
```

### Task 2: Add AUTO_TRANSITION_STATES constant (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(startup): add AUTO_TRANSITION_STATES constant for recovery`

**File:** `src-tauri/src/commands/execution_commands.rs`

All auto-transitions from `check_auto_transition()` at `transition_handler/mod.rs:155-176`:

| State | Target | Spawns Agent? | Include? |
|-------|--------|---------------|----------|
| `QaPassed` | → `PendingReview` | No | Yes (chains to PendingReview) |
| `PendingReview` | → `Reviewing` | ✅ Reviewer | Yes |
| `RevisionNeeded` | → `ReExecuting` | ✅ Worker | Yes |
| `Approved` | → `PendingMerge` | No | Yes (triggers programmatic merge) |

```rust
/// States that have automatic transitions on entry.
/// Tasks stuck in these states on startup should have their entry actions
/// re-triggered to complete the auto-transition.
pub const AUTO_TRANSITION_STATES: &[InternalStatus] = &[
    InternalStatus::QaPassed,         // → PendingReview
    InternalStatus::PendingReview,    // → Reviewing (spawns reviewer)
    InternalStatus::RevisionNeeded,   // → ReExecuting (spawns worker)
    InternalStatus::Approved,         // → PendingMerge (programmatic merge)
];
```

### Task 3: Update StartupJobRunner to handle auto-transition states (BLOCKING)
**Dependencies:** Task 2
**Atomic Commit:** `feat(startup): recover tasks stuck in auto-transition states`

**File:** `src-tauri/src/application/startup_jobs.rs`

After the `AGENT_ACTIVE_STATUSES` loop (around line 158), add a second loop for auto-transition states:

```rust
// Re-trigger auto-transition states that may have been interrupted mid-transition
// These states have on_enter side effects that trigger auto-transitions to spawn agents
use crate::commands::execution_commands::AUTO_TRANSITION_STATES;

for status in AUTO_TRANSITION_STATES {
    let tasks = match self.task_repo.get_by_status(&project.id, *status).await {
        Ok(tasks) => tasks,
        Err(e) => {
            tracing::warn!(
                project_id = project.id.as_str(),
                status = ?status,
                error = %e,
                "Failed to get tasks by status for auto-transition"
            );
            continue;
        }
    };

    eprintln!("[STARTUP] Found {} tasks in {:?} status (auto-transition)", tasks.len(), status);
    for task in tasks {
        // Check max_concurrent before triggering (auto-transitions may spawn agents)
        if !self.execution_state.can_start_task() {
            info!(
                max_concurrent = self.execution_state.max_concurrent(),
                running_count = self.execution_state.running_count(),
                "Max concurrent reached, stopping auto-transition recovery"
            );
            return;
        }

        eprintln!("[STARTUP] Re-triggering auto-transition for task: {} ({})", task.id.as_str(), task.title);
        info!(
            task_id = task.id.as_str(),
            status = ?status,
            "Re-triggering auto-transition for stuck task"
        );

        // Re-execute entry actions - this will trigger check_auto_transition()
        self.transition_service
            .execute_entry_actions(&task.id, &task, *status)
            .await;
    }
}
```

### Task 4: Add tests
**Dependencies:** Task 1, Task 2, Task 3
**Atomic Commit:** `test(startup): add crash recovery tests for auto-transition states`

**File:** `src-tauri/src/application/startup_jobs.rs` (tests module)

Test that:
- Tasks in `Merging` get resumed (agent respawned) on startup
- Tasks in `PendingReview` get auto-transitioned to `Reviewing` on startup
- Tasks in `RevisionNeeded` get auto-transitioned to `ReExecuting` on startup
- Tasks in `Approved` get auto-transitioned to `PendingMerge` on startup

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/commands/execution_commands.rs` | Add `Merging` to `AGENT_ACTIVE_STATUSES` |
| `src-tauri/src/commands/execution_commands.rs` | Add `AUTO_TRANSITION_STATES` constant |
| `src-tauri/src/application/startup_jobs.rs` | Add loop for auto-transition states |
| `src-tauri/src/application/startup_jobs.rs` | Add tests for new recovery cases |

## Verification

### Test 1: Merging State Recovery
1. Get a task to `Merging` state (e.g., via merge conflict)
2. Kill the app
3. Restart app
4. Verify terminal: `[STARTUP] Found 1 tasks in Merging status`
5. Verify merger agent respawns

### Test 2: PendingReview Recovery
1. Set a task to `PendingReview` directly in DB:
   ```bash
   sqlite3 src-tauri/ralphx.db "UPDATE tasks SET internal_status='pending_review' WHERE id='<task-id>';"
   ```
2. Restart app
3. Verify terminal: `[STARTUP] Re-triggering auto-transition for task: <id>`
4. Verify task moves to `Reviewing` and reviewer agent spawns

### Test 3: Approved Recovery (Merge Workflow)
1. Set a task to `Approved` directly in DB
2. Restart app
3. Verify auto-transition to `PendingMerge` triggers
4. Verify programmatic merge attempt runs

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Task Dependency Graph

```
Task 1 (AGENT_ACTIVE_STATUSES) ──┐
                                 ├──► Task 4 (Tests)
Task 2 (AUTO_TRANSITION_STATES) ─┼──► Task 3 (StartupJobRunner) ──► Task 4
                                 │
```

### Compilation Unit Analysis

| Task | Adds New Symbol | Uses Existing | Compiles Alone |
|------|-----------------|---------------|----------------|
| 1 | `Merging` to const | No | ✅ Yes |
| 2 | `AUTO_TRANSITION_STATES` | No | ✅ Yes |
| 3 | Recovery loop | `AUTO_TRANSITION_STATES` | ✅ Yes (after Task 2) |
| 4 | Test functions | All above | ✅ Yes (after Tasks 1-3) |

All tasks are additive and form valid compilation units.
