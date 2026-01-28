# RalphX - Phase 21: Execution Control & Task Resumption

## Overview

This phase implements full integration between the execution control system (pause/resume/stop) and agent spawning, plus automatic task resumption on app startup. Currently, the pause button doesn't actually prevent agents from spawning, the stop command bypasses the state machine with direct DB updates, running counts aren't tracked accurately, and tasks in agent-active states don't resume when the app restarts.

**Reference Plan:**
- `specs/plans/execution_control_task_resumption.md` - Detailed implementation plan with code snippets and integration points

## Goals

1. Wire ExecutionState to AgenticClientSpawner so pause actually prevents spawning
2. Track running count accurately by decrementing on state exit
3. Fix stop command to use TransitionHandler instead of direct DB updates
4. Implement StartupJobRunner to resume tasks in agent-active states on app restart
5. Respect max_concurrent limits during both normal operation and startup resumption

## Dependencies

### Phase 20 (Review System) - Required

| Dependency | Why Needed |
|------------|------------|
| TransitionHandler with on_exit() | Decrement running count when exiting agent-active states |
| TaskTransitionService | Used by stop command and startup resumption |
| Review states (Reviewing, ReExecuting) | Agent-active states that need resumption |
| ChatService integration | Entry actions spawn agents via ChatService |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/execution_control_task_resumption.md`
2. Understand the integration points between ExecutionState and the spawner
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
2. **Read the ENTIRE implementation plan** at `specs/plans/execution_control_task_resumption.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "backend",
    "description": "Add ExecutionState field to AgenticClientSpawner",
    "plan_section": "Part 1: Wire Spawner to Execution State - 1.1",
    "steps": [
      "Read specs/plans/execution_control_task_resumption.md section 'Part 1'",
      "Update src-tauri/src/infrastructure/agents/spawner.rs:",
      "  - Add execution_state: Option<Arc<ExecutionState>> field to struct",
      "  - Add with_execution_state(mut self, state: Arc<ExecutionState>) -> Self builder method",
      "  - Update new() to initialize execution_state: None",
      "Run cargo test",
      "Commit: feat(spawner): add ExecutionState field and builder"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Check can_start_task() before spawning agents",
    "plan_section": "Part 1: Wire Spawner to Execution State - 1.2",
    "steps": [
      "Read specs/plans/execution_control_task_resumption.md section '1.2'",
      "Update src-tauri/src/infrastructure/agents/spawner.rs spawn() method:",
      "  - At start of spawn(), check if let Some(ref exec) = self.execution_state",
      "  - If !exec.can_start_task(), log info and return early (don't spawn)",
      "  - If can start, call exec.increment_running() before spawning",
      "Update spawn_background() with same check",
      "Write test: test_spawn_blocked_when_paused",
      "Write test: test_spawn_blocked_at_max_concurrent",
      "Run cargo test",
      "Commit: feat(spawner): check can_start_task before spawning"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add ExecutionState to TransitionHandler for decrement on exit",
    "plan_section": "Part 1: Wire Spawner to Execution State - 1.3",
    "steps": [
      "Read specs/plans/execution_control_task_resumption.md section '1.3'",
      "Update src-tauri/src/domain/state_machine/transition_handler.rs:",
      "  - TransitionHandler needs access to ExecutionState (via TaskContext or services)",
      "  - Option: Add execution_state field to TaskServices",
      "  - Update on_exit() to decrement running count for agent-active states:",
      "    - Executing, QaRefining, QaTesting, Reviewing, ReExecuting",
      "  - Use pattern: if let Some(ref exec) = self.execution_state { exec.decrement_running(); }",
      "Write test for decrement on exit",
      "Run cargo test",
      "Commit: feat(transition): decrement running count on agent-active state exit"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Fix stop_execution to use TransitionHandler",
    "plan_section": "Part 2: Fix Stop Command",
    "steps": [
      "Read specs/plans/execution_control_task_resumption.md section 'Part 2'",
      "Update src-tauri/src/commands/execution_commands.rs stop_execution():",
      "  - Remove direct task.internal_status = InternalStatus::Failed assignment",
      "  - Find all tasks in agent-active states (Executing, QaRefining, QaTesting, Reviewing, ReExecuting)",
      "  - For each task, use TaskTransitionService.transition_task() to Failed",
      "  - Remove manual running count reset (on_exit handlers will decrement)",
      "  - Keep execution_state.pause() call at start",
      "Write integration test for stop cancelling multiple tasks",
      "Run cargo test",
      "Commit: fix(execution): use TransitionHandler for stop command"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Create StartupJobRunner module",
    "plan_section": "Part 3: Task Resumption on Startup - 3.1",
    "steps": [
      "Read specs/plans/execution_control_task_resumption.md section 'Part 3'",
      "Create src-tauri/src/application/startup_jobs.rs:",
      "  - Define AGENT_ACTIVE_STATUSES constant array",
      "  - Create StartupJobRunner struct with task_repo, project_repo, transition_service, execution_state",
      "  - Implement new() constructor",
      "  - Implement run() method:",
      "    - Check is_paused() first, skip if paused",
      "    - Iterate all projects",
      "    - For each project, find tasks in agent-active statuses",
      "    - For each task, check can_start_task() before resuming",
      "    - Call execute_entry_actions() to re-trigger agent spawn",
      "    - Log resumption count",
      "Update src-tauri/src/application/mod.rs to export startup_jobs",
      "Run cargo test",
      "Commit: feat(startup): add StartupJobRunner module"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Make execute_entry_actions public",
    "plan_section": "Part 3: Task Resumption on Startup - 3.2",
    "steps": [
      "Read specs/plans/execution_control_task_resumption.md section '3.2'",
      "Update src-tauri/src/application/task_transition_service.rs:",
      "  - Change 'async fn execute_entry_actions' to 'pub async fn execute_entry_actions'",
      "  - Verify method signature is suitable for external use",
      "Run cargo test",
      "Commit: refactor(transition): make execute_entry_actions public"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Update TaskTransitionService to accept ExecutionState",
    "plan_section": "Integration Points - TaskTransitionService Creation",
    "steps": [
      "Read specs/plans/execution_control_task_resumption.md section 'Integration Points'",
      "Update src-tauri/src/application/task_transition_service.rs new():",
      "  - Add execution_state: Arc<ExecutionState> parameter",
      "  - Pass execution_state to AgenticClientSpawner via .with_execution_state()",
      "  - Store execution_state for passing to TransitionHandler",
      "Update all call sites of TaskTransitionService::new() to pass execution_state",
      "Run cargo test",
      "Commit: feat(transition): wire ExecutionState to TaskTransitionService"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Wire startup job runner in lib.rs setup",
    "plan_section": "Part 3: Task Resumption on Startup - 3.3",
    "steps": [
      "Read specs/plans/execution_control_task_resumption.md section '3.3'",
      "Update src-tauri/src/lib.rs setup hook:",
      "  - After managing app_state and execution_state",
      "  - Clone Arc references needed for async task",
      "  - Spawn async task with tauri::async_runtime::spawn",
      "  - Add 500ms delay to wait for HTTP server",
      "  - Create StartupJobRunner with cloned dependencies",
      "  - Call runner.run().await",
      "  - Log errors if startup job fails",
      "Run cargo build",
      "Commit: feat(startup): wire StartupJobRunner in app setup"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Add get_by_status method to TaskRepository",
    "plan_section": "Part 3: Task Resumption on Startup",
    "steps": [
      "Check if TaskRepository trait has get_by_status method",
      "If not, add to src-tauri/src/domain/repositories/task_repository.rs:",
      "  - async fn get_by_status(&self, project_id: &ProjectId, status: InternalStatus) -> AppResult<Vec<Task>>",
      "Implement in SQLite repository (sqlite_task_repo.rs)",
      "Implement in Memory repository (memory_task_repo.rs)",
      "Write tests for new method",
      "Run cargo test",
      "Commit: feat(repo): add get_by_status to TaskRepository"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Write integration tests for execution control",
    "plan_section": "Verification",
    "steps": [
      "Create tests in src-tauri/src/commands/execution_commands.rs:",
      "  - test_pause_prevents_new_spawns: pause -> move task -> verify no spawn",
      "  - test_running_count_increments: start task -> verify count increases",
      "  - test_running_count_decrements: complete task -> verify count decreases",
      "  - test_stop_cancels_all_executing: start 2 tasks -> stop -> verify both Failed",
      "  - test_stop_resets_running_count: stop -> verify count is 0",
      "Run cargo test",
      "Commit: test(execution): add integration tests for execution control"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Write integration tests for startup resumption",
    "plan_section": "Verification",
    "steps": [
      "Create tests in src-tauri/src/application/startup_jobs.rs:",
      "  - test_resumption_skipped_when_paused: pause -> run -> verify no resumption",
      "  - test_resumption_spawns_agents: task in Executing -> run -> verify entry action called",
      "  - test_resumption_respects_max_concurrent: 5 tasks, max=2 -> run -> verify only 2 resume",
      "  - test_resumption_handles_empty_projects: no projects -> run -> no panic",
      "  - test_resumption_handles_multiple_statuses: tasks in various states -> run -> all agent-active resume",
      "Run cargo test",
      "Commit: test(startup): add integration tests for task resumption"
    ],
    "passes": false
  }
]
```

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **ExecutionState in spawner** | Single point of control for spawn gating; builder pattern maintains backward compatibility |
| **Decrement in on_exit()** | More reliable than wrapping spawn in async task; leverages existing state machine flow |
| **TransitionHandler for stop** | Ensures proper state machine transitions and side effects; maintains consistency |
| **StartupJobRunner module** | Separates startup concerns from main app setup; testable in isolation |
| **500ms startup delay** | Ensures HTTP server is ready before resuming tasks that may need MCP tools |
| **Respect max_concurrent on resume** | Prevents overwhelming system with too many concurrent agents on restart |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] Spawner blocks when paused
- [ ] Spawner blocks at max concurrent
- [ ] Running count increments on spawn
- [ ] Running count decrements on state exit
- [ ] Stop command transitions all executing tasks to Failed
- [ ] Stop command uses TransitionHandler (not direct DB update)
- [ ] StartupJobRunner resumes tasks in agent-active states
- [ ] StartupJobRunner respects pause state
- [ ] StartupJobRunner respects max_concurrent

### Build Verification
- [ ] `npm run lint` passes
- [ ] `npm run typecheck` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `npm run build` succeeds
- [ ] `cargo build --release` succeeds

### Integration Testing
- [ ] Pause prevents new spawns (drag task to In Progress, verify no agent)
- [ ] Running count updates correctly (start task, verify "Running: 1/2")
- [ ] Stop cancels all executing tasks (start 2, stop, verify both Failed)
- [ ] Resumption works (put task in Executing in DB, restart app, verify agent spawns)
- [ ] Resumption respects limits (5 tasks Executing, max_concurrent=2, verify only 2 resume)
