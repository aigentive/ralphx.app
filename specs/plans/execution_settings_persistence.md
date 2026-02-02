# Plan: Execution Settings Persistence

## Problem Summary

When a user changes "Max Concurrent Tasks" in the settings UI, nothing happens because:

1. **No frontend API wrapper** - No API to call the backend
2. **No persistence** - Settings only exist in memory, lost on restart
3. **No initialization** - App always starts with hardcoded `max_concurrent: 2`
4. **No wiring** - `App.tsx` renders `<SettingsView />` with no props

## Current Architecture

```
SettingsView (UI)           Backend (Rust)
───────────────────         ─────────────────────────────────
onChange callback ──┐       set_max_concurrent command ✅
                    ↓       ExecutionState (in-memory) ✅
onSettingsChange ───→ ❌    TaskScheduler.can_start_task() ✅
  (no handler)
                            Database persistence ❌
                            Load on startup ❌
```

**ExecutionState explained:** This is an in-memory struct (`execution_commands.rs:42-136`) with:
- `max_concurrent: AtomicU32` - starts at hardcoded `2`
- `set_max_concurrent(max)` - updates the value
- `can_start_task()` - checks `running_count() < max_concurrent()`

The scheduler already calls `execution_state.can_start_task()` before starting tasks. We DON'T need to change the scheduler - we just need to:
1. Persist the setting to DB
2. Sync `ExecutionState` with DB on startup and on change

**Target architecture:**
```
Database (source of truth) → ExecutionState (runtime cache) → Scheduler
         ↑                           ↓
    persist on change         scheduler checks can_start_task()
```

## Existing Pattern: ideation_settings

The codebase already has a settings pattern in `sqlite_ideation_settings_repo.rs`:
- Single row with `id = 1` (global, not per-project)
- Repository trait + SQLite implementation
- `get_settings()` → returns defaults if no row
- `update_settings()` → updates the single row

**Decision:** Follow the same pattern for execution settings.

---

## Implementation Plan

### Task 1: Database Migration for Execution Settings (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(db): add execution_settings table migration`

**Files:**
- `src-tauri/src/infrastructure/sqlite/migrations/v10_execution_settings.rs`
- `src-tauri/src/infrastructure/sqlite/migrations/v10_execution_settings_tests.rs`
- `src-tauri/src/infrastructure/sqlite/migrations/mod.rs` (update MIGRATIONS array, bump SCHEMA_VERSION)

```sql
CREATE TABLE IF NOT EXISTS execution_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    max_concurrent_tasks INTEGER NOT NULL DEFAULT 2,
    auto_commit INTEGER NOT NULL DEFAULT 1,
    pause_on_failure INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO execution_settings (id, max_concurrent_tasks, auto_commit, pause_on_failure, updated_at)
VALUES (1, 2, 1, 1, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'));
```

### Task 2: Domain + Repository (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(execution): add ExecutionSettings domain and repository layer`

**Files:**
- `src-tauri/src/domain/execution/mod.rs` - New module
- `src-tauri/src/domain/execution/settings.rs` - ExecutionSettings struct
- `src-tauri/src/domain/mod.rs` - Add `pub mod execution;`
- `src-tauri/src/domain/repositories/mod.rs` - ExecutionSettingsRepository trait
- `src-tauri/src/infrastructure/sqlite/sqlite_execution_settings_repo.rs` - SQLite impl
- `src-tauri/src/infrastructure/sqlite/mod.rs` - Export the repo
- `src-tauri/src/infrastructure/memory/mod.rs` - MemoryExecutionSettingsRepository for tests
- `src-tauri/src/application/app_state.rs` - Add `execution_settings_repo` field

**ExecutionSettings struct:**
```rust
#[derive(Debug, Clone)]
pub struct ExecutionSettings {
    pub max_concurrent_tasks: u32,
    pub auto_commit: bool,
    pub pause_on_failure: bool,
}
```

**Repository trait:**
```rust
#[async_trait]
pub trait ExecutionSettingsRepository: Send + Sync {
    async fn get_settings(&self) -> AppResult<ExecutionSettings>;
    async fn update_settings(&self, settings: &ExecutionSettings) -> AppResult<ExecutionSettings>;
}
```

### Task 3: Tauri Commands (BLOCKING)
**Dependencies:** Task 2
**Atomic Commit:** `feat(commands): add get/update execution settings commands`

**Files:**
- `src-tauri/src/commands/execution_commands.rs` (add commands)
- `src-tauri/src/lib.rs` (register new commands in `.invoke_handler()`)

**Commands:**
```rust
#[tauri::command]
async fn get_execution_settings(
    state: State<'_, AppState>,
) -> AppResult<ExecutionSettingsResponse>

#[tauri::command]
async fn update_execution_settings(
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    input: UpdateExecutionSettingsInput,
) -> AppResult<ExecutionSettingsResponse>
```

**update_execution_settings flow:**
1. Save to DB via repository
2. If `max_concurrent_tasks` changed → call `execution_state.set_max_concurrent()`
3. Emit `settings:execution:updated` event
4. Return updated settings

### Task 4: Frontend API Wrapper (BLOCKING)
**Dependencies:** Task 3
**Atomic Commit:** `feat(api): add execution settings API wrapper`

**Files:** `src/api/execution.ts` (extend existing), `src/api/execution.schemas.ts`, `src/api/execution.types.ts`, `src/api/execution.transforms.ts`

```typescript
export const executionApi = {
  // existing...
  getSettings: () => typedInvokeWithTransform(...),
  updateSettings: (input) => typedInvokeWithTransform(...),
};
```

### Task 5: Wire SettingsView in App.tsx
**Dependencies:** Task 4
**Atomic Commit:** `feat(settings): wire execution settings to SettingsView`

**Files:** `src/App.tsx`

1. Add state: `executionSettings`, `isLoadingSettings`, `isSavingSettings`
2. Load settings on mount via `executionApi.getSettings()`
3. Handle changes: call `executionApi.updateSettings()` with debounce
4. Pass props to `<SettingsView initialSettings={...} onSettingsChange={...} />`

### Task 6: Initialize ExecutionState on Startup
**Dependencies:** Task 2
**Atomic Commit:** `feat(startup): load execution settings from DB on app init`

**Files:** `src-tauri/src/lib.rs`

**Current flow (lib.rs):**
```rust
let execution_state = Arc::new(ExecutionState::new()); // hardcoded max=2
// ... later ...
let app_state = AppState::new_production(...); // has DB access
```

**New flow:**
```rust
let execution_state = Arc::new(ExecutionState::new()); // still starts with default
// ... later ...
let app_state = AppState::new_production(...);

// NEW: Load settings and apply to ExecutionState
if let Ok(settings) = app_state.execution_settings_repo.get_settings().await {
    execution_state.set_max_concurrent(settings.max_concurrent_tasks);
}
```

This must happen in the `setup()` callback (around line 95) after AppState is created but before the HTTP server starts.

---

## Critical Files

| Layer | File | Changes |
|-------|------|---------|
| DB | `src-tauri/.../migrations/v10_execution_settings.rs` | New table |
| DB | `src-tauri/.../migrations/mod.rs` | Register migration, bump version |
| Domain | `src-tauri/src/domain/execution/settings.rs` | ExecutionSettings struct |
| Repo trait | `src-tauri/src/domain/repositories/mod.rs` | ExecutionSettingsRepository trait |
| Repo impl | `src-tauri/.../sqlite_execution_settings_repo.rs` | SQLite implementation |
| Memory | `src-tauri/.../memory/mod.rs` | Memory impl for tests |
| AppState | `src-tauri/src/application/app_state.rs` | Add repo field |
| Commands | `src-tauri/src/commands/execution_commands.rs` | get/update commands |
| Register | `src-tauri/src/lib.rs` | Register commands, init settings |
| API | `src/api/execution.ts` | getSettings/updateSettings |
| API | `src/api/execution.schemas.ts` | Zod schemas |
| UI | `src/App.tsx` | Wire SettingsView props |

## Verification

1. Change max concurrent tasks in UI → verify scheduler respects new limit
2. Restart app → verify setting persisted (not reset to 2)
3. Run `cargo test` for repository tests
4. Run `npm run typecheck` for frontend

## Implementation Notes

- **Auto-save:** Settings save immediately on change (following ideation_settings pattern)
- **Event emission:** `settings:execution:updated` event for other components to react
- **Debounce:** Frontend should debounce rapid changes (300ms)

## Task Dependency Graph

```
Task 1 (Migration)
    ↓
Task 2 (Domain + Repo) ──────┐
    ↓                        ↓
Task 3 (Commands)      Task 6 (Startup Init)
    ↓
Task 4 (Frontend API)
    ↓
Task 5 (Wire UI)
```

**Parallelization opportunity:** Tasks 3 and 6 can run in parallel after Task 2 completes.

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
