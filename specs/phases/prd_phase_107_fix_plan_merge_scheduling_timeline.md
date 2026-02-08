# RalphX - Phase 107: Fix Plan Merge Task Not Auto-Scheduled + Empty Timeline

## Overview

After all tasks in a plan/ideation session merge, the auto-created plan merge task transitions to `Ready` but is never picked up by the scheduler — it only works after app restart. Additionally, the plan merge task's timeline shows no events.

The root cause is that `unblock_dependents()` in `RepoBackedDependencyManager` performs a raw DB update from `Blocked` → `Ready`, bypassing the state machine entirely. This means `on_enter(Ready)` never fires (no scheduling) and `persist_status_change()` is never called (invisible to timeline). Furthermore, 2 of the 3 code paths that call `unblock_dependents()` after merge lack a subsequent `try_schedule_ready_tasks()` call.

**Reference Plan:**
- `specs/plans/fix_plan_merge_task_scheduling_and_timeline.md` - Detailed analysis of root cause with exact code locations and fixes

## Goals

1. Fix timeline visibility for all tasks unblocked via the dependency system
2. Fix auto-scheduling of unblocked tasks after programmatic merge (most common path)
3. Fix auto-scheduling of unblocked tasks after agent auto-complete merge detection

## Dependencies

### Phase 106 (Fix Startup Recovery Ignores Archived Tasks) - Required

| Dependency | Why Needed |
|------------|------------|
| Task state machine | All status transitions and side effects must be stable |
| Dependency manager | `unblock_dependents()` method exists and is functional |
| Task scheduler service | `try_schedule_ready_tasks()` method exists |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_plan_merge_task_scheduling_and_timeline.md`
2. Understand the three code paths that call `unblock_dependents()` after merge
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
- All 3 tasks are independent (no inter-task dependencies) — can be executed in any order
- Recommended: commit all together as a single atomic fix since they address the same root cause

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_plan_merge_task_scheduling_and_timeline.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add persist_status_change() in unblock_dependents() to record Blocked→Ready transition in timeline",
    "plan_section": "1. Add persist_status_change() in unblock_dependents() (fixes timeline)",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(task-transition): record timeline entry when unblocking dependents",
    "steps": [
      "Read specs/plans/fix_plan_merge_task_scheduling_and_timeline.md section '1. Add persist_status_change()'",
      "Open src-tauri/src/application/task_transition_service.rs",
      "After the successful task_repo.update() at line ~211 in unblock_dependents(), add persist_status_change() call with Blocked→Ready and trigger 'blockers_resolved'",
      "Use non-fatal tracing::warn for errors (timeline recording should not block unblocking)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(task-transition): record timeline entry when unblocking dependents"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add try_schedule_ready_tasks() in post_merge_cleanup() for programmatic merge path",
    "plan_section": "2. Add try_schedule_ready_tasks() in post_merge_cleanup() (fixes scheduling - programmatic merge path)",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(merge): schedule unblocked tasks after programmatic merge",
    "steps": [
      "Read specs/plans/fix_plan_merge_task_scheduling_and_timeline.md section '2. Add try_schedule_ready_tasks()'",
      "Open src-tauri/src/domain/state_machine/transition_handler/side_effects.rs",
      "After the unblock_dependents() call at line ~1296 in post_merge_cleanup(), add scheduler trigger using same pattern as on_enter(Merged) at lines 792-798",
      "Use self.machine.context.services.task_scheduler with Arc::clone and tokio::spawn with 600ms delay",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(merge): schedule unblocked tasks after programmatic merge"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add try_schedule_ready_tasks() in attempt_merge_auto_complete() for agent auto-complete merge path",
    "plan_section": "3. Add try_schedule_ready_tasks() in attempt_merge_auto_complete() (fixes scheduling - agent auto-complete path)",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(merge): schedule unblocked tasks after agent auto-complete merge",
    "steps": [
      "Read specs/plans/fix_plan_merge_task_scheduling_and_timeline.md section '3. Add try_schedule_ready_tasks()'",
      "Open src-tauri/src/application/chat_service/chat_service_send_background.rs",
      "After the unblock_dependents() call at line ~980, construct a TaskSchedulerService using all available params from function signature (lines 633-646)",
      "Add .with_plan_branch_repo() if plan_branch_repo is Some",
      "Wrap in Arc, spawn async task with 600ms delay calling try_schedule_ready_tasks()",
      "Verify import exists: use crate::application::task_scheduler_service::TaskSchedulerService (already present at line 13)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(merge): schedule unblocked tasks after agent auto-complete merge"
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
| **Non-fatal timeline recording** | `persist_status_change()` failure should not prevent unblocking — use `tracing::warn` |
| **600ms delay before scheduling** | Matches existing pattern in `on_enter(Merged)` — allows DB writes to settle |
| **Construct TaskSchedulerService in auto-complete path** | No scheduler available in function scope unlike side_effects.rs — must construct from available params |
| **Single root cause, 3 fix sites** | All 3 changes address the same bug (missing scheduling + timeline after unblock) in different code paths |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] All existing tests pass (no regressions)
- [ ] `persist_status_change` called with correct params in `unblock_dependents`

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### Manual Testing
- [ ] Create ideation session with 2+ tasks in a plan with feature branches enabled
- [ ] Execute all tasks → let them merge
- [ ] Verify the plan merge task is automatically picked up and merges the feature branch into main without restart
- [ ] After plan merge task completes, open detail view → verify timeline shows `Blocked → Ready → PendingMerge → Merged` transitions

### Wiring Verification

**For each fix, verify the full path from merge completion to task scheduling:**

- [ ] `unblock_dependents()` → `persist_status_change()` → timeline entry visible
- [ ] `post_merge_cleanup()` → `unblock_dependents()` → `try_schedule_ready_tasks()` → task scheduled
- [ ] `attempt_merge_auto_complete()` → `unblock_dependents()` → `try_schedule_ready_tasks()` → task scheduled
- [ ] `on_enter(Merged)` path still works (existing, unchanged)

See `.claude/rules/gap-verification.md` for full verification workflow.
