# RalphX - Phase 77: Execution Settings Persistence

## Overview

When a user changes "Max Concurrent Tasks" in the settings UI, nothing happens because the setting only exists in memory and there's no frontend API to call the backend. This phase implements full persistence of execution settings (max concurrent tasks, auto-commit, pause on failure) with database storage and proper initialization on app startup.

The scheduler already uses `ExecutionState.can_start_task()` to check concurrency limits - we just need to persist the setting to the database and sync `ExecutionState` with the DB on startup and on change.

**Reference Plan:**
- `specs/plans/execution_settings_persistence.md` - Complete implementation plan with architecture decisions and code patterns

## Goals

1. Persist execution settings to SQLite database using the established singleton pattern
2. Sync ExecutionState with database on app startup (load persisted value)
3. Create frontend API wrapper for settings CRUD operations
4. Wire SettingsView to save changes immediately on user input

## Dependencies

### Phase 76 (Hybrid Merge Completion Detection) - Complete

No direct dependencies on Phase 76 features. This phase builds on existing infrastructure:

| Dependency | Why Needed |
|------------|------------|
| AppState pattern | New repo follows existing dependency injection pattern |
| ideation_settings pattern | Model for singleton settings table design |
| ExecutionState struct | Already exists with `set_max_concurrent()` method |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/execution_settings_persistence.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/execution_settings_persistence.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Create database migration for execution_settings table",
    "plan_section": "Task 1: Database Migration for Execution Settings",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(db): add execution_settings table migration",
    "steps": [
      "Read specs/plans/execution_settings_persistence.md section 'Task 1'",
      "Create v10_execution_settings.rs with CREATE TABLE and INSERT",
      "Create v10_execution_settings_tests.rs with migration tests",
      "Update migrations/mod.rs: add to MIGRATIONS array, bump SCHEMA_VERSION",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(db): add execution_settings table migration"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add ExecutionSettings domain struct and repository layer",
    "plan_section": "Task 2: Domain + Repository",
    "blocking": [3, 6],
    "blockedBy": [1],
    "atomic_commit": "feat(execution): add ExecutionSettings domain and repository layer",
    "steps": [
      "Read specs/plans/execution_settings_persistence.md section 'Task 2'",
      "Create domain/execution/mod.rs and domain/execution/settings.rs",
      "Add pub mod execution to domain/mod.rs",
      "Add ExecutionSettingsRepository trait to domain/repositories/mod.rs",
      "Create sqlite_execution_settings_repo.rs with SQLite implementation",
      "Update infrastructure/sqlite/mod.rs to export the repo",
      "Create MemoryExecutionSettingsRepository in memory/mod.rs for tests",
      "Add execution_settings_repo field to AppState in app_state.rs",
      "Update AppState::new_production() and new_test() to accept repo",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(execution): add ExecutionSettings domain and repository layer"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add get/update execution settings Tauri commands",
    "plan_section": "Task 3: Tauri Commands",
    "blocking": [4],
    "blockedBy": [2],
    "atomic_commit": "feat(commands): add get/update execution settings commands",
    "steps": [
      "Read specs/plans/execution_settings_persistence.md section 'Task 3'",
      "Add get_execution_settings command to execution_commands.rs",
      "Add update_execution_settings command with ExecutionState sync",
      "Add ExecutionSettingsResponse and UpdateExecutionSettingsInput types",
      "Emit settings:execution:updated event on successful update",
      "Register commands in lib.rs invoke_handler",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(commands): add get/update execution settings commands"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Add execution settings API wrapper with schemas and transforms",
    "plan_section": "Task 4: Frontend API Wrapper",
    "blocking": [5],
    "blockedBy": [3],
    "atomic_commit": "feat(api): add execution settings API wrapper",
    "steps": [
      "Read specs/plans/execution_settings_persistence.md section 'Task 4'",
      "Add Zod schemas to execution.schemas.ts (snake_case)",
      "Add TypeScript types to execution.types.ts (camelCase)",
      "Add transform functions to execution.transforms.ts",
      "Add getSettings and updateSettings to executionApi in execution.ts",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(api): add execution settings API wrapper"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Wire SettingsView to load and persist execution settings",
    "plan_section": "Task 5: Wire SettingsView in App.tsx",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(settings): wire execution settings to SettingsView",
    "steps": [
      "Read specs/plans/execution_settings_persistence.md section 'Task 5'",
      "Add executionSettings state to App.tsx",
      "Load settings on mount via executionApi.getSettings()",
      "Add debounced handler for settings changes (300ms)",
      "Pass initialSettings and onSettingsChange props to SettingsView",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(settings): wire execution settings to SettingsView"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "backend",
    "description": "Initialize ExecutionState from database on app startup",
    "plan_section": "Task 6: Initialize ExecutionState on Startup",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "feat(startup): load execution settings from DB on app init",
    "steps": [
      "Read specs/plans/execution_settings_persistence.md section 'Task 6'",
      "In lib.rs setup() callback, after AppState creation",
      "Load settings via execution_settings_repo.get_settings()",
      "Call execution_state.set_max_concurrent() with loaded value",
      "Handle errors gracefully (log warning, use default)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(startup): load execution settings from DB on app init"
    ],
    "passes": false
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start (inverse of blocking)
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Singleton table (id=1)** | Follows existing ideation_settings pattern for global settings |
| **ExecutionState as runtime cache** | Keep existing scheduler logic unchanged, just sync on startup/change |
| **Event emission on update** | Allow other components to react to settings changes |
| **Immediate save on change** | UX pattern from ideation_settings - no explicit "Save" button needed |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] Migration creates execution_settings table successfully
- [ ] Repository returns default settings when table is empty
- [ ] Repository persists and retrieves settings correctly

### Frontend - Run `npm run test`
- [ ] API wrapper calls backend commands correctly
- [ ] Schema validation matches backend response format

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Change max concurrent tasks in UI → verify scheduler respects new limit
- [ ] Restart app → verify setting persisted (not reset to 2)
- [ ] Change setting with multiple tasks running → verify live update

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] SettingsView onChange → executionApi.updateSettings() → Tauri command
- [ ] Tauri command → repository.update_settings() → database
- [ ] Tauri command → execution_state.set_max_concurrent() → scheduler
- [ ] App startup → repository.get_settings() → execution_state.set_max_concurrent()

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
