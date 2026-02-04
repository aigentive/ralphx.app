---
name: project-scoped-execution
overview: Scope execution controls, scheduler, and status to the active project, with per-project execution settings and a global concurrency cap shared across projects, aligned with Phase 80 semantics and Phase 79 patterns.
todos: []
isProject: false
---

# Project-Scoped Execution Control Plan

## Scope Alignment With Pending Phases

- Phase 80 (Execution Stop/Pause Semantics) introduces new statuses and changes stop/pause flows; this plan assumes those semantics are already in place and extends them with per-project scoping.
- Phase 79 (Git Settings Per-Project) established per-project settings patterns; reuse its per-project update patterns for execution settings.
- Phase 81 (Graph toolbar compact) is unrelated; no coupling expected.

## Proposed Architecture (Per-Project Everything)

- Introduce a per-project execution state (paused flag, running count, max concurrent) keyed by `project_id`.
- Scheduler, pause/resume/stop, queue counts, recovery/resumption, and UI status/events are all scoped to the active project.
- Add a **global max concurrent cap** (configurable, default 20; UI allows up to 50) that limits the total concurrency across all projects combined, not per-project.
- Expose the global cap in Settings UI as a distinct “Global Max Concurrent” control with clear copy; keep per-project max in project settings.

## Implementation Steps

### 1) Backend: per-project execution state + active project context (BLOCKING)

**Dependencies:** None
**Atomic Commit:** `feat(execution): add per-project execution state registry and active project context`

- Add a small in-memory manager (e.g., `ExecutionStateRegistry`) keyed by `ProjectId`, replacing the single global state in `src-tauri/src/commands/execution_commands.rs` and `src-tauri/src/lib.rs`.
- Add an "active project" state container (e.g., `ActiveProjectState { current: Option<ProjectId> }`) in `AppState` or managed Tauri state; update it from frontend on project switch.
- Update execution commands with OPTIONAL `project_id` parameter for backwards compatibility:
  - `get_execution_status(project_id: Option<String>)` — if None, use active project or aggregate
  - `pause_execution(project_id: Option<String>)`
  - `resume_execution(project_id: Option<String>)`
  - `stop_execution(project_id: Option<String>)`
  - **CRITICAL:** Must be optional to maintain frontend compatibility until Task 3 completes
- Update event payloads to include `projectId`:
  - `execution:status_changed` and `execution:queue_changed`.
- Adjust scheduler to only consider tasks for the active project:
  - `TaskSchedulerService` should accept an optional `project_id` and query that project only (currently schedules across all projects).
- Update startup resumption and reconciliation to respect the active project:
  - Scope resumption to the active project (or to all only if no active project is set).
  - Ensure per-project paused/stopped tasks are not auto-resumed.

**Key files:**

- [src-tauri/src/commands/execution_commands.rs](src-tauri/src/commands/execution_commands.rs)
- [src-tauri/src/application/task_scheduler_service.rs](src-tauri/src/application/task_scheduler_service.rs)
- [src-tauri/src/application/startup_jobs.rs](src-tauri/src/application/startup_jobs.rs)
- [src-tauri/src/application/reconciliation.rs](src-tauri/src/application/reconciliation.rs)
- [src-tauri/src/lib.rs](src-tauri/src/lib.rs)

### 2) Backend: per-project execution settings + global cap (BLOCKING)

**Dependencies:** Task 1
**Atomic Commit:** `feat(execution): add per-project execution settings and global concurrency cap`

- Add a new repo/table for execution settings keyed by `project_id` (or extend `projects` table if preferred).
- Update command APIs with OPTIONAL `project_id` for backwards compatibility:
  - `get_execution_settings(project_id: Option<String>)` — if None, returns global defaults
  - `update_execution_settings(project_id: Option<String>, …)` — if None, updates global defaults
- Add a global execution settings record (single-row table or config file) that stores `global_max_concurrent` with a hard upper bound of 50.
- Enforce global cap across all projects: scheduling must not exceed `global_max_concurrent` when summing running counts across all projects.

**Key files:**

- [src-tauri/src/domain/repositories/execution_settings_repository.rs](src-tauri/src/domain/repositories/execution_settings_repository.rs)
- [src-tauri/src/infrastructure/sqlite/sqlite_execution_settings_repo.rs](src-tauri/src/infrastructure/sqlite/sqlite_execution_settings_repo.rs)
- [src-tauri/src/infrastructure/sqlite/migrations/](src-tauri/src/infrastructure/sqlite/migrations/)

### 3) Frontend: per-project status store + API changes

**Dependencies:** Task 1, Task 2
**Atomic Commit:** `feat(execution): add per-project execution status and API integration`

- Update execution API wrappers to pass `projectId` and reflect new schemas/types.
- Change `useExecutionStatus` to accept `projectId` and store status per project in `uiStore` (e.g., `executionStatusByProject`).
- Update `useExecutionEvents` to read `projectId` from payload and update only that project's status.
- Update `App.tsx` to:
  - Pass `currentProjectId` to execution API calls.
  - Send `set_active_project` command on project selection.
  - Render ExecutionControlBar with active project's status.
- Update `SettingsView` wiring to load/save execution settings per project, plus load/save the global cap setting.

**Key files:**

- [src/api/execution.ts](src/api/execution.ts)
- [src/hooks/useExecutionControl.ts](src/hooks/useExecutionControl.ts)
- [src/hooks/useExecutionEvents.ts](src/hooks/useExecutionEvents.ts)
- [src/stores/uiStore.ts](src/stores/uiStore.ts)
- [src/App.tsx](src/App.tsx)
- [src/components/settings/SettingsView.tsx](src/components/settings/SettingsView.tsx)

### 4) Tests + verification (TDD-first)

**Dependencies:** Task 1, Task 2, Task 3
**Atomic Commit:** `test(execution): add per-project execution scoping tests`

- Backend tests for per-project scoping:
  - `get_execution_status` counts only Ready tasks in the specified project.
  - `pause/resume/stop` only affect agent-active tasks in the specified project.
  - Scheduler only transitions Ready tasks in the active project.
  - Event payloads include `projectId`.
- Frontend tests:
  - `useExecutionEvents` updates only matching project status.
  - `useExecutionControl` uses query keys per project.

## “What Else” Considerations

- **Active-project sync:** ensure backend knows the active project before scheduling/resumption; define a safe default when none is selected.
- **Background tasks:** decide if tasks already running in an inactive project should continue or be paused; document behavior.
- **Global cap vs per-project max:** global cap is cross-project; per-project max should be clamped so total running never exceeds global cap; reflect in UI copy and validation.
- **Recovery/resumption:** per-project scoping must avoid resuming tasks from other projects on startup.
- **Event ordering:** ensure events for inactive projects don’t clobber the active project UI.

## Open Assumptions

- Execution settings are per project and stored per project in DB; a global cap (default 20, UI max 50) is stored globally and enforced across all projects.
- Scheduling and resumption are scoped to the active project unless no active project is set (then no auto-scheduling).

## Task Dependency Graph

```
Task 1 (Backend: state + context)
   │
   ├──► Task 2 (Backend: settings + global cap)
   │       │
   │       ▼
   └──────► Task 3 (Frontend: status store + API)
               │
               ▼
           Task 4 (Tests + verification)
```

**Critical Path:** 1 → 2 → 3 → 4

**Note:** Tasks 1 and 2 are purely additive backend changes (new structs, new endpoints). Task 3 updates frontend to use the new APIs. All tasks are valid compilation units—each compiles independently without breaking existing functionality.

## Compilation Unit Analysis (Based on File Review)

**Files analyzed:**
- Backend: `execution_commands.rs` (1447 LOC), `task_scheduler_service.rs` (781 LOC), `startup_jobs.rs` (1289 LOC), `reconciliation.rs` (887 LOC), `lib.rs` (545 LOC)
- Frontend: `execution.ts` (144 LOC), `useExecutionControl.ts` (166 LOC), `useExecutionEvents.ts` (92 LOC), `uiStore.ts` (527 LOC)

**Current state:**
- `ExecutionState` is a global singleton (`Arc<ExecutionState>`) created in `lib.rs:56`
- Commands (`get_execution_status`, `pause_execution`, etc.) take NO `project_id` parameter
- `TaskSchedulerService.find_oldest_schedulable_task()` queries across ALL projects
- `StartupJobRunner.run()` iterates ALL projects for resumption
- Frontend `executionStatus` in uiStore is a single object, not per-project

**CRITICAL: Backwards Compatibility Requirement**

Task 1 must make `project_id` an OPTIONAL parameter to maintain compilation:

```rust
// ✅ CORRECT - Optional project_id with fallback
#[tauri::command]
pub async fn get_execution_status(
    project_id: Option<String>,  // Optional for backwards compat
    execution_state: State<'_, Arc<ExecutionStateRegistry>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionStatusResponse, String> {
    let pid = project_id.or_else(|| app_state.active_project.get());
    // ... scope to pid if present, else aggregate across all
}

// ❌ WRONG - Required project_id breaks frontend until Task 3
#[tauri::command]
pub async fn get_execution_status(
    project_id: String,  // Breaks existing frontend calls
    ...
)
```

**Why this matters:**
1. After Task 1 completes, frontend still calls `get_execution_status()` with no args
2. If Task 1 makes project_id required, frontend TypeScript won't pass it → runtime error
3. Task 3 adds the projectId to frontend calls, but Task 3 depends on Task 1 being complete
4. By making project_id optional, Task 1 compiles AND works with existing frontend

**Per-task compilation verification:**
| Task | After completion, what works? |
|------|-------------------------------|
| 1 | Backend compiles, frontend works (uses legacy no-project-id path) |
| 2 | Backend compiles, new settings API available (also with optional project_id) |
| 3 | Frontend passes projectId, backend uses per-project scoping |
| 4 | All tests pass, full feature verified |

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)