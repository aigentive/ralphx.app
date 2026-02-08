# RalphX - Phase 105: Fix Duplicate Agent Processes on App Restart

## Overview

When the RalphX app restarts (including multiple quick restarts), orphaned Claude agent OS processes accumulate — multiple workers for the same task, and reviewers for tasks already in terminal states. This happens because the `RunningAgentRegistry` is in-memory only (lost on restart) and `ChatResumptionRunner` doesn't guard against terminal-state tasks.

This phase fixes both bugs: (1) persisting the agent registry to SQLite so orphaned PIDs can be killed on restart, and (2) adding a terminal-state guard to chat resumption.

**Reference Plan:**
- `specs/plans/fix_duplicate_agent_processes_on_app_restart.md` - Detailed implementation plan with compilation unit analysis and file-level change list

## Goals

1. Persist running agent PIDs to SQLite so they survive app restarts
2. Kill orphaned agent OS processes on startup before spawning new ones
3. Prevent chat resumption from spawning agents for terminal-state tasks
4. Maintain backward compatibility with existing test infrastructure (MemoryRunningAgentRegistry)

## Dependencies

### Phase 104 (Reopen & Reset Ideation Sessions) - Required

| Dependency | Why Needed |
|------------|------------|
| Existing RunningAgentRegistry | Current in-memory registry is the baseline for trait extraction |
| Existing ChatResumptionRunner | Terminal-state guard builds on existing `is_handled_by_task_resumption()` |
| Migration system at v16 | New v17 migration builds on existing schema |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_duplicate_agent_processes_on_app_restart.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_duplicate_agent_processes_on_app_restart.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add DB migration for running_agents table",
    "plan_section": "Task 1: Add DB migration for running_agents table",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(db): add v17 running_agents migration",
    "steps": [
      "Read specs/plans/fix_duplicate_agent_processes_on_app_restart.md section 'Task 1'",
      "Create src-tauri/src/infrastructure/sqlite/migrations/v17_running_agents.rs with CREATE TABLE IF NOT EXISTS running_agents",
      "Register in MIGRATIONS array in migrations/mod.rs",
      "Bump SCHEMA_VERSION to 17",
      "Create v17_running_agents_tests.rs with table creation idempotency test",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(db): add v17 running_agents migration"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Extract RunningAgentRegistry trait, create SQLite impl, update all 17+ call sites atomically",
    "plan_section": "Task 2: Extract RunningAgentRegistry trait, create SQLite impl, wire everything",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(agents): SQLite-backed RunningAgentRegistry with trait extraction",
    "steps": [
      "Read specs/plans/fix_duplicate_agent_processes_on_app_restart.md section 'Task 2' (all substeps 2a-2d)",
      "2a: Extract #[async_trait] pub trait RunningAgentRegistry in running_agent_registry.rs, rename concrete struct to MemoryRunningAgentRegistry, update re-exports in domain/services/mod.rs",
      "2b: Create infrastructure/sqlite/sqlite_running_agent_registry.rs implementing the trait with SQL operations, export from infrastructure/sqlite/mod.rs",
      "2c: Update ALL call sites (17+ files) from Arc<RunningAgentRegistry> to Arc<dyn RunningAgentRegistry> — see plan for complete file list",
      "2d: Wire SqliteRunningAgentRegistry in AppState::new_production() and with_db_path(), keep MemoryRunningAgentRegistry for new_test() and with_repos()",
      "Add tests: test_register_and_get, test_unregister, test_is_running, test_list_all, test_stop_all_clears_table",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(agents): SQLite-backed RunningAgentRegistry with trait extraction"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add startup kill-orphans from persisted SQLite registry",
    "plan_section": "Task 3: Add startup kill-orphans from persisted registry",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "feat(startup): kill orphaned agent processes from SQLite registry on restart",
    "steps": [
      "Read specs/plans/fix_duplicate_agent_processes_on_app_restart.md section 'Task 3'",
      "Store running_agent_registry as a field on StartupJobRunner struct (currently only passed to ReconciliationRunner)",
      "At the start of run(), call self.running_agent_registry.stop_all().await BEFORE cancel_all_running()",
      "Log killed count with info! macro",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(startup): kill orphaned agent processes from SQLite registry on restart"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Guard ChatResumptionRunner against terminal-state tasks (merged/failed/cancelled/stopped)",
    "plan_section": "Task 4: Guard ChatResumptionRunner against terminal-state tasks",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(chat-resumption): skip terminal-state tasks on startup",
    "steps": [
      "Read specs/plans/fix_duplicate_agent_processes_on_app_restart.md section 'Task 4'",
      "4a: Add is_terminal() method to InternalStatus in status.rs — returns true for Merged, Failed, Cancelled, Stopped",
      "Add test_is_terminal test verifying exactly these 4 statuses",
      "4b: Update is_handled_by_task_resumption() in chat_resumption.rs to return true (skip) for terminal states",
      "Add tests: test_is_handled_for_merged_task, test_is_handled_for_failed_task, test_is_handled_for_cancelled_task, test_is_handled_for_stopped_task",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(chat-resumption): skip terminal-state tasks on startup"
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
| **Trait extraction for RunningAgentRegistry** | Allows SQLite impl for production and Memory impl for tests — clean DI |
| **SQLite persistence (not file-based)** | Already have shared DB connection; avoids new file management complexity |
| **Kill orphans BEFORE cancel_all_running()** | Must SIGTERM processes before marking DB records as cancelled |
| **is_terminal() on InternalStatus** | Reusable method — other code paths may need terminal-state checks |
| **Task 4 independent of Tasks 1-3** | Different bug, self-contained fix — can be parallelized |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] v17 migration creates running_agents table (idempotent)
- [ ] SqliteRunningAgentRegistry register/unregister/get/list/stop_all work correctly
- [ ] InternalStatus::is_terminal() returns true for exactly 4 statuses
- [ ] ChatResumptionRunner skips terminal-state tasks

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Build succeeds (`cargo build --release`)

### Manual Testing
- [ ] Start app with task in `executing`, restart → verify only ONE worker process, old one killed
- [ ] Verify no agents spawn for `merged`/`failed`/`cancelled` tasks on restart
- [ ] Multiple quick restarts (3x) → verify no process accumulation

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] SqliteRunningAgentRegistry is constructed in AppState::new_production()
- [ ] StartupJobRunner.run() calls stop_all() before cancel_all_running()
- [ ] is_terminal() is called in is_handled_by_task_resumption()

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No functions exported but never called
- [ ] Arc<dyn RunningAgentRegistry> used consistently (no remaining Arc<RunningAgentRegistry> concrete refs)

See `.claude/rules/gap-verification.md` for full verification workflow.
