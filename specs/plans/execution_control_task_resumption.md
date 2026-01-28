# Execution Control & Task Resumption

## Problem
1. Tasks in agent-active states don't resume when app restarts
2. Pause/Stop buttons don't actually prevent agent spawning
3. Running count isn't tracked accurately
4. Stop command bypasses state machine

## Solution
Full integration of execution control with agent spawning, plus startup resumption.

---

## Part 1: Wire Spawner to Execution State

### 1.1 Add ExecutionState to AgenticClientSpawner

**File:** `src-tauri/src/infrastructure/agents/spawner.rs`

```rust
pub struct AgenticClientSpawner {
    client: Arc<dyn AgenticClient>,
    working_directory: PathBuf,
    event_bus: Option<Arc<EventBus>>,
    handles: Arc<Mutex<HashMap<String, AgentHandle>>>,
    execution_state: Option<Arc<ExecutionState>>,  // NEW
}

impl AgenticClientSpawner {
    pub fn with_execution_state(mut self, state: Arc<ExecutionState>) -> Self {
        self.execution_state = Some(state);
        self
    }
}
```

### 1.2 Check can_start_task() Before Spawning

In `spawn()` method:
```rust
async fn spawn(&self, agent_type: &str, task_id: &str) {
    // Check execution state before spawning
    if let Some(ref exec) = self.execution_state {
        if !exec.can_start_task() {
            info!(task_id, "Spawn blocked: execution paused or at max concurrent");
            return;
        }
        exec.increment_running();
    }

    // ... existing spawn logic ...
}
```

### 1.3 Decrement on Agent Completion

Need to track when agents complete. Two options:
- **Option A:** Wrap spawn in async task that decrements when done
- **Option B:** Call decrement from TransitionHandler when exiting agent-active states

**Recommended: Option B** - more reliable, leverages existing state machine.

In `TransitionHandler.on_exit()` for Executing, Reviewing, ReExecuting, QaRefining, QaTesting:
```rust
fn on_exit(&mut self, state: &State) {
    match state {
        State::Executing | State::Reviewing | ... => {
            if let Some(ref exec) = self.execution_state {
                exec.decrement_running();
            }
        }
        _ => {}
    }
}
```

---

## Part 2: Fix Stop Command

### 2.1 Use TransitionHandler Instead of Direct Update

**File:** `src-tauri/src/commands/execution_commands.rs`

Current `stop_execution()` does direct DB update. Change to:

```rust
#[tauri::command]
pub async fn stop_execution(
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
) -> Result<ExecutionCommandResponse, String> {
    execution_state.pause();

    // Find executing tasks
    let executing_tasks = find_tasks_by_status(&state, InternalStatus::Executing).await?;

    // Transition each via TaskTransitionService
    for task in executing_tasks {
        let transition_service = build_transition_service(&state);
        transition_service.transition_task(&task.id, InternalStatus::Failed).await;
    }

    // running_count will be decremented by on_exit handlers

    Ok(build_response(&execution_state))
}
```

---

## Part 3: Task Resumption on Startup

### 3.1 New Module: `src-tauri/src/application/startup_jobs.rs`

```rust
const AGENT_ACTIVE_STATUSES: &[InternalStatus] = &[
    InternalStatus::Executing,
    InternalStatus::QaRefining,
    InternalStatus::QaTesting,
    InternalStatus::Reviewing,
    InternalStatus::ReExecuting,
];

pub struct StartupJobRunner {
    task_repo: Arc<dyn TaskRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    transition_service: TaskTransitionService,
    execution_state: Arc<ExecutionState>,
}

impl StartupJobRunner {
    pub async fn run(&self) {
        if self.execution_state.is_paused() {
            info!("Execution paused, skipping task resumption");
            return;
        }

        let projects = self.project_repo.get_all().await.unwrap_or_default();
        let mut resumed = 0;

        for project in projects {
            for status in AGENT_ACTIVE_STATUSES {
                let tasks = self.task_repo.get_by_status(&project.id, *status).await;
                for task in tasks.unwrap_or_default() {
                    if !self.execution_state.can_start_task() {
                        info!("Max concurrent reached, stopping resumption");
                        return;
                    }

                    info!(task_id = task.id.as_str(), status = ?status, "Resuming task");
                    self.transition_service
                        .execute_entry_actions(&task.id, &task, *status)
                        .await;
                    resumed += 1;
                }
            }
        }

        info!(count = resumed, "Task resumption complete");
    }
}
```

### 3.2 Make execute_entry_actions Public

**File:** `src-tauri/src/application/task_transition_service.rs`

Change `async fn execute_entry_actions` to `pub async fn execute_entry_actions`

### 3.3 Spawn on App Startup

**File:** `src-tauri/src/lib.rs`

In setup hook, after managing states:
```rust
// Clone what we need for the async task
let startup_app_state = Arc::clone(&app_state);
let startup_execution = Arc::clone(&execution_state);

tauri::async_runtime::spawn(async move {
    // Wait for HTTP server to be ready
    tokio::time::sleep(Duration::from_millis(500)).await;

    let runner = StartupJobRunner::new(
        startup_app_state.task_repo.clone(),
        startup_app_state.project_repo.clone(),
        build_transition_service(&startup_app_state, &startup_execution),
        startup_execution,
    );

    if let Err(e) = runner.run().await {
        error!("Startup job failed: {}", e);
    }
});
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/infrastructure/agents/spawner.rs` | Add execution_state, check can_start_task(), builder method |
| `src-tauri/src/domain/state_machine/transition_handler.rs` | Add on_exit() to decrement running count |
| `src-tauri/src/commands/execution_commands.rs` | Fix stop_execution to use TransitionHandler |
| `src-tauri/src/application/startup_jobs.rs` | **NEW** - StartupJobRunner |
| `src-tauri/src/application/mod.rs` | Export startup_jobs |
| `src-tauri/src/application/task_transition_service.rs` | Make execute_entry_actions pub |
| `src-tauri/src/lib.rs` | Wire execution_state to spawner, spawn startup job |

---

## Integration Points

### TaskTransitionService Creation

Currently builds spawner in `task_transition_service.rs:148-199`. Need to pass ExecutionState:

```rust
pub fn new(app_state: &AppState, execution_state: Arc<ExecutionState>) -> Self {
    let spawner = AgenticClientSpawner::new(client)
        .with_execution_state(Arc::clone(&execution_state))
        .with_event_bus(event_bus);
    // ...
}
```

### lib.rs Changes

Need to pass execution_state to move_task command's TaskTransitionService. Currently TaskTransitionService is built per-call in move_task - need to ensure it has access to execution_state.

---

## Verification

1. **Pause prevents new spawns:**
   - Click Pause
   - Drag task to "In Progress"
   - Verify agent doesn't spawn (check logs)

2. **Running count updates:**
   - Start task execution
   - Verify "Running: 1/2" in UI
   - Task completes → verify "Running: 0/2"

3. **Stop cancels tasks:**
   - Start 2 tasks executing
   - Click Stop
   - Verify both transition to Failed
   - Verify running count resets

4. **Resumption works:**
   - Put task in Executing status in DB
   - Restart app
   - Check logs for "Resuming task"
   - Verify agent spawns

5. **Resumption respects limits:**
   - Put 5 tasks in Executing status
   - Set max_concurrent=2
   - Restart app
   - Verify only 2 resume (logs show "Max concurrent reached")
