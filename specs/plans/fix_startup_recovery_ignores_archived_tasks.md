# Fix: Startup Recovery Ignores Archived Tasks

## Context

On app startup, `StartupJobRunner` re-triggers entry actions for tasks stuck in agent-active states (`Executing`, `Reviewing`, `Merging`, etc.) and auto-transition states (`PendingMerge`, `Approved`, etc.). The underlying `get_by_status()` query has **no `archived_at IS NULL` filter**, so archived/soft-deleted test tasks get re-triggered every startup — spawning agents, attempting merges, and polluting logs.

The user's logs show 6+ archived test tasks in `PendingMerge` being re-triggered on every app launch.

**No "deleted" field exists** — `archived_at` is the only soft-delete mechanism. Hard-deleted tasks are gone from the DB. Abandoned plan branches already hard-delete their merge tasks, so that path is safe.

## Fix (2 layers)

### Layer 1: Fix `get_by_status` query (root cause) (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(task-repo): exclude archived tasks from get_by_status query`

**File:** `src-tauri/src/infrastructure/sqlite/sqlite_task_repo/mod.rs:173`

Add `AND archived_at IS NULL` to the SQL WHERE clause:

```sql
-- Before:
FROM tasks WHERE project_id = ?1 AND internal_status = ?2

-- After:
FROM tasks WHERE project_id = ?1 AND internal_status = ?2 AND archived_at IS NULL
```

This fixes all callers at once (startup recovery, reconciliation, blocker unblocking, queue counting, pause listing). No caller of `get_by_status` would ever want archived tasks — they're soft-deleted.

**File:** `src-tauri/src/infrastructure/memory/memory_task_repo/mod.rs:124`

Add archived filter to the memory implementation too (for test parity):

```rust
// Before:
.filter(|t| t.project_id == *project_id && t.internal_status == status)

// After:
.filter(|t| t.project_id == *project_id && t.internal_status == status && t.archived_at.is_none())
```

### Layer 2: Defense-in-depth in startup_jobs.rs
**Dependencies:** Layer 1
**Atomic Commit:** `fix(startup): skip archived tasks in startup recovery loops`

**File:** `src-tauri/src/application/startup_jobs.rs`

Add explicit skip + log in both recovery loops (agent-active at ~line 247, auto-transition at ~line 306) for archived tasks. This is a safety net in case `get_by_status` is ever changed or a different query path is used:

```rust
if task.archived_at.is_some() {
    eprintln!("[STARTUP] Skipping archived task: {} ({})", task.id.as_str(), task.title);
    continue;
}
```

Also add the same check in the blocker unblock loop (~line 370).

### Tests
**Dependencies:** Layer 1 (tests validate the query fix)
**Atomic Commit:** `test(task-repo): add archived exclusion tests for get_by_status`

**File:** `src-tauri/src/infrastructure/sqlite/sqlite_task_repo/tests.rs`

Add test `test_get_by_status_excludes_archived` that:
1. Creates two tasks in `PendingMerge` status — one active, one archived
2. Archives one (`repo.archive(&task.id)`)
3. Calls `get_by_status(project_id, PendingMerge)`
4. Asserts only the non-archived task is returned

**File:** `src-tauri/src/infrastructure/memory/memory_task_repo/tests.rs`

Same test for the memory implementation.

## Files Modified

| File | Change |
|------|--------|
| `src-tauri/src/infrastructure/sqlite/sqlite_task_repo/mod.rs` | Add `AND archived_at IS NULL` to `get_by_status` query |
| `src-tauri/src/infrastructure/memory/memory_task_repo/mod.rs` | Add `archived_at.is_none()` filter |
| `src-tauri/src/application/startup_jobs.rs` | Add archived task skip in 3 loops |
| `src-tauri/src/infrastructure/sqlite/sqlite_task_repo/tests.rs` | Add archived exclusion test |
| `src-tauri/src/infrastructure/memory/memory_task_repo/tests.rs` | Add archived exclusion test |

## Verification

1. `cargo test -- get_by_status` — run existing + new tests
2. `cargo clippy --all-targets --all-features -- -D warnings` — no warnings
3. Manual: restart app, confirm no `[STARTUP] Re-triggering` log for archived tasks

## Compilation Unit Analysis

All 3 sections (Layer 1, Layer 2, Tests) are additive — no renames, no signature changes, no removed exports. Each compiles independently:

- **Layer 1** adds a WHERE clause and a filter predicate — existing callers unaffected
- **Layer 2** adds `continue` guards inside existing loops — purely additive
- **Tests** add new test functions — no existing code modified

No chicken-egg issues detected. Tasks can be executed in dependency order safely.

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
