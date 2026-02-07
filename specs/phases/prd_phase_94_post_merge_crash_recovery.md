# RalphX - Phase 94: Post-Merge Crash Recovery

## Overview

Fix bugs in the startup crash recovery system that incorrectly unblocks dependent tasks when a blocker is in `Approved` state (not yet merged), and add recovery for tasks stuck in `PendingMerge` after a crash.

The existing `unblock_ready_tasks()` startup job correctly handles the `Merged` crash scenario, but `all_blockers_complete()` inconsistently treats `Approved` as terminal (the runtime `is_blocker_complete()` correctly excludes it). Additionally, tasks stuck in `PendingMerge` are not recovered because `PendingMerge` is missing from `AUTO_TRANSITION_STATES`.

**Reference Plan:**
- `specs/plans/post_merge_crash_recovery.md` - Detailed plan with compilation unit analysis and file references

## Goals

1. Fix `Approved` inconsistency between startup and runtime blocker completion checks
2. Add `PendingMerge` crash recovery via `AUTO_TRANSITION_STATES`
3. Fix incorrect test that asserts wrong behavior for `Approved` blocker state

## Dependencies

### Phase 93 (Merge Target MCP Tool & Branch Fix) - Required

| Dependency | Why Needed |
|------------|------------|
| Startup job infrastructure | `StartupJobRunner`, `unblock_ready_tasks()`, `AUTO_TRANSITION_STATES` all exist |
| Merge workflow states | `PendingMerge`, `Approved`, `Merged` states and transitions are defined |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/post_merge_crash_recovery.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/post_merge_crash_recovery.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Fix Approved inconsistency in all_blockers_complete() and update test to assert correct behavior",
    "plan_section": "Task 1: Fix Approved inconsistency in all_blockers_complete() + test",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(startup): remove Approved from blocker terminal states",
    "steps": [
      "Read specs/plans/post_merge_crash_recovery.md section 'Task 1'",
      "In startup_jobs.rs:456-459, update doc comment to list only Merged, Failed, Cancelled (remove Approved)",
      "In startup_jobs.rs:468-474, remove InternalStatus::Approved from the match arm in all_blockers_complete()",
      "In startup_jobs/tests.rs:649-686, rename test to test_blocked_task_remains_blocked_when_blocker_is_approved",
      "Add execution_state.pause() after setup_test_state() to isolate unblock logic from auto-transition recovery",
      "Change assertion: task should remain InternalStatus::Blocked (not Ready)",
      "Run cargo test -p ralphx-lib -- startup_jobs",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: fix(startup): remove Approved from blocker terminal states"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add PendingMerge to AUTO_TRANSITION_STATES and add recovery test",
    "plan_section": "Task 2: Add PendingMerge to AUTO_TRANSITION_STATES + test",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(startup): add PendingMerge crash recovery",
    "steps": [
      "Read specs/plans/post_merge_crash_recovery.md section 'Task 2'",
      "In execution_commands.rs:41-46, add InternalStatus::PendingMerge to AUTO_TRANSITION_STATES with comment: // attempt_programmatic_merge() (→ Merged or → Merging)",
      "In startup_jobs/tests.rs, add test_pending_merge_auto_transitions_on_startup following the pattern of test_approved_auto_transitions_on_startup (lines 471-512)",
      "Test should: create project, create task in PendingMerge state, set high max_concurrent, set active project, run startup, verify task transitions out of PendingMerge",
      "Run cargo test -p ralphx-lib -- startup_jobs",
      "Run cargo test -p ralphx-lib -- execution_commands",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: fix(startup): add PendingMerge crash recovery"
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
| **Merge Tasks 1+2 from original plan into one compilation unit** | Removing `Approved` from the match arm breaks the existing test — both changes must land atomically |
| **Merge Tasks 3+4 from original plan into one compilation unit** | The `AUTO_TRANSITION_STATES` constant and its test validate each other — must land together |
| **Use `execution_state.pause()` in Approved test** | Isolates the unblock logic from auto-transition recovery which would otherwise transition Approved→PendingMerge, masking the test's intent |
| **`PendingMerge` recovery is safe** | `attempt_programmatic_merge()` is idempotent — safe to re-trigger on startup |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] `cargo test -p ralphx-lib -- startup_jobs` — all startup tests pass
- [ ] `cargo test -p ralphx-lib -- execution_commands` — constant assertions pass
- [ ] `test_blocked_task_remains_blocked_when_blocker_is_approved` — asserts task stays Blocked
- [ ] `test_pending_merge_auto_transitions_on_startup` — asserts task transitions out of PendingMerge

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### Wiring Verification

**For each change, verify the full path:**

- [ ] `all_blockers_complete()` no longer includes `Approved` — blocked tasks with Approved blockers stay blocked on startup
- [ ] `AUTO_TRANSITION_STATES` includes `PendingMerge` — tasks stuck in PendingMerge are recovered on startup
- [ ] No regression: tasks with `Merged`, `Failed`, `Cancelled` blockers still correctly unblock

**Common failure modes to check:**
- [ ] `Approved` blocker correctly triggers auto-transition recovery (Approved→PendingMerge) instead of unblocking dependents
- [ ] `PendingMerge` recovery respects `max_concurrent` execution limit

See `.claude/rules/gap-verification.md` for full verification workflow.
