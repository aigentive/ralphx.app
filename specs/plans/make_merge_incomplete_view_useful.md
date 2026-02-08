# Plan: Make MergeIncomplete View Useful

## Context

The `MergeIncompleteTaskDetail` view is essentially useless. When a merge fails with a non-conflict git error, the view shows:
- Generic hardcoded bullet points ("branch deleted or corrupted", "git lock file", etc.)
- "Branch: task branch" literally (because `task.taskBranch` is often null)
- No actual error message from the git operation
- No source/target branch information
- Retry fails again with zero diagnostic info

**Root cause**: When `attempt_programmatic_merge()` fails, the error message and branch names are only logged via `tracing` but never persisted to the task. The tasks table has no `metadata` column, and the Rust Task entity has no `metadata` field — so the frontend has nothing to display.

## Pre-Implementation Notes

**Frontend already wired:** `TaskSchema`, `Task` interface, and `transformTask` in `src/types/task.ts` already include `metadata` field. No frontend schema/type changes needed.

**Mock repo needs no change:** `MockTaskRepository` returns `Task::new()` which will get `metadata: None` automatically. No field-specific logic in mock.

**`Task::from_row()` uses named columns:** Adding `metadata` won't break column index ordering — `row.get("metadata")` is safe.

**Inline SQL queries in `mod.rs`:** Beyond `queries.rs` constants, `mod.rs` has ~10 hardcoded SELECT column lists (in `get_by_status`, `get_next_executable`, `get_blockers`, `get_dependents`, `archive`, `restore`). ALL must include `metadata`.

## Changes

### 1. Backend metadata plumbing — migration, entity, queries, repo, response (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(task): add metadata column to tasks table`

Single compilation unit — all must change together for `Task::from_row()` to work at runtime.

**Files** (all in `src-tauri/src/`):

| File | Change |
|------|--------|
| `infrastructure/sqlite/migrations/v18_task_metadata.rs` | **NEW** — `add_column_if_not_exists(conn, "tasks", "metadata", "TEXT DEFAULT NULL")` |
| `infrastructure/sqlite/migrations/v18_task_metadata_tests.rs` | **NEW** — column exists, can set/get, idempotent |
| `infrastructure/sqlite/migrations/mod.rs` | Register v18, bump `SCHEMA_VERSION` to 18 |
| `domain/entities/task.rs` | Add `pub metadata: Option<String>` after `merge_commit_sha`, set `None` in `new()`, add `row.get("metadata")?` in `from_row()`, add `metadata` column to `setup_test_db()` |
| `infrastructure/sqlite/sqlite_task_repo/queries.rs` | Append `, metadata` to `TASK_COLUMNS` and all 5 query constants |
| `infrastructure/sqlite/sqlite_task_repo/mod.rs` | Add `metadata` to INSERT (col list + `?21` param), UPDATE (`metadata = ?19` + param), and all ~10 inline SELECT column lists (`get_by_status`, `get_next_executable`, `get_blockers`, `get_dependents`, `archive` x2, `restore` x2) |
| `commands/task_commands/types.rs` | Add `pub metadata: Option<String>` to `TaskResponse`, add `metadata: task.metadata` in `From<Task>` impl |

**Why single task:** `Task` struct gains `metadata` field → `from_row()` reads `row.get("metadata")` → all SELECT queries must include it or runtime crash. INSERT/UPDATE must bind it or data loss. `TaskResponse` must expose it or frontend never sees it. These cannot be split.

### 2. Persist error context on programmatic merge failure
**Dependencies:** Task 1
**Atomic Commit:** `feat(merge): persist error context to task metadata on merge failure`

**File**: `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`

Two error paths need updating:

**a) Worktree mode error (line ~1079-1118)**:
Before `task.internal_status = InternalStatus::MergeIncomplete`, set:
```rust
task.metadata = Some(serde_json::json!({
    "error": e.to_string(),
    "source_branch": source_branch,
    "target_branch": target_branch,
}).to_string());
```

**b) Local mode error (line ~1202-1236)**:
Same pattern — store error, source_branch, target_branch in metadata.

### 3. Persist error context from agent report_incomplete
**Dependencies:** Task 1
**Atomic Commit:** `feat(merge): persist agent error context to task metadata`

**File**: `src-tauri/src/http_server/handlers/git.rs` (report_incomplete handler, line ~352-426)

After getting the task and before transitioning, update task metadata:
```rust
let mut task = task; // make mutable
task.metadata = Some(serde_json::json!({
    "error": req.reason,
    "diagnostic_info": req.diagnostic_info,
}).to_string());
task_repo.update(&task).await?;
```

### 4. Frontend — Display actual error details
**Dependencies:** Task 1
**Atomic Commit:** `feat(merge-ui): show actual error details in MergeIncomplete view`

**File**: `src/components/tasks/detail-views/MergeIncompleteTaskDetail.tsx`

Replace the generic `ErrorContextCard` with one that parses task metadata:

```typescript
const mergeError = (() => {
  if (!task.metadata) return null;
  try {
    const m = JSON.parse(task.metadata);
    return {
      error: m.error ?? null,
      sourceBranch: m.source_branch ?? null,
      targetBranch: m.target_branch ?? null,
      diagnosticInfo: m.diagnostic_info ?? null,
    };
  } catch { return null; }
})();
```

Display:
- **Error message** in a red-tinted code block (the actual git error)
- **Source branch → Target branch** clearly labeled (e.g., `ralphx/my-app/task-abc → main`)
- **Diagnostic info** if present (from agent reports)
- Fall back to current generic text if metadata is null (backwards compat)

Update `RecoverySteps` to use the actual branch name from metadata or `task.taskBranch`.

## Task Dependency Graph

```
Task 1: Backend metadata plumbing (BLOCKING)
  ├─ Task 2: Persist error on programmatic merge failure
  ├─ Task 3: Persist error from agent report_incomplete
  └─ Task 4: Frontend display actual error details
```

Tasks 2, 3, 4 are independent of each other — can be done in any order after Task 1.

## Files Modified

| File | Change | Task |
|------|--------|------|
| `src-tauri/src/infrastructure/sqlite/migrations/v18_task_metadata.rs` | **NEW** — migration | 1 |
| `src-tauri/src/infrastructure/sqlite/migrations/v18_task_metadata_tests.rs` | **NEW** — migration test | 1 |
| `src-tauri/src/infrastructure/sqlite/migrations/mod.rs` | Register v18, bump SCHEMA_VERSION | 1 |
| `src-tauri/src/domain/entities/task.rs` | Add `metadata` field + `from_row` + `new()` + test helper | 1 |
| `src-tauri/src/infrastructure/sqlite/sqlite_task_repo/queries.rs` | Add `metadata` to all SELECTs | 1 |
| `src-tauri/src/infrastructure/sqlite/sqlite_task_repo/mod.rs` | Include in INSERT/UPDATE + all inline SELECTs | 1 |
| `src-tauri/src/commands/task_commands/types.rs` | Add to TaskResponse + From impl | 1 |
| `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` | Store error context on merge failure | 2 |
| `src-tauri/src/http_server/handlers/git.rs` | Store agent error context | 3 |
| `src/components/tasks/detail-views/MergeIncompleteTaskDetail.tsx` | Show real error details | 4 |

**Not modified (already wired):**
- `src/types/task.ts` — `TaskSchema`, `Task`, `transformTask` already include `metadata`
- `src-tauri/src/domain/repositories/task_repository.rs` — `MockTaskRepository` uses `Task::new()` which auto-gets `metadata: None`
- `src-tauri/src/infrastructure/sqlite/sqlite_task_repo/helpers.rs` — `Task::from_row()` handles row mapping (no separate `row_to_task()` exists)

## Compilation Unit Analysis

**Task 1 is a single compilation unit because:**
1. Adding `metadata` to `Task` struct → `Task::new()` must initialize it (compile error)
2. `Task::from_row()` calls `row.get("metadata")` → all SQL queries must SELECT it (runtime crash)
3. INSERT/UPDATE must bind `task.metadata` → new param index (data loss if missing)
4. `TaskResponse` must expose it → frontend needs to receive it

Splitting any of these into separate tasks creates a broken intermediate state.

**Tasks 2-4 are safe to split because:**
- They are purely additive (setting `task.metadata = Some(...)` where it was `None` before)
- No renames, no signature changes, no removed exports
- Each task compiles independently after Task 1

## Verification

1. `cargo check` — compiles
2. `cargo clippy --all-targets --all-features -- -D warnings` — no warnings
3. `cargo test` — migration tests pass
4. `npm run typecheck` — frontend types match
5. `npm run lint` — clean
6. Manual: trigger a merge failure → view shows actual error, branch names, and error message

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
