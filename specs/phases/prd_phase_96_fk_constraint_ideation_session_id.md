# RalphX - Phase 96: Fix FK Constraint — Add ideation_session_id to Tasks

## Overview

`apply_proposals_to_kanban` fails with "FOREIGN KEY constraint failed" when the ideation session has no `plan_artifact_id`. The code falls back to using the session UUID as a fake artifact ID, then writes it to `tasks.plan_artifact_id` which has `REFERENCES artifacts(id)`. The session UUID doesn't exist in `artifacts`, causing the FK violation.

This phase adds an `ideation_session_id` column to the tasks table — a direct, always-valid link to the originating session. This eliminates the need to fake `plan_artifact_id` with session IDs and fixes the FK constraint error.

**Reference Plan:**
- `specs/plans/fix_fk_constraint_ideation_session_id.md` - Detailed implementation plan with compilation unit analysis and file-level code review notes

## Goals

1. Add `ideation_session_id` column to tasks with migration + backfill from existing proposals
2. Fix the apply command to stop faking `plan_artifact_id` with session IDs
3. Update graph query to group tasks by `ideation_session_id` when `plan_artifact_id` is NULL
4. Update plan branch commands to set `ideation_session_id` on merge tasks

## Dependencies

### Phase 95 (Fix Feature Branch Not Created on Accept Plan) - Required

| Dependency | Why Needed |
|------------|------------|
| `plan_artifact_id` fallback logic | Phase 95 introduced the session_id fallback that this phase replaces |
| `apply_proposals_to_kanban` | The command this phase fixes |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_fk_constraint_ideation_session_id.md`
2. Understand the compilation unit analysis (Tasks 1-3 are a single unit)
3. Review the "File Analysis Notes" section for hardcoded SELECT lists
4. Then proceed with the specific task

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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_fk_constraint_ideation_session_id.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add ideation_session_id column: migration v15, Task entity field, and all repository queries",
    "plan_section": "1. Migration v15 + 2. Task Entity + 3. Repository Queries",
    "blocking": [2, 3, 4],
    "blockedBy": [],
    "atomic_commit": "fix(tasks): add ideation_session_id column, entity field, and repo queries",
    "steps": [
      "Read specs/plans/fix_fk_constraint_ideation_session_id.md sections 1, 2, and 3",
      "Create v15_task_ideation_session_id.rs migration: add_column_if_not_exists + backfill from task_proposals",
      "Create v15_task_ideation_session_id_tests.rs with column existence, backfill, and idempotency tests",
      "Register v15 in migrations/mod.rs: add mod, #[cfg(test)] mod, MIGRATIONS entry, bump SCHEMA_VERSION to 15",
      "Add ideation_session_id: Option<IdeationSessionId> field to Task struct in task.rs (after plan_artifact_id)",
      "Add IdeationSessionId import to task.rs (use super::IdeationSessionId — already re-exported from entities/mod.rs)",
      "Update Task::new() to include ideation_session_id: None",
      "Update Task::from_row() to parse ideation_session_id from row",
      "Update setup_test_db() in task.rs tests to include ideation_session_id TEXT column",
      "Update queries.rs: append ', ideation_session_id' to TASK_COLUMNS and all 4 hardcoded SELECT constants",
      "Update mod.rs create() INSERT: add column name + ?20 param for ideation_session_id",
      "Update mod.rs update() SET: add ideation_session_id = ?18, renumber subsequent params",
      "Update ALL inline SELECTs in mod.rs: get_by_status, get_next_executable, get_blockers, get_dependents, archive, restore — append ideation_session_id to column lists",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(tasks): add ideation_session_id column, entity field, and repo queries"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Fix apply_proposals_to_kanban: set ideation_session_id on tasks, stop faking plan_artifact_id",
    "plan_section": "4. Apply Command Fix (ideation_commands_apply.rs)",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "fix(ideation): stop faking plan_artifact_id, use ideation_session_id",
    "steps": [
      "Read specs/plans/fix_fk_constraint_ideation_session_id.md section 4",
      "At ~line 86 after task creation: set task.ideation_session_id = Some(session_id.clone()) on each new task before repo.create()",
      "At lines 168-173: replace fake artifact fallback — change to: let plan_artifact_id: Option<ArtifactId> = session.plan_artifact_id.clone()",
      "Lines 176-197 (set plan_artifact_id loop): wrap in if let Some(ref artifact_id) = plan_artifact_id so it only runs when real artifact exists",
      "At ~line 265: set merge_task.ideation_session_id = Some(session_id.clone())",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(ideation): stop faking plan_artifact_id, use ideation_session_id"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add 3rd pass to graph query: group tasks by ideation_session_id when plan_artifact_id is NULL",
    "plan_section": "5. Graph Query (query.rs)",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "fix(graph): group tasks by ideation_session_id when plan_artifact_id is NULL",
    "steps": [
      "Read specs/plans/fix_fk_constraint_ideation_session_id.md section 5",
      "After the existing '5b. Second pass' block (lines 574-601), add section 5c",
      "Build grouped_task_ids HashSet from existing plan_groups",
      "Build session_group_index HashMap<String, usize> from plan_groups",
      "Iterate tasks: skip already-grouped, match by ideation_session_id to session_group_index",
      "Push matched task_ids into plan_groups and update status_summary",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(graph): group tasks by ideation_session_id when plan_artifact_id is NULL"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Set ideation_session_id on merge task and backfilled tasks in plan branch commands",
    "plan_section": "6. Plan Branch Commands (plan_branch_commands.rs)",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "fix(plan-branch): set ideation_session_id on merge task and backfilled tasks",
    "steps": [
      "Read specs/plans/fix_fk_constraint_ideation_session_id.md section 6",
      "In enable_feature_branch backfill loop (~line 182-191): also set task_to_update.ideation_session_id = Some(session_id.clone()) when backfilling plan_artifact_id",
      "At merge task creation (~line 207): set merge_task.ideation_session_id = Some(session_id.clone())",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(plan-branch): set ideation_session_id on merge task and backfilled tasks"
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
| **Add new column instead of fixing fallback** | The session_id-as-artifact-id hack violates FK constraints. A dedicated column is the correct fix. |
| **Tasks 1-3 as single compilation unit** | Entity field, repo queries, and migration must ship together — `from_row()` fails if column missing, repo fails if field missing |
| **Backfill via task_proposals join** | Existing tasks linked to sessions via proposals; backfill populates ideation_session_id from that relationship |
| **Keep plan_artifact_id as-is** | No schema change to existing column — only stop putting non-artifact IDs into it |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] v15 migration tests pass (column exists, backfill works, idempotent)
- [ ] Existing task repo tests pass with new column
- [ ] Task entity serialization/deserialization includes ideation_session_id

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### Manual Testing
- [ ] Apply proposals from session with `plan_artifact_id = NULL` — no FK error
- [ ] Apply proposals from session WITH `plan_artifact_id` — still works correctly
- [ ] Graph view groups tasks by session when no artifact exists
- [ ] Enable feature branch on plan without artifact — merge task gets ideation_session_id

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `ideation_session_id` column exists after migration
- [ ] Tasks created via `apply_proposals_to_kanban` have `ideation_session_id` set
- [ ] Tasks with `ideation_session_id` but no `plan_artifact_id` appear in correct graph group
- [ ] Merge tasks have `ideation_session_id` set

**Common failure modes to check:**
- [ ] No hardcoded SELECT in sqlite_task_repo/mod.rs missing ideation_session_id column
- [ ] No `from_row()` call where the query doesn't include ideation_session_id
- [ ] Backfill migration correctly joins task_proposals on created_task_id

See `.claude/rules/gap-verification.md` for full verification workflow.
