# RalphX - Phase 100: Plan Merge Tasks Auto-Promote to Merge Workflow

## Overview

When all tasks in a plan complete (reach `Merged`), the auto-created plan merge task gets unblocked (`Blocked` → `Ready`) but then sits idle. It should automatically proceed to the merge workflow (`PendingMerge` → programmatic merge attempt).

Two root causes: (1) `unblock_dependents()` bypasses the state machine — does a direct DB update to `Ready` without triggering `try_schedule_ready_tasks()`, so newly-unblocked tasks are never picked up. (2) The scheduler always routes Ready tasks to `Executing`, but `plan_merge` tasks should skip execution and go directly to `PendingMerge`.

**Reference Plan:**
- `specs/plans/fix_plan_merge_tasks_auto_promote.md` - Detailed fix with compilation unit analysis and startup recovery verification

## Goals

1. Trigger task scheduling after `unblock_dependents()` in `on_enter(Merged)` so newly-Ready tasks are picked up
2. Allow `Ready → PendingMerge` transition in the state machine for plan_merge tasks
3. Route `plan_merge` category tasks to `PendingMerge` instead of `Executing` in the scheduler

## Dependencies

### Phase 85 (Feature Branch for Plan Groups) - Required

| Dependency | Why Needed |
|------------|------------|
| Plan merge task (category=`plan_merge`) | Created by feature branch system, this fix makes it auto-promote |
| `resolve_merge_branches()` | Already handles plan merge tasks correctly (feature branch → main) |
| `attempt_programmatic_merge()` | Already triggered on `PendingMerge` entry |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_plan_merge_tasks_auto_promote.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_plan_merge_tasks_auto_promote.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Fix plan merge tasks auto-promote: trigger scheduling after unblock, add Ready→PendingMerge transition, route plan_merge to PendingMerge in scheduler",
    "plan_section": "Changes (all 3 sections)",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(scheduler): auto-promote plan_merge tasks to PendingMerge on unblock",
    "steps": [
      "Read specs/plans/fix_plan_merge_tasks_auto_promote.md",
      "In side_effects.rs on_enter(Merged): add self.try_schedule_ready_tasks().await after unblock_dependents()",
      "In status.rs valid_transitions(): add PendingMerge to Ready's valid targets",
      "In task_scheduler_service.rs try_schedule_ready_tasks(): check task.category == 'plan_merge' and route to PendingMerge instead of Executing",
      "Update existing tests for valid_transitions to include PendingMerge from Ready",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(scheduler): auto-promote plan_merge tasks to PendingMerge on unblock"
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
| **Single task for all 3 changes** | All changes compile independently but are only meaningful together — single commit ensures atomic fix |
| **Route via scheduler, not auto-transition** | Plan merge tasks need scheduling awareness (Local mode single-task enforcement), auto-transition would bypass these guards |
| **No startup recovery changes** | Existing `try_schedule_ready_tasks()` at startup + fix #3 handles all crash recovery scenarios |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] Valid transitions test includes `PendingMerge` in Ready's targets
- [ ] Scheduler test for plan_merge routing (if added)

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### Manual Testing
- [ ] Create a plan with feature branch enabled
- [ ] Execute all plan tasks through to Merged
- [ ] Verify merge task auto-transitions: Blocked → Ready → PendingMerge → (Merged or Merging)
- [ ] Verify standalone tasks (non-plan_merge) still route to Executing normally

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `on_enter(Merged)` calls `try_schedule_ready_tasks()` after `unblock_dependents()`
- [ ] `Ready` valid transitions include `PendingMerge`
- [ ] Scheduler checks `task.category == "plan_merge"` and transitions to `PendingMerge`
- [ ] `PendingMerge` entry triggers `attempt_programmatic_merge()` (existing, unchanged)

**Common failure modes to check:**
- [ ] Non-plan_merge tasks still route to Executing (regression check)
- [ ] Local mode single-task enforcement still works for plan_merge tasks entering PendingMerge

See `.claude/rules/gap-verification.md` for full verification workflow.
