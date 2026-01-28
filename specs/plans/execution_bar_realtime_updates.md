# Execution Bar Real-time Updates

## Problem

The ExecutionControlBar UI doesn't update in real-time:
- Running count changes (task starts/completes) only visible after 5-second poll
- Pause/Resume state changes require polling to reflect
- Stop action doesn't immediately update UI counts
- No visual feedback when execution state changes

## Dependencies

- **Phase 21 (Execution Control & Task Resumption)** - Must be complete first
  - Provides accurate running count tracking via ExecutionState
  - Provides increment/decrement at correct points (spawn/state exit)

## Solution

Emit Tauri events when execution state changes, frontend listens and updates immediately.

---

## Part 1: Backend Event Emission

### 1.1 Create ExecutionState Event Helper

**File:** `src-tauri/src/commands/execution_commands.rs`

Add emit helper that takes AppHandle:

```rust
impl ExecutionState {
    /// Emit execution:status_changed event with current state
    pub fn emit_status_changed<R: Runtime>(&self, handle: &AppHandle<R>, reason: &str) {
        let _ = handle.emit(
            "execution:status_changed",
            serde_json::json!({
                "isPaused": self.is_paused(),
                "runningCount": self.running_count(),
                "maxConcurrent": self.max_concurrent(),
                "reason": reason,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }
}
```

### 1.2 Emit on Increment/Decrement

**File:** `src-tauri/src/infrastructure/agents/spawner.rs`

After incrementing in spawn():
```rust
exec.increment_running();
if let Some(ref handle) = self.app_handle {
    exec.emit_status_changed(handle, "task_started");
}
```

**File:** `src-tauri/src/domain/state_machine/transition_handler.rs`

After decrementing in on_exit():
```rust
exec.decrement_running();
if let Some(ref handle) = self.app_handle {
    exec.emit_status_changed(handle, "task_completed");
}
```

### 1.3 Emit on Pause/Resume/Stop

**File:** `src-tauri/src/commands/execution_commands.rs`

Update pause_execution:
```rust
execution_state.pause();
if let Some(ref handle) = app_state.app_handle {
    execution_state.emit_status_changed(handle, "paused");
}
```

Same pattern for resume_execution ("resumed") and stop_execution ("stopped").

### 1.4 Pass AppHandle to Components

Need to ensure:
- `AgenticClientSpawner` has access to `AppHandle` (add field + builder method)
- `TransitionHandler` has access to `AppHandle` (via TaskServices or direct)
- Commands already have access via Tauri State

---

## Part 2: Frontend Event Listening

### 2.1 Add Event Listener Hook

**File:** `src/hooks/useExecutionEvents.ts`

```typescript
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useUiStore } from "@/stores/uiStore";

interface ExecutionStatusEvent {
  isPaused: boolean;
  runningCount: number;
  maxConcurrent: number;
  reason: string;
  timestamp: string;
}

export function useExecutionEvents() {
  const setExecutionStatus = useUiStore((state) => state.setExecutionStatus);

  useEffect(() => {
    const unlisten = listen<ExecutionStatusEvent>(
      "execution:status_changed",
      (event) => {
        setExecutionStatus({
          isPaused: event.payload.isPaused,
          runningCount: event.payload.runningCount,
          maxConcurrent: event.payload.maxConcurrent,
          queuedCount: 0, // Will be updated by next poll or separate event
          canStartTask: !event.payload.isPaused &&
            event.payload.runningCount < event.payload.maxConcurrent,
        });
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setExecutionStatus]);
}
```

### 2.2 Wire Hook in App

**File:** `src/App.tsx`

Add to App component:
```typescript
// Real-time execution status updates
useExecutionEvents();
```

### 2.3 Reduce Polling Frequency (Optional)

Since events provide real-time updates, can reduce polling from 5s to 30s as a fallback:

**File:** `src/hooks/useExecutionControl.ts`

```typescript
refetchInterval: 30000, // Fallback poll every 30s instead of 5s
```

---

## Part 3: Queued Count Updates

### 3.1 Emit When Queue Changes

Queued count (tasks in Ready status) changes when:
- Task moved to Ready column
- Task moved out of Ready column
- Task created with Ready status

**Option A:** Emit from move_task command when target is Ready or source was Ready
**Option B:** Include queuedCount in execution:status_changed (requires DB query)

**Recommended: Option A** - More efficient, emit when queue changes

**File:** `src-tauri/src/commands/task_commands.rs` (move_task)

```rust
// After successful move
if old_status == InternalStatus::Ready || new_status == InternalStatus::Ready {
    // Emit queue changed event
    let queued_count = count_tasks_in_ready_status(&app_state, &task.project_id).await?;
    let _ = handle.emit("execution:queue_changed", json!({ "queuedCount": queued_count }));
}
```

### 3.2 Listen for Queue Events

**File:** `src/hooks/useExecutionEvents.ts`

Add second listener:
```typescript
listen<{ queuedCount: number }>("execution:queue_changed", (event) => {
  setExecutionStatus((prev) => ({
    ...prev,
    queuedCount: event.payload.queuedCount,
  }));
});
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/commands/execution_commands.rs` | Add emit_status_changed helper, emit on pause/resume/stop |
| `src-tauri/src/infrastructure/agents/spawner.rs` | Add app_handle field, emit on increment |
| `src-tauri/src/domain/state_machine/transition_handler.rs` | Emit on decrement |
| `src-tauri/src/commands/task_commands.rs` | Emit queue_changed on move affecting Ready |
| `src/hooks/useExecutionEvents.ts` | **NEW** - Event listeners for execution status |
| `src/hooks/useExecutionControl.ts` | Reduce polling frequency |
| `src/App.tsx` | Wire useExecutionEvents hook |

---

## Event Schema

### execution:status_changed
```json
{
  "isPaused": boolean,
  "runningCount": number,
  "maxConcurrent": number,
  "reason": "task_started" | "task_completed" | "paused" | "resumed" | "stopped",
  "timestamp": "ISO8601"
}
```

### execution:queue_changed
```json
{
  "queuedCount": number,
  "timestamp": "ISO8601"
}
```

---

## Verification

1. **Real-time running count:**
   - Start task execution
   - Verify "Running: 1/2" appears immediately (no 5s delay)
   - Task completes → verify "Running: 0/2" immediately

2. **Real-time pause/resume:**
   - Click Pause
   - Verify button changes to Resume immediately
   - Status indicator turns yellow immediately

3. **Real-time stop:**
   - Start 2 tasks
   - Click Stop
   - Verify running count drops to 0 immediately

4. **Queue updates:**
   - Drag task to Ready column
   - Verify Queued count increments immediately
   - Move task out of Ready
   - Verify Queued count decrements immediately

5. **Fallback polling:**
   - Disconnect event listener (dev tools)
   - Verify status still updates (within 30s)
