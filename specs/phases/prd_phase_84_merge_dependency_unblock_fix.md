# RalphX - Phase 84: Merge Dependency Unblock Fix

## Overview

When a task enters `merge_conflict` (a failure state), its dependent blocked tasks are incorrectly promoted to `ready` and picked up by the execution queue. This happens because `unblock_dependents()` is called at `Approved` (before merge), and `is_blocker_complete()` treats `Approved` as terminal. Since Phase 66 introduced the merge workflow, `Approved` auto-transitions to `PendingMerge` and is no longer terminal — unblocking should only happen at `Merged`.

**Reference Plan:**
- `specs/plans/fix_blocked_tasks_incorrectly_promoted_on_merge_failure.md` - Root cause analysis, execution flow diagrams, and fix details

## Goals

1. Remove premature `unblock_dependents()` call from `on_enter(Approved)` — defer to `on_enter(Merged)`
2. Remove `Approved` from `is_blocker_complete()` and `get_incomplete_blocker_names()` — only `Merged` means work is on main
3. Update existing test to assert the corrected behavior

## Dependencies

### Phase 66 (Per-Task Git Branch Isolation) - Required

| Dependency | Why Needed |
|------------|------------|
| Merge workflow (Approved → PendingMerge → Merging → Merged) | This bug was introduced when Approved became non-terminal in Phase 66 |
| `on_enter(Merged)` side effect already calls `unblock_dependents()` | Fix relies on this existing correct path |

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_blocked_tasks_incorrectly_promoted_on_merge_failure.md`
2. Understand the bug's root cause and execution flow
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Run linters for modified code only (backend: `cargo clippy`, `cargo test`)
4. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_blocked_tasks_incorrectly_promoted_on_merge_failure.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Fix dependency unblocking to require Merged (not Approved) and update tests",
    "plan_section": "Fix (Changes 1-3) + Tests (Test 4)",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(state_machine): defer dependency unblocking until Merged, not Approved",
    "steps": [
      "Read specs/plans/fix_blocked_tasks_incorrectly_promoted_on_merge_failure.md sections 'Fix' and 'Tests'",
      "In side_effects.rs:624-638, remove the unblock_dependents() call from on_enter(Approved) — keep task_completed event emission",
      "In task_transition_service.rs:125-132, update doc comment and remove InternalStatus::Approved from is_blocker_complete() match arm",
      "In task_transition_service.rs:149-152, remove InternalStatus::Approved from get_incomplete_blocker_names() match arm",
      "In tests.rs:~334-336, update existing test to assert unblock_dependents is NOT called when entering Approved",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(state_machine): defer dependency unblocking until Merged, not Approved"
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

**Compilation Unit Note:** All changes (side_effects.rs removal, task_transition_service.rs match arm updates, test update) form a single compilation unit. The test at line ~336 asserts `unblock_dependents` IS called at Approved — it will fail if the side_effects.rs change is applied without the test update.

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Remove unblock from Approved, keep at Merged** | `on_enter(Merged)` at side_effects.rs:687-695 already calls `unblock_dependents()` — this is the correct location post-Phase 66 |
| **Single task (not split)** | All changes form one compilation unit — test assertion depends on behavior change |
| **Keep task_completed event at Approved** | Event emission is correct — task work IS complete at Approved, just not merged yet |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] Existing test updated: `test_review_approved_auto_transitions_to_pending_merge` asserts `unblock_dependents` is NOT called
- [ ] All existing tests pass (no regressions from removing Approved from blocker-complete check)

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### Manual Testing
- [ ] Create two tasks with dependency (Task B blocked by Task A)
- [ ] Execute blocker task (Task A) through to Approved
- [ ] Verify dependent (Task B) stays `Blocked` through Approved → PendingMerge
- [ ] On successful merge → Merged: Task B promoted to Ready
- [ ] On failed merge → merge_conflict: Task B remains Blocked

### Wiring Verification

**Verify existing unblock paths remain correct:**

- [ ] `on_enter(Merged)` at side_effects.rs:687-695 still calls `unblock_dependents()`
- [ ] Programmatic merge success at side_effects.rs:849-855 still calls `unblock_dependents()`
- [ ] `attempt_merge_auto_complete` at chat_service_send_background.rs:956-966 still calls `unblock_dependents()`
- [ ] No other path calls `unblock_dependents()` for merge-related states

See `.claude/rules/gap-verification.md` for full verification workflow.
