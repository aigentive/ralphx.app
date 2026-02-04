---
name: stop-and-pause-execution
overview: Implement definitive stop/pause semantics so stop is a permanent kill requiring manual restart, while pause allows auto-recovery on resume with clear UI messaging.
todos:
  - id: task-1-add-statuses
    content: "Task 1: Add Paused/Stopped statuses to InternalStatus (BE + FE types + tests)"
    status: pending
  - id: task-2-stop-command
    content: "Task 2: Update stop_execution to transition to Stopped status"
    status: pending
  - id: task-3-pause-command
    content: "Task 3: Update pause_execution to transition to Paused status"
    status: pending
  - id: task-4-resume-command
    content: "Task 4: Update resume_execution to restore only Paused tasks"
    status: pending
  - id: task-5-recovery-unblock
    content: "Task 5: Prevent auto-recovery/unblock for Paused/Stopped blockers"
    status: pending
  - id: task-6-frontend-ui
    content: "Task 6: Update frontend UI (badges, icons, labels, execution bar messaging)"
    status: pending
isProject: false
---

# Execution Stop/Pause Hard-Stop Plan

## Scope and behavior decisions

- Introduce two explicit statuses: `stopped` for Stop All and `paused` for Pause. `stopped` requires manual restart; `paused` can auto-recover on resume.
- Resume from the execution bar only restarts tasks that were `paused`, and resumes scheduler/queue processing. It does **not** restart `stopped` tasks.
- Blocked dependents should **not** auto-unblock when a blocker is `stopped` or `paused`.
- Add clear pre/post user messaging for Stop vs Pause behavior in the execution bar UI.

## Implementation Tasks

### Task 1: Add Paused/Stopped statuses to InternalStatus (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(status): add Paused and Stopped status variants`

Add new enum variants and update all status-related helpers in both backend and frontend:

**Backend files:**
- `src-tauri/src/domain/entities/status.rs` - Add `Paused`, `Stopped` variants
- Update `is_terminal()` - `Stopped` is terminal, `Paused` is NOT
- Update `is_agent_active()` - neither is agent-active
- Update `allowed_transitions()` for new statuses
- Add tests for new variants

**Frontend files:**
- `src/types/status.ts` - Add to `InternalStatus` type and Zod schema
- Update `TERMINAL_STATUSES` to include `stopped`
- Update `ACTIVE_STATUSES` - neither included
- Update any status lists/arrays

**Compilation unit note:** Adding new enum variants is additive. Existing code won't break because it doesn't reference the new variants yet. Both BE and FE types must be added together to maintain cross-layer type parity.

---

### Task 2: Update stop_execution to transition to Stopped status
**Dependencies:** Task 1
**Atomic Commit:** `feat(execution): transition to Stopped status on stop_execution`

Modify `stop_execution` command in `src-tauri/src/commands/execution_commands.rs`:
- Kill running agent processes immediately (registry stop)
- Transition all agent-active tasks to `Stopped` (not `Failed`)
- Keep execution paused
- Emit status updates after count reaches 0
- Add tests for stop behavior

---

### Task 3: Update pause_execution to transition to Paused status
**Dependencies:** Task 1
**Atomic Commit:** `feat(execution): transition to Paused status on pause_execution`

Modify `pause_execution` command in `src-tauri/src/commands/execution_commands.rs`:
- Stop running agents
- Transition agent-active tasks to `Paused`
- Set execution paused
- Add tests for pause behavior

---

### Task 4: Update resume_execution to restore only Paused tasks
**Dependencies:** Task 2, Task 3
**Atomic Commit:** `feat(execution): restore Paused tasks on resume, not Stopped`

Modify `resume_execution` command in `src-tauri/src/commands/execution_commands.rs`:
- Clear pause state
- Restore only `Paused` tasks to their last pre-pause status using status history
- Re-run entry actions for restored status via `TaskTransitionService::execute_entry_actions()`
- Do NOT restore `Stopped` tasks automatically
- Add tests for resume behavior with both Paused and Stopped tasks

---

### Task 5: Prevent auto-recovery/unblock for Paused/Stopped blockers
**Dependencies:** Task 1
**Atomic Commit:** `fix(recovery): prevent auto-unblock for Paused/Stopped blockers`

Update startup and reconciliation logic:
- `StartupJobRunner::run()` in `src-tauri/src/application/startup_jobs.rs` - ignore `paused`/`stopped`
- `ReconciliationRunner::recover_execution_stop()` - no-op for `paused`/`stopped` tasks
- `RepoBackedDependencyManager::is_blocker_complete()` in `src-tauri/src/application/task_transition_service.rs` - `Paused`/`Stopped` are NOT complete
- `StartupJobRunner::all_blockers_complete()` - exclude `Paused`/`Stopped`
- Add tests for blocker behavior

---

### Task 6: Update frontend UI for Paused/Stopped statuses
**Dependencies:** Task 1
**Atomic Commit:** `feat(ui): add Paused/Stopped status display and execution bar messaging`

Update UI components to display new statuses:
- `src/components/tasks/TaskDetailView.tsx` - status badges
- `src/components/tasks/TaskDetailPanel.tsx` - status display
- `src/components/TaskGraph/nodes/nodeStyles.ts` - node colors
- `src/types/status-icons.ts` - status icons
- `src/types/workflow.ts` - workflow mappings
- Update execution bar tooltips/confirmation copy to distinguish Stop vs Pause
- Ensure filters/counts include `stopped` in terminal but NOT `paused`

## Reference: Key Backend Files

| File | Purpose |
|------|---------|
| `src-tauri/src/domain/entities/status.rs` | InternalStatus enum, transitions, helpers |
| `src-tauri/src/commands/execution_commands.rs` | stop/pause/resume commands |
| `src-tauri/src/application/startup_jobs.rs` | StartupJobRunner, reconciliation |
| `src-tauri/src/application/task_transition_service.rs` | RepoBackedDependencyManager |

## Reference: Key Frontend Files

| File | Purpose |
|------|---------|
| `src/types/status.ts` | InternalStatus type, Zod schema, status lists |
| `src/types/status-icons.ts` | Status icons mapping |
| `src/types/workflow.ts` | Workflow status mappings |
| `src/components/tasks/TaskDetailView.tsx` | Task detail status display |
| `src/components/tasks/TaskDetailPanel.tsx` | Task panel status display |
| `src/components/TaskGraph/nodes/nodeStyles.ts` | Graph node colors |

## Risk checks / verification

- Ensure running count hits zero after stop/pause via `ExecutionState::emit_status_changed` updates.
- Confirm no startup resumption or unblock happens when blockers are `paused`/`stopped`.
- Verify Resume restarts only tasks that were paused/stopped (using status history), and does not auto-start unrelated Ready tasks unless scheduler is enabled.

## Task Dependency Graph

```
Task 1: Add statuses (BLOCKING)
   │
   ├──► Task 2: stop_execution
   │
   ├──► Task 3: pause_execution
   │       │
   │       └──► Task 4: resume_execution (depends on 2 + 3)
   │              ▲
   │──────────────┘
   │
   ├──► Task 5: recovery/unblock prevention
   │
   └──► Task 6: frontend UI
```

**Execution order:** 1 → (2, 3, 5, 6 in parallel) → 4

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Parallel Execution Strategy

Tasks 2, 3, 5, and 6 can run in parallel after Task 1 completes:
- **Task 2 + Task 3:** Both modify `execution_commands.rs` but different functions — coordinate via commit lock
- **Task 5:** Modifies `startup_jobs.rs` and `task_transition_service.rs` — independent
- **Task 6:** Frontend only — fully independent from backend tasks

Task 4 must wait for both Task 2 and Task 3 to complete.