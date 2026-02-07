# Fix: ActiveProjectState Race Condition — Persist to SQLite

## Context

The Phase 89 Notify-based fix doesn't work because the webview hasn't loaded the React app within 5 seconds (especially in dev mode). The frontend `useEffect` that calls `setActiveProject()` fires after the timeout expires.

**Root cause**: `ActiveProjectState` is in-memory only — starts as `None` every restart. The backend depends on the frontend to re-send the active project, but the frontend isn't ready yet.

**Fix**: Persist `active_project_id` to a new `app_state` singleton SQLite table. On startup, `StartupJobRunner` reads directly from DB — no waiting, no race condition.

## Files to Modify/Create (~10 files)

### Task 1: Create `AppStateRepository` trait + entity (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(startup): add AppStateRepository for persisted app state`

**New files:**
- `src-tauri/src/domain/repositories/app_state_repository.rs` — trait definition
- `src-tauri/src/domain/entities/app_state.rs` — `AppSettings` entity (just `active_project_id: Option<ProjectId>`)

**Modify:**
- `src-tauri/src/domain/repositories/mod.rs` — add module + re-export
- `src-tauri/src/domain/entities/mod.rs` — add module + re-export

**Pattern** (follow `GlobalExecutionSettingsRepository`):
```rust
#[async_trait]
pub trait AppStateRepository: Send + Sync {
    async fn get(&self) -> Result<AppSettings, Box<dyn std::error::Error>>;
    async fn set_active_project(&self, project_id: Option<&ProjectId>) -> Result<(), Box<dyn std::error::Error>>;
}
```

Entity:
```rust
#[derive(Debug, Clone, Default)]
pub struct AppSettings {
    pub active_project_id: Option<ProjectId>,
}
```

### Task 2: Migration v14 + SQLite impl + Memory impl
**Dependencies:** Task 1
**Atomic Commit:** `feat(startup): add app_state table with SQLite and memory implementations`

**New files:**
- `src-tauri/src/infrastructure/sqlite/migrations/v14_app_state.rs` — migration
- `src-tauri/src/infrastructure/sqlite/migrations/v14_app_state_tests.rs` — migration tests
- `src-tauri/src/infrastructure/sqlite/sqlite_app_state_repo.rs` — SQLite impl
- `src-tauri/src/infrastructure/memory/memory_app_state_repo.rs` — Memory impl (for tests)

**Modify:**
- `src-tauri/src/infrastructure/sqlite/migrations/mod.rs` — bump SCHEMA_VERSION to 14, register migration
- `src-tauri/src/infrastructure/sqlite/mod.rs` — add module + re-export
- `src-tauri/src/infrastructure/memory/mod.rs` — add module + re-export

**Migration v14:**
```sql
CREATE TABLE IF NOT EXISTS app_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    active_project_id TEXT DEFAULT NULL,
    updated_at TEXT NOT NULL
);
INSERT OR IGNORE INTO app_state (id, active_project_id, updated_at)
VALUES (1, NULL, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'));
```

**SQLite impl** (follow `SqliteGlobalExecutionSettingsRepository` pattern):
- `from_shared(conn: Arc<Mutex<Connection>>)` constructor
- `get()` → `SELECT active_project_id FROM app_state WHERE id = 1`
- `set_active_project()` → `UPDATE app_state SET active_project_id = ?, updated_at = ... WHERE id = 1`

**Memory impl** (follow `MemoryGlobalExecutionSettingsRepository` pattern):
- `Arc<RwLock<AppSettings>>` internal state
- `Default` impl creates with `active_project_id: None`

### Task 3: Wire into AppState + set_active_project command + StartupJobRunner
**Dependencies:** Task 1, Task 2
**Atomic Commit:** `fix(startup): persist active project to DB, read on startup`

**Modify:**
- `src-tauri/src/application/app_state.rs` — add `app_state_repo: Arc<dyn AppStateRepository>` field, wire in `new_production()`, `new_app_data()`, `new_test()`
- `src-tauri/src/commands/execution_commands.rs`:
  - `set_active_project` command: after setting in-memory state, also write to DB via `app_state.app_state_repo.set_active_project()`
  - Remove `Notify` from `ActiveProjectState` (revert to simple `RwLock`, remove `wait_for_project()`)
- `src-tauri/src/application/startup_jobs.rs`:
  - Add `app_state_repo: Arc<dyn AppStateRepository>` field to `StartupJobRunner`
  - In `run()`: read `active_project_id` from `app_state_repo.get()`, then set in-memory `ActiveProjectState` from DB value
  - Remove `active_project_wait_timeout` field and `with_active_project_timeout()` builder
- `src-tauri/src/application/startup_jobs/tests.rs`:
  - Update `build_runner()` to pass `app_state_repo`
  - Remove `with_active_project_timeout()` calls
  - Tests that set active project: set it on the memory repo instead (or keep setting in-memory — both work since runner reads from repo)
  - Remove `test_resumption_waits_for_active_project` (Notify-based, no longer relevant)
  - Add new test: `test_resumption_reads_active_project_from_db` — set project in repo, don't set in-memory, verify runner reads from DB and resumes
- `src-tauri/src/lib.rs` — pass `app_state_repo` to `StartupJobRunner::new()`

## Startup Flow (After Fix)

```
1. Rust binary starts
2. Migrations run (app_state table exists with last active_project_id)
3. 500ms delay (for HTTP server)
4. StartupJobRunner reads active_project_id from app_state table
5. Sets in-memory ActiveProjectState from DB value
6. Proceeds with task resumption — no waiting needed
7. (Later) Frontend loads, calls setActiveProject() — writes to DB again (idempotent)
```

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| Normal restart | DB has active_project_id from last session → immediate resumption |
| Fresh install | app_state.active_project_id = NULL → skip resumption (correct) |
| Project deleted between sessions | DB has stale ID → project_repo lookup fails → skip (existing logic handles this) |
| Frontend switches project | set_active_project writes to DB + in-memory → next restart uses new project |

## Compilation Unit Validation

All 3 tasks verified as complete compilation units:

| Task | Type | Why It Compiles Alone |
|------|------|----------------------|
| 1 | Additive | New entity + trait files. Module declarations in `mod.rs` files. Nothing references these yet. |
| 2 | Additive (blockedBy: 1) | New migration + impl files. References Task 1 types via `use crate::domain::...`. No existing code changes. |
| 3 | Breaking changes (blockedBy: 1, 2) | Removes `Notify`, `wait_for_project()`, changes `StartupJobRunner::new()` signature. All callers (`lib.rs`, `tests.rs`) updated in same task. |

**Task 3 is the critical compilation unit** — it groups:
- Removal of `Notify` from `ActiveProjectState` (breaks `wait_for_project()` callers)
- Removal of `active_project_wait_timeout` field (breaks `with_active_project_timeout()` callers)
- Addition of `app_state_repo` param to `StartupJobRunner::new()` (breaks all callers)
- All callers updated: `lib.rs`, `startup_jobs/tests.rs`

## Verification

1. `cargo test -p ralphx -- startup_jobs` — all tests pass
2. `cargo test -p ralphx -- app_state` — new repo tests pass
3. `cargo clippy --all-targets --all-features -- -D warnings` — no warnings
4. Manual: `npm run tauri dev` → quit → restart → observe logs show immediate project detection and task resumption (no "waiting", no "no active project")

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
