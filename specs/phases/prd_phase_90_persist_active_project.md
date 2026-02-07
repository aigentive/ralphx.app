# RalphX - Phase 90: Persist ActiveProjectState to SQLite

## Overview

Phase 89's Notify-based fix for the `ActiveProjectState` race condition doesn't work because the webview hasn't loaded the React app within the 5-second timeout (especially in dev mode). The frontend `useEffect` that calls `setActiveProject()` fires after the timeout expires.

**Root cause**: `ActiveProjectState` is in-memory only — starts as `None` every restart. The backend depends on the frontend to re-send the active project, but the frontend isn't ready yet.

**Fix**: Persist `active_project_id` to a new `app_state` singleton SQLite table. On startup, `StartupJobRunner` reads directly from DB — no waiting, no race condition. Remove the Phase 89 Notify-based wait mechanism entirely.

**Reference Plan:**
- `specs/plans/fix_active_project_state_race_condition.md` - Detailed implementation plan with entity/trait definitions, migration SQL, and compilation unit analysis

## Goals

1. Persist `active_project_id` to SQLite so it survives app restarts
2. Remove Notify-based `wait_for_project()` from `ActiveProjectState` (revert to simple RwLock)
3. Have `StartupJobRunner` read active project from DB — no frontend dependency, no race condition
4. Keep `set_active_project` command writing to both DB and in-memory state

## Dependencies

### Phase 89 (Fix ActiveProjectState Race Condition on Startup) - Superseded

| Dependency | Why Needed |
|------------|------------|
| `ActiveProjectState` struct | This phase modifies it (removes Notify, adds DB persistence) |
| `StartupJobRunner` | This phase changes how it obtains the active project |

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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_active_project_state_race_condition.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Create AppStateRepository trait and AppSettings entity",
    "plan_section": "Task 1: Create AppStateRepository trait + entity",
    "blocking": [2, 3],
    "blockedBy": [],
    "atomic_commit": "feat(startup): add AppStateRepository for persisted app state",
    "steps": [
      "Read specs/plans/fix_active_project_state_race_condition.md section 'Task 1'",
      "Create src-tauri/src/domain/entities/app_state.rs with AppSettings entity (active_project_id: Option<ProjectId>)",
      "Create src-tauri/src/domain/repositories/app_state_repository.rs with AppStateRepository trait (get, set_active_project)",
      "Follow GlobalExecutionSettingsRepository pattern for trait definition",
      "Add module declarations and re-exports in domain/entities/mod.rs and domain/repositories/mod.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(startup): add AppStateRepository for persisted app state"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add v14 migration for app_state table, SQLite implementation, and Memory implementation",
    "plan_section": "Task 2: Migration v14 + SQLite impl + Memory impl",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(startup): add app_state table with SQLite and memory implementations",
    "steps": [
      "Read specs/plans/fix_active_project_state_race_condition.md section 'Task 2'",
      "Create src-tauri/src/infrastructure/sqlite/migrations/v14_app_state.rs with singleton table migration (id=1 CHECK, active_project_id TEXT, updated_at TEXT)",
      "Create src-tauri/src/infrastructure/sqlite/migrations/v14_app_state_tests.rs with migration tests",
      "Bump SCHEMA_VERSION to 14 and register v14 migration in migrations/mod.rs",
      "Create src-tauri/src/infrastructure/sqlite/sqlite_app_state_repo.rs following SqliteGlobalExecutionSettingsRepository pattern (from_shared, get, set_active_project)",
      "Create src-tauri/src/infrastructure/memory/memory_app_state_repo.rs following MemoryGlobalExecutionSettingsRepository pattern (Arc<RwLock<AppSettings>>)",
      "Add module declarations and re-exports in sqlite/mod.rs and memory/mod.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(startup): add app_state table with SQLite and memory implementations"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Wire AppStateRepository into AppState, set_active_project command, and StartupJobRunner; remove Notify-based wait",
    "plan_section": "Task 3: Wire into AppState + set_active_project command + StartupJobRunner",
    "blocking": [],
    "blockedBy": [1, 2],
    "atomic_commit": "fix(startup): persist active project to DB, read on startup",
    "steps": [
      "Read specs/plans/fix_active_project_state_race_condition.md section 'Task 3'",
      "Add app_state_repo: Arc<dyn AppStateRepository> field to AppState struct in application/app_state.rs",
      "Wire SqliteAppStateRepository in new_production() and with_db_path(), MemoryAppStateRepository in new_test() and with_repos()",
      "In commands/execution_commands.rs: remove Notify from ActiveProjectState (revert to simple RwLock, remove wait_for_project method)",
      "In commands/execution_commands.rs: update set_active_project command to also write to DB via app_state.app_state_repo.set_active_project()",
      "In application/startup_jobs.rs: add app_state_repo field to StartupJobRunner, read active_project_id from DB in run(), set in-memory ActiveProjectState from DB value",
      "In application/startup_jobs.rs: remove active_project_wait_timeout field and with_active_project_timeout() builder method",
      "In lib.rs: pass app_state_repo to StartupJobRunner::new()",
      "Update startup_jobs/tests.rs: pass app_state_repo to build_runner(), remove with_active_project_timeout() calls, remove test_resumption_waits_for_active_project, add test_resumption_reads_active_project_from_db",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(startup): persist active project to DB, read on startup"
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
| **Singleton `app_state` table (id=1 CHECK)** | Only one active project at a time; CHECK constraint prevents multiple rows |
| **Remove Notify, keep simple RwLock** | DB persistence eliminates the race condition entirely; Notify mechanism was a workaround |
| **Both DB + in-memory writes in `set_active_project`** | In-memory for fast reads during execution; DB for persistence across restarts |
| **StartupJobRunner reads from DB, not in-memory** | DB has the last-known project from previous session; in-memory starts as None |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] `cargo test -p ralphx -- startup_jobs` — all startup job tests pass
- [ ] `cargo test -p ralphx -- app_state` — new repo tests pass
- [ ] New test `test_resumption_reads_active_project_from_db` passes
- [ ] Migration v14 tests pass

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### Manual Testing
- [ ] `npm run tauri dev` → quit → restart → logs show immediate project detection (no "waiting", no "no active project")
- [ ] Start fresh (delete ralphx.db) → restart → logs show "no active project" (correct for fresh install)
- [ ] Switch project in frontend → quit → restart → correct project active on startup

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `set_active_project` command writes to both in-memory state AND app_state DB table
- [ ] `StartupJobRunner.run()` reads from `app_state_repo.get()` before attempting task resumption
- [ ] `ActiveProjectState` no longer has `Notify` or `wait_for_project()` method
- [ ] `StartupJobRunner` no longer has `active_project_wait_timeout` or `with_active_project_timeout()`
- [ ] `lib.rs` passes `app_state_repo` to `StartupJobRunner::new()`

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
