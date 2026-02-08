# RalphX - Phase 108: Make MergeIncomplete View Useful

## Overview

The `MergeIncompleteTaskDetail` view currently shows generic hardcoded error explanations and "Branch: task branch" literally when a merge fails with a non-conflict git error. The actual error message, source/target branch names, and diagnostic info are only logged via `tracing` but never persisted to the task — leaving the user with zero diagnostic information.

This phase adds a `metadata` column to the tasks table, persists error context (error message, source/target branches) when merge operations fail, and updates the frontend view to display actual error details instead of generic placeholders.

**Reference Plan:**
- `specs/plans/make_merge_incomplete_view_useful.md` - Detailed implementation plan with compilation unit analysis

## Goals

1. Add `metadata` column to tasks table for structured error context
2. Persist actual git error messages and branch names on merge failure
3. Display real error details in the MergeIncomplete detail view
4. Maintain backwards compatibility when metadata is null

## Dependencies

### Phase 66 (Per-Task Git Branch Isolation) - Required

| Dependency | Why Needed |
|------------|------------|
| `task_branch` field on Task entity | Merge workflow uses task branches |
| `attempt_programmatic_merge()` in side_effects.rs | Error paths being enhanced |
| `MergeIncompleteTaskDetail` component | View being updated |

### Phase 83 (Merge Flow Fix) - Required

| Dependency | Why Needed |
|------------|------------|
| `MergeIncomplete` status | Status this phase targets |
| `report_incomplete` HTTP handler | Agent error path being enhanced |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/make_merge_incomplete_view_useful.md`
2. Understand the compilation unit analysis (why Task 1 is monolithic)
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

**Task Execution Order:**
- Task 1 must complete first (all others depend on it)
- Tasks 2, 3, 4 are independent and can be done in any order after Task 1
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/make_merge_incomplete_view_useful.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add metadata column to tasks — migration, entity, queries, repo, and TaskResponse",
    "plan_section": "Task 1: Backend metadata plumbing",
    "blocking": [2, 3, 4],
    "blockedBy": [],
    "atomic_commit": "feat(task): add metadata column to tasks table",
    "steps": [
      "Read specs/plans/make_merge_incomplete_view_useful.md section 'Task 1'",
      "Create src-tauri/src/infrastructure/sqlite/migrations/v18_task_metadata.rs with add_column_if_not_exists(conn, 'tasks', 'metadata', 'TEXT DEFAULT NULL')",
      "Create src-tauri/src/infrastructure/sqlite/migrations/v18_task_metadata_tests.rs with column exists, set/get, idempotent tests",
      "Register v18 in migrations/mod.rs, bump SCHEMA_VERSION to 18",
      "Add pub metadata: Option<String> to Task struct (after merge_commit_sha), set None in new(), add row.get('metadata')? in from_row(), add metadata column to setup_test_db()",
      "Append ', metadata' to TASK_COLUMNS and all 5 query constants in queries.rs",
      "Add metadata to INSERT (col list + ?21 param) and UPDATE (metadata = ?19 + param) in sqlite_task_repo/mod.rs",
      "Add metadata to ALL ~10 inline SELECT column lists in mod.rs (get_by_status, get_next_executable, get_blockers, get_dependents, archive x2, restore x2)",
      "Add pub metadata: Option<String> to TaskResponse and metadata: task.metadata in From<Task> impl in types.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(task): add metadata column to tasks table"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Persist error context (error message, source/target branches) when programmatic merge fails",
    "plan_section": "Task 2: Persist error context on programmatic merge failure",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(merge): persist error context to task metadata on merge failure",
    "steps": [
      "Read specs/plans/make_merge_incomplete_view_useful.md section 'Task 2'",
      "In side_effects.rs worktree mode error path (~line 1079-1118): before setting task.internal_status = InternalStatus::MergeIncomplete, set task.metadata = Some(serde_json::json!({ 'error': e.to_string(), 'source_branch': source_branch, 'target_branch': target_branch }).to_string())",
      "In side_effects.rs local mode error path (~line 1202-1236): same pattern — set task.metadata with error, source_branch, target_branch before status change",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(merge): persist error context to task metadata on merge failure"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Persist agent error context from report_incomplete HTTP handler",
    "plan_section": "Task 3: Persist error context from agent report_incomplete",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(merge): persist agent error context to task metadata",
    "steps": [
      "Read specs/plans/make_merge_incomplete_view_useful.md section 'Task 3'",
      "In src-tauri/src/http_server/handlers/git.rs report_incomplete handler (~line 352-426): after getting task and before transitioning, make task mutable and set task.metadata = Some(serde_json::json!({ 'error': req.reason, 'diagnostic_info': req.diagnostic_info }).to_string()), then task_repo.update(&task).await",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(merge): persist agent error context to task metadata"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Display actual error details in MergeIncomplete view instead of generic placeholders",
    "plan_section": "Task 4: Frontend — Display actual error details",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(merge-ui): show actual error details in MergeIncomplete view",
    "steps": [
      "Read specs/plans/make_merge_incomplete_view_useful.md section 'Task 4'",
      "In MergeIncompleteTaskDetail.tsx: parse task.metadata JSON to extract error, source_branch, target_branch, diagnostic_info",
      "Replace generic ErrorContextCard with conditional display: if metadata present show actual error in red code block, source→target branch labels, and diagnostic info; if null fall back to current generic text",
      "Update RecoverySteps to use branch name from metadata.source_branch or task.taskBranch instead of hardcoded 'task branch'",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(merge-ui): show actual error details in MergeIncomplete view"
    ],
    "passes": true
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
| **Generic `metadata` column (not merge-specific fields)** | Reusable for future error context in other states without new migrations |
| **JSON string in TEXT column** | Flexible schema, no need for additional tables; matches existing pattern (state history metadata) |
| **Task 1 as single compilation unit** | `Task::from_row()` reads `row.get("metadata")` — all SQL queries must SELECT it simultaneously or runtime crash |
| **Backwards-compatible null fallback** | Existing tasks have null metadata — frontend shows generic text when metadata absent |
| **Frontend already has metadata in types** | `TaskSchema`, `Task` interface, and `transformTask` already include metadata — no schema changes needed |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] v18 migration adds `metadata` column to tasks
- [ ] v18 migration is idempotent
- [ ] Task with metadata can be inserted and queried
- [ ] Task without metadata defaults to NULL

### Frontend - Run `npm run test`
- [ ] MergeIncompleteTaskDetail renders with null metadata (generic fallback)
- [ ] MergeIncompleteTaskDetail renders with metadata (actual error details)

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes

### Manual Testing
- [ ] Trigger a merge failure → MergeIncomplete view shows actual git error message
- [ ] Verify source and target branch names display correctly
- [ ] Verify retry button still works from MergeIncomplete state
- [ ] Verify mark resolved button still works
- [ ] Verify old tasks without metadata show generic fallback text

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Backend: metadata column exists in DB (migration ran)
- [ ] Backend: Task entity has metadata field
- [ ] Backend: TaskResponse includes metadata in serialization
- [ ] Backend: side_effects.rs sets metadata on merge error
- [ ] Backend: report_incomplete handler sets metadata on agent error
- [ ] Frontend: MergeIncompleteTaskDetail parses and displays metadata

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] JSON.parse handles malformed metadata gracefully (try/catch)

See `.claude/rules/gap-verification.md` for full verification workflow.
