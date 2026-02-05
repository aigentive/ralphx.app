# Fix: Blocked Tasks Incorrectly Promoted on Merge Failure

## Bug Summary

When a task enters `merge_conflict` (a failure state), its dependent blocked tasks are incorrectly promoted to `ready` and picked up by the execution queue.

## Root Cause

**Two coupled issues in the dependency unblocking logic:**

### Issue 1: Premature `unblock_dependents()` call

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs:624-638`

```rust
State::Approved => {
    // Emit task completed event
    self.machine.context.services.event_emitter
        .emit("task_completed", &self.machine.context.task_id).await;
    // Unblock dependent tasks     <-- BUG: Called BEFORE merge!
    self.machine.context.services.dependency_manager
        .unblock_dependents(&self.machine.context.task_id).await;
}
```

`Approved` always auto-transitions to `PendingMerge` (mod.rs:172-176). The `on_enter(Approved)` runs first, calling `unblock_dependents()`, then the auto-transition fires. If the merge later fails, dependents were already unblocked.

### Issue 2: `is_blocker_complete()` treats `Approved` as terminal

**File:** `src-tauri/src/application/task_transition_service.rs:127-137`

```rust
async fn is_blocker_complete(&self, blocker_id: &TaskId) -> bool {
    if let Ok(Some(task)) = self.task_repo.get_by_id(blocker_id).await {
        matches!(
            task.internal_status,
            InternalStatus::Approved | InternalStatus::Merged | InternalStatus::Failed | InternalStatus::Cancelled
            //  ^^^^^^^^ BUG: Approved is NOT terminal since Phase 66
        )
    } else {
        true
    }
}
```

`Approved` is in the "complete" list, so when `unblock_dependents()` checks whether all blockers are done, it sees `Approved` as complete.

## Execution Flow (Current - Buggy)

```
1. Task A (blocker) transitions to Approved
2. on_enter(Approved): unblock_dependents() called
   → checks Task A status: Approved → is_blocker_complete = TRUE
   → Task B (dependent) promoted: Blocked → Ready  ← BUG
3. check_auto_transition: Approved → PendingMerge
4. on_enter(PendingMerge): attempt_programmatic_merge()
5. Merge fails → merging → merge_conflict
6. Task B is already Ready, gets picked up by queue!
```

## Fix

**Compilation Unit Analysis:** All 3 changes + test update form a single compilation unit. Change 1 removes the `unblock_dependents()` call, which means Test 4 (asserting it IS called) would fail if applied separately. Changes 2-3 are logically coupled to Change 1. All changes MUST be one atomic commit.

### Change 1: Remove `unblock_dependents()` from `on_enter(Approved)` (BLOCKING)

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs:624-638`

```rust
State::Approved => {
    // Emit task completed event
    self.machine.context.services.event_emitter
        .emit("task_completed", &self.machine.context.task_id).await;
    // REMOVED: unblock_dependents() - Approved auto-transitions to PendingMerge,
    // unblocking should only happen at Merged (after successful merge)
}
```

**Reasoning:** `Approved` is no longer terminal. It always auto-transitions to `PendingMerge` (Phase 66). The `on_enter(Merged)` handler at line 687-695 already calls `unblock_dependents()`, which is the correct place.

### Change 2: Remove `Approved` from `is_blocker_complete()` (BLOCKING)

**File:** `src-tauri/src/application/task_transition_service.rs:129-132`

```rust
// Before:
matches!(
    task.internal_status,
    InternalStatus::Approved | InternalStatus::Merged | InternalStatus::Failed | InternalStatus::Cancelled
)

// After:
matches!(
    task.internal_status,
    InternalStatus::Merged | InternalStatus::Failed | InternalStatus::Cancelled
)
```

**Also update the mirror at line 149-152** (`get_incomplete_blocker_names`):
```rust
// Before:
if !matches!(
    task.internal_status,
    InternalStatus::Approved | InternalStatus::Merged | InternalStatus::Failed | InternalStatus::Cancelled
)

// After:
if !matches!(
    task.internal_status,
    InternalStatus::Merged | InternalStatus::Failed | InternalStatus::Cancelled
)
```

**Reasoning:** `Approved` is an intermediate state. Only `Merged` means the task's work is actually on main and safe to depend on.

### Change 3: Update comment on `is_blocker_complete()`

**File:** `src-tauri/src/application/task_transition_service.rs:125-126`

```rust
// Before:
/// Check if a blocking task is complete (Approved, Merged, Failed, or Cancelled).

// After:
/// Check if a blocking task is complete (Merged, Failed, or Cancelled).
/// Note: Approved is NOT complete - task still needs to merge successfully.
```

## Execution Flow (Fixed)

```
1. Task A (blocker) transitions to Approved
2. on_enter(Approved): emit task_completed event (NO unblocking)
3. check_auto_transition: Approved → PendingMerge
4. on_enter(PendingMerge): attempt_programmatic_merge()

Path A - Merge succeeds:
5a. on_enter(Merged): unblock_dependents() called
    → checks Task A status: Merged → is_blocker_complete = TRUE
    → Task B promoted: Blocked → Ready ✓

Path B - Merge fails:
5b. merging → merge_conflict
    → NO unblock_dependents() called
    → Task B remains Blocked ✓
```

## Existing Correct Paths (Verify No Regression)

| Path | Where `unblock_dependents()` is called | Status |
|------|----------------------------------------|--------|
| `on_enter(Merged)` | side_effects.rs:687-695 | Correct: merge complete |
| Programmatic merge success | side_effects.rs:849-855 | Correct: merge complete |
| `attempt_merge_auto_complete` success | chat_service_send_background.rs:956-966 | Correct: merge complete |

All three paths only trigger AFTER successful merge. No other paths call `unblock_dependents()` for merge-related states.

## Tests

### Test 1: Unit test - Approved should not unblock dependents

```rust
#[tokio::test]
async fn test_approved_does_not_unblock_dependents() {
    // Setup: Task A blocks Task B
    // Transition Task A to Approved
    // Assert: dep_manager.unblock_dependents NOT called
    // Assert: Task B still Blocked
}
```

### Test 2: Unit test - Merged should unblock dependents

```rust
#[tokio::test]
async fn test_merged_unblocks_dependents() {
    // Setup: Task A blocks Task B
    // Transition Task A through full flow to Merged
    // Assert: dep_manager.unblock_dependents called
    // Assert: Task B promoted to Ready
}
```

### Test 3: Unit test - is_blocker_complete rejects Approved

```rust
#[tokio::test]
async fn test_approved_is_not_blocker_complete() {
    // Setup: Task in Approved status
    // Assert: is_blocker_complete returns false
}
```

### Test 4: Update existing test

**File:** `src-tauri/src/domain/state_machine/transition_handler/tests.rs:324-337`

The existing test at line 324 asserts that `unblock_dependents` IS called when entering `Approved`. This test needs to be updated to assert it is NOT called:

```rust
// Before (line 336):
assert!(calls.iter().any(|c| c.method == "unblock_dependents"));

// After:
assert!(!calls.iter().any(|c| c.method == "unblock_dependents"),
    "unblock_dependents should NOT be called at Approved - only at Merged");
```

## Task: Fix dependency unblocking to require Merged (not Approved)

**Dependencies:** None
**Atomic Commit:** `fix(state_machine): defer dependency unblocking until Merged, not Approved`

All changes below form a single compilation unit and MUST be committed together.

### Files to Modify

| File | Change | Lines |
|------|--------|-------|
| `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` | Remove `unblock_dependents` from `on_enter(Approved)` | 632-638 |
| `src-tauri/src/application/task_transition_service.rs` | Remove `Approved` from `is_blocker_complete()` and `get_incomplete_blocker_names()`, update doc comment | 125-132, 149-152 |
| `src-tauri/src/domain/state_machine/transition_handler/tests.rs` | Update existing test to assert NO unblock at Approved | ~334-336 |

### Verification

1. `cargo test` - all tests pass (including updated test)
2. `cargo clippy --all-targets --all-features -- -D warnings` - no warnings
3. Manual test: Create two tasks with dependency, execute blocker task, verify dependent stays `Blocked` through `Approved` → `PendingMerge`, only promoted to `Ready` after `Merged`

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
