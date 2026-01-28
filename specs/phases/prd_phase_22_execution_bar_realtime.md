# RalphX - Phase 22: Execution Bar Real-time Updates

## Overview

This phase implements real-time UI updates for the ExecutionControlBar. Currently, the execution status (running count, paused state, queued count) only updates via 5-second polling. This creates a laggy user experience where starting/stopping tasks or pausing execution doesn't immediately reflect in the UI.

After this phase, execution state changes emit Tauri events that the frontend listens for, providing instant UI feedback.

**Reference Plan:**
- `specs/plans/execution_bar_realtime_updates.md` - Detailed implementation plan with event schemas and integration points

## Goals

1. Emit `execution:status_changed` events when running count increments/decrements
2. Emit events when pause/resume/stop commands execute
3. Emit `execution:queue_changed` events when tasks move to/from Ready status
4. Add frontend event listeners that update UI store immediately
5. Reduce polling frequency from 5s to 30s (fallback only)

## Dependencies

### Phase 21 (Execution Control & Task Resumption) - Required

| Dependency | Why Needed |
|------------|------------|
| ExecutionState wired to spawner | Accurate increment_running() calls at spawn time |
| Decrement in on_exit() | Accurate decrement_running() calls when tasks complete |
| Running count tracking | Backend must track counts correctly before emitting events |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/execution_bar_realtime_updates.md`
2. Understand the event schemas and emission points
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run `npm run lint && npm run typecheck` and `cargo clippy --all-targets --all-features -- -D warnings`
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/execution_bar_realtime_updates.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "backend",
    "description": "Add emit_status_changed helper to ExecutionState",
    "plan_section": "Part 1: Backend Event Emission - 1.1",
    "steps": [
      "Read specs/plans/execution_bar_realtime_updates.md section 'Part 1'",
      "Update src-tauri/src/commands/execution_commands.rs:",
      "  - Add emit_status_changed<R: Runtime>(&self, handle: &AppHandle<R>, reason: &str) method",
      "  - Emit 'execution:status_changed' event with isPaused, runningCount, maxConcurrent, reason, timestamp",
      "  - Use serde_json::json! macro for payload",
      "Write unit test for emit helper",
      "Run cargo test",
      "Commit: feat(execution): add emit_status_changed helper"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add AppHandle to AgenticClientSpawner",
    "plan_section": "Part 1: Backend Event Emission - 1.4",
    "steps": [
      "Read specs/plans/execution_bar_realtime_updates.md section '1.4'",
      "Update src-tauri/src/infrastructure/agents/spawner.rs:",
      "  - Add app_handle: Option<AppHandle<tauri::Wry>> field to struct",
      "  - Add with_app_handle(mut self, handle: AppHandle) -> Self builder method",
      "  - Update new() to initialize app_handle: None",
      "Run cargo test",
      "Commit: feat(spawner): add AppHandle field for event emission"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Emit event on running count increment",
    "plan_section": "Part 1: Backend Event Emission - 1.2",
    "steps": [
      "Read specs/plans/execution_bar_realtime_updates.md section '1.2'",
      "Update src-tauri/src/infrastructure/agents/spawner.rs spawn() method:",
      "  - After exec.increment_running()",
      "  - If app_handle is Some, call exec.emit_status_changed(handle, 'task_started')",
      "Write test: test_spawn_emits_status_changed_event",
      "Run cargo test",
      "Commit: feat(spawner): emit status_changed on task start"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Pass AppHandle to TransitionHandler for decrement emission",
    "plan_section": "Part 1: Backend Event Emission - 1.4",
    "steps": [
      "Determine how TransitionHandler can access AppHandle:",
      "  - Option A: Add to TaskServices struct",
      "  - Option B: Add to TaskContext",
      "  - Option C: Pass through TransitionHandler constructor",
      "Update TaskServices or TaskContext to hold Option<AppHandle>",
      "Update TaskTransitionService::new() to pass app_handle",
      "Run cargo test",
      "Commit: feat(transition): add AppHandle access for event emission"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Emit event on running count decrement",
    "plan_section": "Part 1: Backend Event Emission - 1.2",
    "steps": [
      "Read specs/plans/execution_bar_realtime_updates.md section '1.2'",
      "Update src-tauri/src/domain/state_machine/transition_handler.rs on_exit():",
      "  - After exec.decrement_running()",
      "  - If app_handle is Some, call exec.emit_status_changed(handle, 'task_completed')",
      "Write test: test_on_exit_emits_status_changed_event",
      "Run cargo test",
      "Commit: feat(transition): emit status_changed on task complete"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Emit events on pause/resume/stop commands",
    "plan_section": "Part 1: Backend Event Emission - 1.3",
    "steps": [
      "Read specs/plans/execution_bar_realtime_updates.md section '1.3'",
      "Update src-tauri/src/commands/execution_commands.rs:",
      "  - In pause_execution: emit with reason 'paused'",
      "  - In resume_execution: emit with reason 'resumed'",
      "  - In stop_execution: emit with reason 'stopped'",
      "  - Get app_handle from Tauri state or parameter",
      "Write tests for each command's event emission",
      "Run cargo test",
      "Commit: feat(execution): emit events on pause/resume/stop"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Emit queue_changed event on task move affecting Ready status",
    "plan_section": "Part 3: Queued Count Updates",
    "steps": [
      "Read specs/plans/execution_bar_realtime_updates.md section 'Part 3'",
      "Update src-tauri/src/commands/task_commands.rs move_task:",
      "  - After successful move, check if old_status or new_status is Ready",
      "  - If so, count tasks in Ready status for the project",
      "  - Emit 'execution:queue_changed' with queuedCount",
      "Add helper function: count_tasks_in_ready_status()",
      "Write test for queue_changed emission",
      "Run cargo test",
      "Commit: feat(tasks): emit queue_changed on Ready status changes"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create useExecutionEvents hook",
    "plan_section": "Part 2: Frontend Event Listening - 2.1",
    "steps": [
      "Read specs/plans/execution_bar_realtime_updates.md section 'Part 2'",
      "Create src/hooks/useExecutionEvents.ts:",
      "  - Import listen from @tauri-apps/api/event",
      "  - Define ExecutionStatusEvent interface",
      "  - Listen for 'execution:status_changed' event",
      "  - Update uiStore.executionStatus on event",
      "  - Clean up listener on unmount",
      "Export from src/hooks/index.ts",
      "Run npm run typecheck",
      "Commit: feat(hooks): add useExecutionEvents for real-time updates"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add queue_changed listener to useExecutionEvents",
    "plan_section": "Part 3: Queued Count Updates - 3.2",
    "steps": [
      "Read specs/plans/execution_bar_realtime_updates.md section '3.2'",
      "Update src/hooks/useExecutionEvents.ts:",
      "  - Add second listener for 'execution:queue_changed' event",
      "  - Update only queuedCount in executionStatus",
      "  - Clean up both listeners on unmount",
      "Run npm run typecheck",
      "Commit: feat(hooks): add queue_changed listener"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Wire useExecutionEvents in App.tsx",
    "plan_section": "Part 2: Frontend Event Listening - 2.2",
    "steps": [
      "Read specs/plans/execution_bar_realtime_updates.md section '2.2'",
      "Update src/App.tsx:",
      "  - Import useExecutionEvents from @/hooks",
      "  - Call useExecutionEvents() in App component (near other event hooks)",
      "Run npm run typecheck",
      "Commit: feat(app): wire useExecutionEvents for real-time status"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Reduce polling frequency to 30 seconds",
    "plan_section": "Part 2: Frontend Event Listening - 2.3",
    "steps": [
      "Read specs/plans/execution_bar_realtime_updates.md section '2.3'",
      "Update src/hooks/useExecutionControl.ts:",
      "  - Change refetchInterval from 5000 to 30000",
      "  - Add comment explaining this is fallback only",
      "Run npm run typecheck",
      "Commit: feat(hooks): reduce polling to 30s (events provide real-time)"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Write tests for useExecutionEvents hook",
    "plan_section": "Verification",
    "steps": [
      "Create src/hooks/useExecutionEvents.test.ts:",
      "  - Mock @tauri-apps/api/event listen function",
      "  - Test: status_changed event updates store",
      "  - Test: queue_changed event updates queuedCount only",
      "  - Test: listeners cleaned up on unmount",
      "Run npm run test:run",
      "Commit: test(hooks): add useExecutionEvents tests"
    ],
    "passes": false
  }
]
```

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Tauri events over WebSocket** | Tauri's built-in event system is simpler and already available |
| **Separate status/queue events** | Queue changes are less frequent, keep payloads focused |
| **Reason field in events** | Helps debugging and could enable different UI animations |
| **30s fallback polling** | Events may be missed (race conditions), polling ensures eventual consistency |
| **AppHandle in spawner/services** | Required for emit; builder pattern maintains backward compatibility |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] emit_status_changed emits correct payload
- [ ] Spawn emits task_started event
- [ ] on_exit emits task_completed event
- [ ] pause_execution emits paused event
- [ ] resume_execution emits resumed event
- [ ] stop_execution emits stopped event
- [ ] move_task emits queue_changed when affecting Ready status

### Frontend - Run `npm run test`
- [ ] useExecutionEvents updates store on status_changed
- [ ] useExecutionEvents updates queuedCount on queue_changed
- [ ] Listeners cleaned up on unmount

### Build Verification
- [ ] `npm run lint` passes
- [ ] `npm run typecheck` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `npm run build` succeeds
- [ ] `cargo build --release` succeeds

### Integration Testing
- [ ] Start task → "Running: 1/2" appears immediately (< 100ms)
- [ ] Task completes → "Running: 0/2" appears immediately
- [ ] Click Pause → button changes to Resume immediately
- [ ] Click Stop → running count drops to 0 immediately
- [ ] Drag task to Ready → Queued count increments immediately
- [ ] Move task from Ready → Queued count decrements immediately
- [ ] Disconnect events (dev tools) → status still updates within 30s (polling fallback)
