# Fix: FK constraint failed in apply_proposals_to_kanban — Add ideation_session_id to tasks

## Context

`apply_proposals_to_kanban` fails with "FOREIGN KEY constraint failed" when the ideation session has no `plan_artifact_id`. The code at line 168-172 falls back to using the session UUID as a fake artifact ID, then writes it to `tasks.plan_artifact_id` which has `REFERENCES artifacts(id)`. The session UUID doesn't exist in `artifacts`.

**Root cause**: Tasks link to plans via `plan_artifact_id` (optional, FK-constrained), but sessions always exist while artifacts may not. The session_id fallback puts a non-artifact ID into an artifact FK column.

**Fix**: Add `ideation_session_id` column to tasks — a direct, always-valid link to the session. Stop faking `plan_artifact_id` with session IDs.

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/infrastructure/sqlite/migrations/v15_task_ideation_session_id.rs` | **NEW** — migration: add column + backfill |
| `src-tauri/src/infrastructure/sqlite/migrations/v15_task_ideation_session_id_tests.rs` | **NEW** — migration tests |
| `src-tauri/src/infrastructure/sqlite/migrations/mod.rs` | Register v15, bump SCHEMA_VERSION to 15 |
| `src-tauri/src/domain/entities/task.rs` | Add `ideation_session_id: Option<IdeationSessionId>` field, update `new()`, `from_row()` |
| `src-tauri/src/infrastructure/sqlite/sqlite_task_repo/queries.rs` | Add `ideation_session_id` to `TASK_COLUMNS` + all hardcoded SELECTs |
| `src-tauri/src/infrastructure/sqlite/sqlite_task_repo/mod.rs` | Add param to `create()` INSERT (?20) and `update()` SET (?18) |
| `src-tauri/src/commands/ideation_commands/ideation_commands_apply.rs` | Fix the bug: set `ideation_session_id` on tasks, only use real `plan_artifact_id` |
| `src-tauri/src/commands/task_commands/query.rs` | Add 3rd pass: group tasks by `ideation_session_id` when `plan_artifact_id` is NULL |
| `src-tauri/src/commands/plan_branch_commands.rs` | Set `ideation_session_id` on merge task + backfilled tasks |

## Compilation Unit Analysis

Tasks 1-3 (Migration + Entity + Repo) must be in a single commit — repo queries reference `ideation_session_id` column which requires both the migration (DB column) and entity (`from_row` field) to exist. The entity `from_row()` calls `row.get("ideation_session_id")` which fails if the column doesn't exist, and the INSERT/UPDATE in the repo reference `task.ideation_session_id` which doesn't exist without the entity change.

Tasks 4, 5, 6 are all additive — they set or read `task.ideation_session_id` which is `Option`, so they compile independently as long as Task 1-3 is done first.

**Note:** `mod.rs` hardcoded SELECT queries in `get_by_status`, `get_next_executable`, `get_blockers`, `get_dependents`, `archive`, `restore` also enumerate columns explicitly and MUST be updated alongside `queries.rs`.

## Implementation

### 1. Migration v15 (BLOCKING)
**Dependencies:** None
**Atomic Commit:** Combined with Tasks 2 and 3 — single compilation unit

```rust
// v15_task_ideation_session_id.rs
helpers::add_column_if_not_exists(conn, "tasks", "ideation_session_id", "TEXT DEFAULT NULL")?;

// Backfill from proposals: task → proposal.created_task_id → proposal.session_id
conn.execute(
    "UPDATE tasks SET ideation_session_id = (
        SELECT tp.session_id FROM task_proposals tp WHERE tp.created_task_id = tasks.id LIMIT 1
    ) WHERE ideation_session_id IS NULL
      AND EXISTS (SELECT 1 FROM task_proposals tp WHERE tp.created_task_id = tasks.id)", [],
)?;
```

Register in `mod.rs`: add to `MIGRATIONS` array, bump `SCHEMA_VERSION` to 15.

### 2. Task Entity (`task.rs`) (BLOCKING)
**Dependencies:** None (additive field, but same compilation unit as Task 1 & 3)
**Atomic Commit:** Combined with Tasks 1 and 3

Add field after `plan_artifact_id`:
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub ideation_session_id: Option<IdeationSessionId>,
```
- Import `IdeationSessionId` from `super::super::entities::types`
- `new()`: add `ideation_session_id: None`
- `from_row()`: add `ideation_session_id: row.get::<_, Option<String>>("ideation_session_id")?.map(IdeationSessionId::from_string)`

### 3. Repository Queries (`queries.rs` + `mod.rs`) (BLOCKING)
**Dependencies:** Task 2 (entity field must exist)
**Atomic Commit:** `fix(tasks): add ideation_session_id column, entity field, and repo queries` (Tasks 1+2+3 together)

- `TASK_COLUMNS`: append `, ideation_session_id`
- All hardcoded `SELECT ... FROM tasks` in `GET_BY_ID`, `GET_BY_PROJECT`, `GET_OLDEST_READY_TASK`, `GET_OLDEST_READY_TASKS`: append `, ideation_session_id`
- **Also** all inline SELECTs in `mod.rs`: `get_by_status` (line 151), `get_next_executable` (line 228), `get_blockers` (line 253), `get_dependents` (line 274), `archive` (line 348), `restore` (line 372)
- `create()` INSERT: add column + param `?20` = `task.ideation_session_id.as_ref().map(|id| id.as_str())`
- `update()` SET: add `ideation_session_id = ?18`, renumber subsequent params, add to params array

### 4. Apply Command Fix (`ideation_commands_apply.rs`)
**Dependencies:** Tasks 1-3 (entity + repo must have ideation_session_id)
**Atomic Commit:** `fix(ideation): stop faking plan_artifact_id, use ideation_session_id`

**Line ~86** — Set `ideation_session_id` on each new task before creation:
```rust
task.ideation_session_id = Some(session_id.clone());
```

**Lines 168-173** — Replace fake artifact fallback:
```rust
// BEFORE (broken):
let plan_artifact_id: Option<ArtifactId> = Some(session.plan_artifact_id.clone().unwrap_or_else(|| ...session_id...));

// AFTER (correct):
let plan_artifact_id: Option<ArtifactId> = session.plan_artifact_id.clone();
```

Lines 176-197 now only run when `plan_artifact_id` is `Some` (real artifact exists). When None, tasks still have `ideation_session_id` for grouping.

**Line ~265** — Merge task also gets `ideation_session_id`:
```rust
merge_task.ideation_session_id = Some(session_id.clone());
```

### 5. Graph Query (`query.rs`)
**Dependencies:** Tasks 1-3 (entity must have ideation_session_id)
**Atomic Commit:** `fix(graph): group tasks by ideation_session_id when plan_artifact_id is NULL`

After the existing "5b. Second pass" (lines 574-601), add a 3rd pass:
```rust
// 5c. Catch tasks with ideation_session_id but no plan_artifact_id
// (tasks from sessions without a plan artifact)
{
    let grouped_task_ids: HashSet<String> = plan_groups.iter().flat_map(|g| g.task_ids.iter().cloned()).collect();
    let session_group_index: HashMap<String, usize> = plan_groups.iter().enumerate()
        .map(|(i, g)| (g.session_id.clone(), i)).collect();

    for task in &tasks {
        let task_id_str = task.id.as_str().to_string();
        if grouped_task_ids.contains(&task_id_str) { continue; }
        if let Some(sid) = task.ideation_session_id.as_ref().map(|id| id.as_str().to_string()) {
            if let Some(&idx) = session_group_index.get(&sid) {
                plan_groups[idx].task_ids.push(task_id_str);
                categorize_status(&task.internal_status, &mut plan_groups[idx].status_summary);
            }
        }
    }
}
```

### 6. Plan Branch Commands (`plan_branch_commands.rs`)
**Dependencies:** Tasks 1-3 (entity must have ideation_session_id)
**Atomic Commit:** `fix(plan-branch): set ideation_session_id on merge task and backfilled tasks`

In `enable_feature_branch` backfill loop (~line 182-191): also set `ideation_session_id` if None.
Merge task creation (~line 207): set `merge_task.ideation_session_id = Some(session_id.clone())`.

## What Stays the Same

- `PlanGroupInfo` struct — `plan_artifact_id` field stays as `String` (can hold session_id fallback for display)
- Frontend schemas/types — no changes needed (grouping handled server-side)
- `plan_branches` table — no FK, session_id fallback still safe there
- `TaskGraphNode` — `plan_artifact_id` remains optional display field

## Dependency Graph

```
Task 1 (Migration) ──┐
Task 2 (Entity)   ───┤── Single commit (compilation unit)
Task 3 (Repo)     ───┘
                       ├── Task 4 (Apply fix)     → separate commit
                       ├── Task 5 (Graph query)   → separate commit
                       └── Task 6 (Plan branch)   → separate commit
```

Tasks 4, 5, 6 are independent of each other and can be done in any order after 1-3.

## File Analysis Notes (from code review)

**`sqlite_task_repo/mod.rs` has 10+ hardcoded SELECT column lists** (not using `TASK_COLUMNS`):
- `get_by_status` (line 151)
- `get_next_executable` (line 228)
- `get_blockers` (line 253)
- `get_dependents` (line 274)
- `archive` (line 348)
- `restore` (line 372)

These MUST all include `ideation_session_id` or `from_row()` will fail at runtime when parsing the new field. This was not originally in the plan's Task 3 scope but is critical for compilation.

**`IdeationSessionId` is already re-exported** from `domain::entities::mod.rs` (line 45), so no new import needed in `task.rs` — just use `super::IdeationSessionId` via the existing import path.

**`task.rs` test helper `setup_test_db()`** (line 625-652) creates a local in-memory table schema — must be updated to include `ideation_session_id TEXT` column.

## Verification

1. `cargo check` — compiles
2. `cargo clippy --all-targets --all-features -- -D warnings` — no warnings
3. `cargo test` — all tests pass (migration tests + existing)
4. Manual: apply proposals from session with `plan_artifact_id = NULL` — no FK error
5. Verify graph view groups tasks correctly by session when no artifact exists

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
