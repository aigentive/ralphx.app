# RalphX - Phase 106: Fix Startup Recovery Ignores Archived Tasks

## Overview

On app startup, `StartupJobRunner` re-triggers entry actions for tasks stuck in agent-active states (`Executing`, `Reviewing`, `Merging`, etc.) and auto-transition states (`PendingMerge`, `Approved`, etc.). The underlying `get_by_status()` query has **no `archived_at IS NULL` filter**, so archived/soft-deleted test tasks get re-triggered every startup — spawning agents, attempting merges, and polluting logs.

This phase fixes the root cause in the repository query and adds defense-in-depth guards in the startup recovery loops.

**Reference Plan:**
- `specs/plans/fix_startup_recovery_ignores_archived_tasks.md` - Detailed two-layer fix with SQL changes, guard clauses, and test specifications

## Goals

1. Exclude archived tasks from `get_by_status()` in both SQLite and memory repository implementations
2. Add defense-in-depth skip guards in all three startup recovery loops
3. Add test coverage for archived task exclusion

## Dependencies

### Phase 105 (Fix Duplicate Agent Processes) - Not required

No direct dependency — this fix targets a different subsystem (query filtering vs process management).

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

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_startup_recovery_ignores_archived_tasks.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_startup_recovery_ignores_archived_tasks.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Exclude archived tasks from get_by_status query in both SQLite and memory repos, add tests",
    "plan_section": "Layer 1: Fix get_by_status query (root cause)",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "fix(task-repo): exclude archived tasks from get_by_status query",
    "steps": [
      "Read specs/plans/fix_startup_recovery_ignores_archived_tasks.md section 'Layer 1'",
      "Add `AND archived_at IS NULL` to SQL WHERE clause in sqlite_task_repo/mod.rs:173",
      "Add `&& t.archived_at.is_none()` filter in memory_task_repo/mod.rs:124",
      "Add test_get_by_status_excludes_archived in sqlite_task_repo/tests.rs",
      "Add test_get_by_status_excludes_archived in memory_task_repo/tests.rs",
      "Run cargo test -- get_by_status",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: fix(task-repo): exclude archived tasks from get_by_status query"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add defense-in-depth archived task skip guards in startup_jobs.rs recovery loops",
    "plan_section": "Layer 2: Defense-in-depth in startup_jobs.rs",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "fix(startup): skip archived tasks in startup recovery loops",
    "steps": [
      "Read specs/plans/fix_startup_recovery_ignores_archived_tasks.md section 'Layer 2'",
      "Add archived_at.is_some() skip + eprintln log in agent-active recovery loop (~line 247)",
      "Add archived_at.is_some() skip + eprintln log in auto-transition recovery loop (~line 306)",
      "Add archived_at.is_some() skip + eprintln log in blocker unblock loop (~line 394)",
      "Run cargo test",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: fix(startup): skip archived tasks in startup recovery loops"
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
| **Fix query rather than callers** | `get_by_status` is the single source — fixing it covers all callers (startup, reconciliation, blocker unblocking, queue counting, pause listing) |
| **Defense-in-depth guards** | Safety net in case `get_by_status` is ever changed or a different query path is used |
| **Tests bundled with Layer 1** | Tests validate the query fix directly — must be in same compilation unit to avoid writing tests against unfixed code |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] `test_get_by_status_excludes_archived` passes for SQLite repo
- [ ] `test_get_by_status_excludes_archived` passes for memory repo
- [ ] All existing `get_by_status` tests still pass

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### Manual Testing
- [ ] Restart app with archived tasks in PendingMerge — no `[STARTUP] Re-triggering` log for archived tasks
- [ ] Active (non-archived) tasks in agent-active states still resume correctly on startup

### Wiring Verification

- [ ] `get_by_status` SQL includes `AND archived_at IS NULL`
- [ ] Memory repo filter includes `archived_at.is_none()`
- [ ] All three startup loops (agent-active, auto-transition, blocker unblock) have archived skip guard

See `.claude/rules/gap-verification.md` for full verification workflow.
