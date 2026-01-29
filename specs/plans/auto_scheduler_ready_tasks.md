# Auto-Scheduler for Ready Tasks

## Goal
When a task enters Ready status, automatically start execution if capacity allows (`can_start_task()` returns true). Also trigger scheduling when a running task completes and frees up a slot.

## Trigger Points
1. **Task enters Ready** - Check if slot available, start immediately
2. **Running task completes** - Slot freed, pick next Ready task
3. **App startup** - Already handled by `StartupJobRunner` for agent-active states; extend to Ready
4. **Unpause/increase max_concurrent** - Pick up waiting Ready tasks

## Implementation

### 1. Add `try_schedule_ready_tasks()` to TransitionHandler

**File:** `src-tauri/src/domain/state_machine/transition_handler/mod.rs`

Add a new method that:
- Checks `can_start_task()`
- Fetches Ready tasks across all projects (oldest first)
- Transitions first Ready task to Executing via state machine event

```rust
async fn try_schedule_ready_tasks(&self) {
    let Some(ref exec) = self.machine.context.services.execution_state else { return };
    if !exec.can_start_task() { return; }

    // Get all projects, find oldest Ready task
    // Trigger StartExecution event (not direct status change)
}
```

### 2. Call from `on_exit()` when slot frees

**File:** `src-tauri/src/domain/state_machine/transition_handler/mod.rs` (line ~117)

After `decrement_running()` in `on_exit()`:
```rust
State::Executing | State::QaRefining | ... => {
    if let Some(ref exec) = ... {
        exec.decrement_running();

        // NEW: Try to schedule next Ready task
        self.try_schedule_ready_tasks().await;

        // Emit status change
        ...
    }
}
```

### 3. Call from `on_enter(Ready)` when task becomes Ready

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`

Add entry action for Ready state:
```rust
State::Ready => {
    // Task just entered Ready - try to execute if slot available
    self.try_schedule_ready_tasks().await;
}
```

### 4. Add to startup resumption

**File:** `src-tauri/src/application/startup_jobs.rs`

After resuming agent-active tasks, also check for Ready tasks:
```rust
// After existing resumption loop
self.try_schedule_ready_tasks().await;
```

### 5. Handle unpause/max_concurrent changes

**File:** `src-tauri/src/commands/execution_commands.rs`

In `resume_execution` and `set_max_concurrent` commands, trigger scheduling:
```rust
// After resume/increase
try_schedule_ready_tasks(state, execution_state, app).await;
```

## Key Files to Modify

1. `src-tauri/src/domain/state_machine/transition_handler/mod.rs` - Add `try_schedule_ready_tasks()`, call from `on_exit()`
2. `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` - Add `on_enter(Ready)` handler
3. `src-tauri/src/application/startup_jobs.rs` - Extend to schedule Ready tasks on startup
4. `src-tauri/src/commands/execution_commands.rs` - Trigger scheduling on unpause/max change

## Challenges

1. **Circular dependency risk**: TransitionHandler needs to transition another task while handling a transition. Solution: Spawn async task to avoid blocking.

2. **Cross-project scheduling**: Need to iterate all projects to find Ready tasks. May add a global query method.

3. **Race conditions**: Multiple tasks completing simultaneously. Solution: Use atomic check-and-schedule pattern.

## Verification

1. Create task in Draft, drag to Ready → Should auto-start if slots available
2. Set max_concurrent=1, queue 3 tasks → Should execute sequentially
3. Pause, queue tasks, unpause → Should pick up queued tasks
4. Restart app with Ready tasks → Should auto-start on startup
