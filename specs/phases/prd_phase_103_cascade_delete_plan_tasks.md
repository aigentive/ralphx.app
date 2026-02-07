# RalphX - Phase 103: Cascade Delete Tasks When Plan Group is Deleted

## Overview

When pressing Backspace on a plan group in the task graph, the ideation session is deleted but its tasks survive as orphans — they appear under "Uncategorized" instead of being deleted. The confirmation dialog already says "This will permanently delete [plan name] and all N tasks", so the expected behavior is clear: delete the group AND the tasks.

**Root cause**: Migration v15 added `ideation_session_id TEXT DEFAULT NULL` to the tasks table with no FK constraint. The `delete_ideation_session` command only does `DELETE FROM ideation_sessions WHERE id = ?1`, and CASCADE only covers proposals/messages (which have proper FK constraints), not tasks.

**Reference Plan:**
- `specs/plans/cascade_delete_tasks_when_plan_group_is_deleted.md` - Application-level cascade with force-stop for active agent tasks

## Goals

1. Delete all tasks belonging to a session when the session is deleted
2. Force-stop any active agent tasks before deletion (user already confirmed via dialog)
3. Clean up plan branch (abandon + delete git feature branch) when session is deleted
4. Emit proper `task:deleted` events so the frontend updates in real-time

## Dependencies

### Phase 102 (Fix Merge Task Display) - Not Required

No direct dependency — this phase modifies backend command logic only.

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/cascade_delete_tasks_when_plan_group_is_deleted.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/cascade_delete_tasks_when_plan_group_is_deleted.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add get_by_ideation_session to TaskRepository trait + all implementations (compilation unit: trait + SQLite + memory + mock)",
    "plan_section": "Task 1: Add get_by_ideation_session to TaskRepository trait + all implementations",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(task-repo): add get_by_ideation_session query method",
    "steps": [
      "Read specs/plans/cascade_delete_tasks_when_plan_group_is_deleted.md section 'Task 1'",
      "Add IdeationSessionId import to task_repository.rs",
      "Add trait method: async fn get_by_ideation_session(&self, session_id: &IdeationSessionId) -> AppResult<Vec<Task>>",
      "Add mock stub in tests section MockTaskRepository impl",
      "Add GET_BY_IDEATION_SESSION query constant to queries.rs (follow GET_BY_PROJECT pattern)",
      "Implement in SqliteTaskRepository (follow get_by_project pattern)",
      "Implement in MemoryTaskRepository (filter by ideation_session_id match)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(task-repo): add get_by_ideation_session query method"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Update delete_ideation_session command with cascade delete: force-stop active agents, delete tasks, clean up plan branch",
    "plan_section": "Task 2: Update delete_ideation_session command (core fix)",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "fix(ideation): cascade delete tasks when deleting plan group",
    "steps": [
      "Read specs/plans/cascade_delete_tasks_when_plan_group_is_deleted.md section 'Task 2'",
      "Add imports: ExecutionState, AGENT_ACTIVE_STATUSES, TaskTransitionService, RunningAgentKey, GitService, PlanBranchStatus",
      "Add params to delete_ideation_session: app: tauri::AppHandle, execution_state: State<'_, Arc<ExecutionState>>",
      "Query tasks: task_repo.get_by_ideation_session(&session_id)",
      "For each task in AGENT_ACTIVE_STATUSES: stop agent via running_agent_registry.stop(), transition to Stopped via TaskTransitionService",
      "Delete each task via task_repo.delete(), emit task:deleted event per task",
      "Look up plan branch via plan_branch_repo.get_by_session_id() — if found: best-effort delete git feature branch, mark status Abandoned",
      "Delete the session (existing CASCADE handles proposals/messages)",
      "Verify: No lib.rs change needed (Tauri DI handles new params)",
      "Verify: Frontend invalidation already handled by task:deleted events + queryClient.invalidateQueries",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(ideation): cascade delete tasks when deleting plan group"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add tests for cascade delete: get_by_ideation_session query, session delete with tasks, session delete with active tasks",
    "plan_section": "Task 4: Tests",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "test(ideation): add cascade delete session tests",
    "steps": [
      "Read specs/plans/cascade_delete_tasks_when_plan_group_is_deleted.md section 'Task 4'",
      "Add unit test for get_by_ideation_session in SQLite task repo tests",
      "Add test: delete session with tasks → verify tasks are gone",
      "Add test: delete session with active task → verify agent stopped + tasks deleted",
      "Run cargo test",
      "Commit: test(ideation): add cascade delete session tests"
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
| **Application-level cascade (not DB FK)** | Tasks table has no FK to ideation_sessions (migration v15 design). Adding FK retroactively is risky. Application code gives full control over force-stop logic. |
| **Force-stop active agents before delete** | User already confirmed via the deletion dialog. Leaving agents running on deleted tasks would cause errors. Follow existing `stop_execution` pattern. |
| **Best-effort git branch cleanup** | Branch may not exist (task never executed). Don't fail the session delete if git cleanup fails. |
| **Emit per-task delete events** | Frontend uses event-driven updates. One event per task ensures React Query invalidation and real-time UI refresh. |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] `get_by_ideation_session` returns correct tasks filtered by session ID
- [ ] `delete_ideation_session` deletes all session tasks
- [ ] Active agent tasks are force-stopped before deletion

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### Manual Testing
- [ ] Create plan with tasks → Backspace on group → confirm → all tasks gone, no Uncategorized remnants
- [ ] Start task execution → delete plan → agents stopped, all tasks deleted
- [ ] Delete plan with no tasks → session deleted normally (existing behavior preserved)
- [ ] Delete plan with feature branch → branch deleted, plan branch marked Abandoned

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Backspace on plan group → `delete_ideation_session` command → cascade logic runs
- [ ] `task:deleted` events emitted → frontend query invalidation triggers → UI updates
- [ ] No `lib.rs` registration change needed (Tauri DI handles new params)

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
