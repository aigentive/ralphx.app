# Fix: Duplicate Agent Processes on App Restart

## Context

When the RalphX app restarts (including multiple quick restarts), orphaned Claude agent OS processes accumulate:
- **3 worker processes** for task `b9d31bfc` (all doing the same work, all writing to the same worktree)
- **1 reviewer process** for task `3cf42c48` which is already in `merged` status

**Root cause:** Two independent bugs in the startup recovery pipeline.

## Bug Analysis

### Bug 1: Orphaned OS processes not killed on restart (PRIMARY)

`RunningAgentRegistry` is **in-memory only** (`HashMap<RunningAgentKey, RunningAgentInfo>` storing PIDs). On app restart:
1. Registry is lost (fresh empty HashMap)
2. `cancel_all_running()` marks DB `agent_runs` as cancelled — but **does NOT kill OS processes**
3. `StartupJobRunner` spawns NEW agents for tasks still in agent-active states
4. Old Claude CLI processes continue running as orphans alongside new ones

With N restarts → up to N+1 processes per active task.

### Bug 2: ChatResumptionRunner resumes conversations for terminal-state tasks

`is_handled_by_task_resumption()` (chat_resumption.rs:191-228) only returns `true` (skip) when task is in `AGENT_ACTIVE_STATUSES`. For terminal states like `merged`, the method returns `false` → ChatResumptionRunner resumes the old conversation, spawning an agent for a completed task.

## Compilation Unit Analysis

**Critical constraint:** Renaming `RunningAgentRegistry` (struct) to a trait and renaming the concrete struct to `MemoryRunningAgentRegistry` changes the meaning of `Arc<RunningAgentRegistry>` at **17+ call sites**. All of these must be updated atomically — the code will not compile if the trait exists but call sites still reference the old concrete struct. Steps 1a-1e in the original plan MUST be a single task.

Fix 2 (terminal-state guard) is independent — `is_terminal()` is additive and `chat_resumption.rs` changes are self-contained. Can be a separate task with no dependency on Fix 1.

## Implementation Tasks

### Task 1: Add DB migration for running_agents table (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(db): add v17 running_agents migration`

**File:** `src-tauri/src/infrastructure/sqlite/migrations/v17_running_agents.rs`

```sql
CREATE TABLE IF NOT EXISTS running_agents (
    context_type TEXT NOT NULL,
    context_id TEXT NOT NULL,
    pid INTEGER NOT NULL,
    conversation_id TEXT NOT NULL,
    agent_run_id TEXT NOT NULL,
    started_at TEXT NOT NULL,
    PRIMARY KEY (context_type, context_id)
);
```

Register in `MIGRATIONS` array (migrations/mod.rs), bump `SCHEMA_VERSION` to 17.
Add migration test: `v17_running_agents_tests.rs` — table creation idempotency.

### Task 2: Extract RunningAgentRegistry trait, create SQLite impl, wire everything (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(agents): SQLite-backed RunningAgentRegistry with trait extraction`

**This is the largest task — it MUST be atomic because renaming the struct breaks all 17+ call sites simultaneously.**

**Substeps (all in one commit):**

**2a. Extract trait + rename in-memory struct:**

**File:** `src-tauri/src/domain/services/running_agent_registry.rs`

- Extract `#[async_trait] pub trait RunningAgentRegistry: Send + Sync { ... }` with 7 methods
- Rename current `RunningAgentRegistry` struct → `MemoryRunningAgentRegistry`
- Implement `RunningAgentRegistry` trait for `MemoryRunningAgentRegistry`
- Update re-exports in `domain/services/mod.rs` to export trait + `MemoryRunningAgentRegistry`

**2b. Create SQLite implementation:**

**File:** `src-tauri/src/infrastructure/sqlite/sqlite_running_agent_registry.rs`

New struct `SqliteRunningAgentRegistry` implementing `RunningAgentRegistry` trait:

```rust
pub struct SqliteRunningAgentRegistry {
    conn: Arc<Mutex<Connection>>,
}
```

Methods map to SQL:
| Method | SQL |
|--------|-----|
| `register()` | `INSERT OR REPLACE INTO running_agents ...` |
| `unregister()` | `SELECT ... WHERE; DELETE ... WHERE` (return info) |
| `is_running()` | `SELECT 1 FROM running_agents WHERE ...` |
| `get()` | `SELECT ... WHERE ...` |
| `stop()` | `unregister()` + `kill -TERM {pid}` (reuse existing SIGTERM logic from current `stop()`) |
| `list_all()` | `SELECT * FROM running_agents` |
| `stop_all()` | `SELECT * FROM running_agents; DELETE FROM running_agents` + kill each |

Export from `infrastructure/sqlite/mod.rs`.

Add tests: `test_register_and_get`, `test_unregister`, `test_is_running`, `test_list_all`, `test_stop_all_clears_table`.

**2c. Update all call sites** (`Arc<RunningAgentRegistry>` → `Arc<dyn RunningAgentRegistry>`):

| File | Lines Affected | Change |
|------|---------------|--------|
| `app_state.rs` | :121 | Field type `Arc<dyn RunningAgentRegistry>` |
| `app_state.rs` | :233, :333, :381, :431 | `Arc::new(SqliteRunningAgentRegistry::new(conn))` for prod, `Arc::new(MemoryRunningAgentRegistry::new())` for test |
| `chat_service/mod.rs` | :183, :201 | Constructor param + field type |
| `chat_service_send_background.rs` | :86, :643, :996, :1034, :1091 | Param types |
| `reconciliation.rs` | :253, :274 | Constructor param + field type |
| `chat_resumption.rs` | :42, :61 | Field type + constructor param |
| `session_reopen_service.rs` | :24, :34 | Constructor param + field type |
| `task_transition_service.rs` | :393 | Param type |
| `task_scheduler_service.rs` | :56, :79 | Field type + constructor param |
| `startup_jobs.rs` | :74 | Constructor param (already takes it but as concrete type) |
| `lib.rs` | :185, :211, :225, :232, :245, :263, :279, :304 | All `Arc::clone()` calls — no change needed (they clone `Arc<dyn ...>`) |
| `ideation_commands_session.rs` | (via `state.running_agent_registry`) | No type annotation change needed (inferred) |
| `execution_commands.rs` | (via `app_state.running_agent_registry`) | No type annotation change needed (inferred) |

**2d. Wire SQLite impl in AppState:**

| Factory | Implementation |
|---------|---------------|
| `new_production()` | `Arc::new(SqliteRunningAgentRegistry::new(Arc::clone(&shared_conn)))` |
| `with_db_path()` | `Arc::new(SqliteRunningAgentRegistry::new(Arc::clone(&shared_conn)))` |
| `new_test()` | `Arc::new(MemoryRunningAgentRegistry::new())` |
| `with_repos()` | `Arc::new(MemoryRunningAgentRegistry::new())` |

### Task 3: Add startup kill-orphans from persisted registry
**Dependencies:** Task 2
**Atomic Commit:** `feat(startup): kill orphaned agent processes from SQLite registry on restart`

**File:** `src-tauri/src/application/startup_jobs.rs`

The `StartupJobRunner` already receives `running_agent_registry` in its constructor (line 74). After Task 2, this is `Arc<dyn RunningAgentRegistry>` backed by SQLite. The registry field is passed to `ReconciliationRunner` at line 90 but NOT stored on `self`.

**Changes:**
1. Store `running_agent_registry` as a field on `StartupJobRunner` struct (currently only passed through to `ReconciliationRunner`)
2. At the start of `run()`, call `self.running_agent_registry.stop_all().await` BEFORE `cancel_all_running()`:

```rust
pub async fn run(&self) {
    // Kill orphaned agent processes from previous session.
    // The SQLite-backed registry persists PIDs across restarts.
    let killed = self.running_agent_registry.stop_all().await;
    if !killed.is_empty() {
        info!(count = killed.len(), "Killed orphaned agent processes from previous session");
    }

    // Then clean up orphaned agent runs in DB
    match self.agent_run_repo.cancel_all_running().await {
    // ... existing code
```

### Task 4: Guard ChatResumptionRunner against terminal-state tasks
**Dependencies:** None (independent of Tasks 1-3)
**Atomic Commit:** `fix(chat-resumption): skip terminal-state tasks on startup`

**4a. Add `is_terminal()` to `InternalStatus`:**

**File:** `src-tauri/src/domain/entities/status.rs`

```rust
pub fn is_terminal(&self) -> bool {
    matches!(self, Self::Merged | Self::Failed | Self::Cancelled | Self::Stopped)
}
```

Add test: `test_is_terminal` — verify exactly these 4 statuses return true, all others false.

**4b. Update `is_handled_by_task_resumption()`:**

**File:** `src-tauri/src/application/chat_resumption.rs:191-228`

```rust
Ok(Some(task)) => {
    let is_agent_active = AGENT_ACTIVE_STATUSES.contains(&task.internal_status);
    if is_agent_active {
        info!(/* existing log */);
        return true;
    }
    // Skip terminal states — task is done, no agent needed
    if task.internal_status.is_terminal() {
        info!(
            task_id = task.id.as_str(),
            status = ?task.internal_status,
            "[CHAT_RESUMPTION] Task in terminal state, skipping"
        );
        return true;
    }
    false
}
```

Add tests: `test_is_handled_for_merged_task`, `test_is_handled_for_failed_task`, `test_is_handled_for_cancelled_task`, `test_is_handled_for_stopped_task`.

## Dependency Graph

```
Task 1 (migration)     Task 4 (terminal guard)
    ↓                      (independent)
Task 2 (trait + impl + wiring)
    ↓
Task 3 (startup kill)
```

Tasks 1→2→3 are sequential (each depends on prior).
Task 4 can be done in parallel with any of 1-3.

## Files to Modify

| File | Change | Task |
|------|--------|------|
| `src-tauri/src/infrastructure/sqlite/migrations/v17_running_agents.rs` | New migration | 1 |
| `src-tauri/src/infrastructure/sqlite/migrations/v17_running_agents_tests.rs` | Migration test | 1 |
| `src-tauri/src/infrastructure/sqlite/migrations/mod.rs` | Register v17, bump SCHEMA_VERSION | 1 |
| `src-tauri/src/domain/services/running_agent_registry.rs` | Extract trait, rename struct | 2 |
| `src-tauri/src/domain/services/mod.rs` | Re-export trait + `MemoryRunningAgentRegistry` | 2 |
| `src-tauri/src/infrastructure/sqlite/sqlite_running_agent_registry.rs` | New SQLite impl | 2 |
| `src-tauri/src/infrastructure/sqlite/mod.rs` | Export new module | 2 |
| `src-tauri/src/application/app_state.rs` | Field type → `Arc<dyn ...>`, wire SQLite impl | 2 |
| `src-tauri/src/application/chat_service/mod.rs` | `Arc<dyn RunningAgentRegistry>` | 2 |
| `src-tauri/src/application/chat_service/chat_service_send_background.rs` | `Arc<dyn RunningAgentRegistry>` | 2 |
| `src-tauri/src/application/reconciliation.rs` | `Arc<dyn RunningAgentRegistry>` | 2 |
| `src-tauri/src/application/chat_resumption.rs` | `Arc<dyn RunningAgentRegistry>` + terminal guard | 2, 4 |
| `src-tauri/src/application/session_reopen_service.rs` | `Arc<dyn RunningAgentRegistry>` | 2 |
| `src-tauri/src/application/task_transition_service.rs` | `Arc<dyn RunningAgentRegistry>` | 2 |
| `src-tauri/src/application/task_scheduler_service.rs` | `Arc<dyn RunningAgentRegistry>` | 2 |
| `src-tauri/src/application/startup_jobs.rs` | `Arc<dyn RunningAgentRegistry>` + store field + kill on start | 2, 3 |
| `src-tauri/src/domain/entities/status.rs` | Add `is_terminal()` method + test | 4 |

## Verification

1. `cargo check` — compiles
2. `cargo test` — all tests pass (existing + new)
3. `cargo clippy --all-targets --all-features -- -D warnings` — no warnings
4. Manual: Start app with task in `executing`, restart → verify only ONE worker process, old one killed
5. Manual: Verify no agents spawn for `merged`/`failed`/`cancelled` tasks on restart

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
