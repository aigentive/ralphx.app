# Plan: Persist and Apply Per-Project Max Concurrency End-to-End

## Summary
Wire `Max Concurrent Tasks` so it is reliably persisted per project in SQLite and consistently used as the active quota for:
1. scheduler capacity checks, and
2. execution bar (`running/max`) display.

This will fix the current drift where UI/project switching can show or run against stale in-memory quota.

## Decisions Locked
- Scheduler scope: **active project only**.
- Per-project `max_concurrent_tasks` is the quota shown in execution bar.
- Global cap (`global_max_concurrent`) remains an additional cross-project ceiling.

## Implementation

1. **Backend quota sync helper (single source of truth)**
- Add a helper in `src-tauri/src/commands/execution_commands.rs` to:
  - resolve effective project (`explicit project_id` > `active_project_state` > none),
  - load execution settings from `execution_settings_repo.get_settings(...)`,
  - update `execution_state.set_max_concurrent(...)`.
- Return resolved project + resolved max for callers.

2. **Apply helper in execution command paths**
- Call the helper in:
  - `get_execution_status` (so bar always reflects active project quota),
  - `resume_execution` (before `can_start_task()` loops),
  - `pause_execution` / `stop_execution` (status/event consistency),
  - `set_active_project` (immediately rebind runtime quota on project switch).
- In `set_active_project`, emit `execution:status_changed` after sync so UI updates instantly.

3. **Scheduler scoping in execution command scheduling triggers**
- Where `TaskSchedulerService::new(...).try_schedule_ready_tasks()` is called from execution commands, set scheduler active project before scheduling:
  - use resolved/effective project ID,
  - avoid scheduling other projects when active-project mode is intended.

4. **Startup recovery alignment**
- In `src-tauri/src/application/startup_jobs.rs`, after loading persisted active project, load that project’s execution settings and set `execution_state.max_concurrent` before resumption/scheduling logic.

5. **Settings UI correctness on project switch**
- In `src/components/settings/SettingsView.tsx`, add prop-to-state sync:
  - when `initialSettings` changes, refresh internal `settings` state.
- This prevents stale settings UI across project switches and ensures edits target the correct project values.

6. **Execution status query scoping in frontend**
- In `src/App.tsx`, call `useExecutionStatus(currentProjectId || undefined)` instead of unscoped `useExecutionStatus()`.
- Keeps query keys/project scoping explicit and aligned with active project context.

## Public API / Interface Impact
- No breaking API changes.
- Existing Tauri commands remain the same.
- Event behavior change: more reliable/instant `execution:status_changed` after active project switch (same event name/payload shape, with project context where available).

## Tests

1. **Rust command tests (`src-tauri/src/commands/execution_commands.rs`)**
- Add tests:
  - switching active project updates `execution_state.max_concurrent` from that project’s persisted settings,
  - `get_execution_status` reports the project-specific `max_concurrent`,
  - increasing project quota triggers scheduler in active-project scope.

2. **Rust startup test (`src-tauri/src/application/startup_jobs/tests.rs`)**
- Add/extend test:
  - persisted active project on startup applies that project’s quota before resumption.

3. **Frontend component test (`src/components/settings/SettingsView.test.tsx`)**
- Add test:
  - rerender with new `initialSettings` updates displayed `Max Concurrent Tasks` input.

4. **Frontend app-level behavior (existing app/integration test location)**
- Add/extend test:
  - project switch causes execution status/bar max to reflect switched project quota.

## Assumptions and Defaults
- Active project is always the scheduler scope for UI-driven flows.
- If no active project exists, fallback remains global-default execution settings (`project_id IS NULL` row).
- Global execution cap continues to constrain `can_start_task` in addition to project quota.
