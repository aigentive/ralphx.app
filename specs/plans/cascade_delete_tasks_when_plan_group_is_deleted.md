# Plan: Cascade Delete Tasks When Plan Group is Deleted

## Context

When pressing Backspace on a plan group in the task graph, the ideation session is deleted but its tasks survive as orphans — they appear under "Uncategorized" instead of being deleted. The confirmation dialog already says "This will permanently delete [plan name] and all N tasks", so the expected behavior is clear: delete the group AND the tasks.

**Root cause**: Migration v15 added `ideation_session_id TEXT DEFAULT NULL` to the tasks table with no FK constraint. The `delete_ideation_session` command only does `DELETE FROM ideation_sessions WHERE id = ?1`, and CASCADE only covers proposals/messages (which have proper FK constraints), not tasks.

## Approach: Application-Level Cascade with Force-Stop

Add explicit cascade logic in the `delete_ideation_session` command. For tasks in active agent states, force-stop agents and transition to `Stopped` before deleting. The user already confirmed via the deletion dialog, so force-stopping is acceptable.

## Changes

### Task 1: Add `get_by_ideation_session` to TaskRepository trait + all implementations (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(task-repo): add get_by_ideation_session query method`

> **Compilation unit note:** Adding a trait method requires ALL implementations to compile. Trait definition, SQLite impl, memory impl, and mock stub must all be in the same task.

**Files:**
- `src-tauri/src/domain/repositories/task_repository.rs` — Add method to trait + mock stub
- `src-tauri/src/infrastructure/sqlite/sqlite_task_repo/mod.rs` — Implement query (`SELECT ... FROM tasks WHERE ideation_session_id = ?1`, follow `get_by_project` pattern)
- `src-tauri/src/infrastructure/sqlite/sqlite_task_repo/queries.rs` — Add `GET_BY_IDEATION_SESSION` constant
- `src-tauri/src/infrastructure/memory/memory_task_repo/mod.rs` — Filter tasks by `ideation_session_id` match

### Task 2: Update `delete_ideation_session` command (core fix)
**Dependencies:** Task 1
**Atomic Commit:** `fix(ideation): cascade delete tasks when deleting plan group`

**File**: `src-tauri/src/commands/ideation_commands/ideation_commands_session.rs:137-149`

Add params: `app: tauri::AppHandle`, `execution_state: State<'_, Arc<ExecutionState>>`

New logic:
1. Query all tasks for the session via `task_repo.get_by_ideation_session()`
2. **Force-stop any active agent tasks** — follow the `stop_execution` pattern from `execution_commands.rs:693-788`:
   - For each task in `AGENT_ACTIVE_STATUSES`: stop agent via `running_agent_registry.stop(RunningAgentKey::new("task_execution", task_id.as_str()))`
   - Build `TaskTransitionService`, transition each active task to `Stopped` (triggers on_exit handlers to decrement running_count)
3. Delete each task via `task_repo.delete()`, emit `task:deleted` event per task
4. Look up plan branch via `plan_branch_repo.get_by_session_id()` — if found:
   - Best-effort delete the git feature branch
   - Mark plan branch status as `Abandoned`
5. Delete the session (existing CASCADE handles proposals/messages)

### Task 3: Verify wiring (no code changes)
**Dependencies:** Task 2
**Atomic Commit:** N/A — verification only

- **Command signature:** No `lib.rs` change needed — Tauri handles added `app`/`execution_state` params via DI
- **Frontend invalidation:** `TaskGraphView.tsx:826` already calls `queryClient.invalidateQueries({ queryKey: taskGraphKeys.graphPrefix(projectId) })` — the `task:deleted` events from backend will also trigger event-driven updates
- **Lint/check:** `cargo clippy --all-targets --all-features -- -D warnings`

### Task 4: Tests
**Dependencies:** Task 2
**Atomic Commit:** `test(ideation): add cascade delete session tests`

- Unit test for `get_by_ideation_session` in sqlite task repo
- Test in `ideation_commands_session` tests: delete session with tasks → verify tasks are gone
- Test: delete session with active task → verify tasks get stopped then deleted

## Key Files

| File | Change |
|------|--------|
| `src-tauri/src/domain/repositories/task_repository.rs` | Add trait method + mock |
| `src-tauri/src/infrastructure/sqlite/sqlite_task_repo/mod.rs` | Implement query |
| `src-tauri/src/infrastructure/memory/memory_task_repo.rs` | Implement for tests |
| `src-tauri/src/commands/ideation_commands/ideation_commands_session.rs` | Core cascade logic |
| Existing test files for above | Add tests |

## Existing Code to Reuse

| What | Where |
|------|-------|
| `task_repo.delete(&task_id)` | Already exists in trait + impls |
| `plan_branch_repo.get_by_session_id()` | `sqlite_plan_branch_repo.rs:76` |
| `plan_branch_repo.update_status(id, Abandoned)` | `sqlite_plan_branch_repo.rs:133` |
| `GitService::delete_feature_branch()` | `git_service.rs` (static method) |
| `PlanBranchStatus::Abandoned` | `plan_branch.rs:50` |
| `AGENT_ACTIVE_STATUSES` | `execution_commands.rs:26` |
| `running_agent_registry.stop(key)` | `running_agent_registry.rs:101` |
| `TaskTransitionService::new(...)` pattern | `execution_commands.rs:713-727` |
| `transition_service.transition_task(id, Stopped)` | `execution_commands.rs:757-766` |

## Edge Cases

| Case | Handling |
|------|----------|
| Tasks in active agent states | Force-stop agents, transition to Stopped, then delete |
| Git branch doesn't exist | Best-effort delete, log warning, don't fail |
| Plan branch doesn't exist | Skip cleanup, continue with session delete |
| No tasks in session | Delete session directly (existing behavior) |
| Task transition to Stopped fails | Log warning, continue deleting (DB delete is priority) |

## Verification

1. `cargo clippy --all-targets --all-features -- -D warnings`
2. `cargo test` (unit + integration tests)
3. Manual: Create plan with tasks → Backspace on group → confirm → all tasks gone, no Uncategorized remnants
4. Manual: Start task execution → delete plan → agents stopped, all tasks deleted

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
