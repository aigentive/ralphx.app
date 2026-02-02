# RalphX - Phase 68: Task Crash Recovery Gaps Fix

## Overview

This phase fixes two gaps in the crash recovery system where tasks can get stuck in intermediate states after an app crash:

1. **Gap 1: Auto-Transition States** - Tasks that crash *after* entering a state with auto-transition but *before* the transition completes (e.g., `PendingReview` → `Reviewing`). These states need their entry actions re-triggered on startup.

2. **Gap 2: Merging State** - The `Merging` state spawns a merger agent but was missing from `AGENT_ACTIVE_STATUSES`, so tasks stuck in this state weren't being recovered.

**Reference Plan:**
- `specs/plans/fix_task_crash_recovery_gaps.md` - Detailed analysis of gaps and implementation approach

## Goals

1. Add `Merging` to `AGENT_ACTIVE_STATUSES` so merger agents are respawned on crash recovery
2. Create `AUTO_TRANSITION_STATES` constant for states with automatic transitions
3. Update `StartupJobRunner` to re-trigger auto-transition states
4. Add tests for the new recovery scenarios

## Dependencies

### Phase 66 (Per-Task Git Branch Isolation) - Required

| Dependency | Why Needed |
|------------|------------|
| Merging state and merger agent | This phase adds `Merging` to crash recovery; Phase 66 introduced the state |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_task_crash_recovery_gaps.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_task_crash_recovery_gaps.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add Merging to AGENT_ACTIVE_STATUSES for crash recovery",
    "plan_section": "Task 1: Add Merging to AGENT_ACTIVE_STATUSES",
    "blocking": [4],
    "blockedBy": [],
    "atomic_commit": "fix(startup): add Merging to AGENT_ACTIVE_STATUSES for crash recovery",
    "steps": [
      "Read specs/plans/fix_task_crash_recovery_gaps.md section 'Task 1'",
      "Open src-tauri/src/commands/execution_commands.rs",
      "Find the AGENT_ACTIVE_STATUSES constant",
      "Add InternalStatus::Merging to the array",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: fix(startup): add Merging to AGENT_ACTIVE_STATUSES for crash recovery"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add AUTO_TRANSITION_STATES constant for recovery",
    "plan_section": "Task 2: Add AUTO_TRANSITION_STATES constant",
    "blocking": [3, 4],
    "blockedBy": [],
    "atomic_commit": "feat(startup): add AUTO_TRANSITION_STATES constant for recovery",
    "steps": [
      "Read specs/plans/fix_task_crash_recovery_gaps.md section 'Task 2'",
      "Open src-tauri/src/commands/execution_commands.rs",
      "Add the AUTO_TRANSITION_STATES constant with doc comment after AGENT_ACTIVE_STATUSES",
      "Include: QaPassed, PendingReview, RevisionNeeded, Approved",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: feat(startup): add AUTO_TRANSITION_STATES constant for recovery"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Update StartupJobRunner to recover tasks stuck in auto-transition states",
    "plan_section": "Task 3: Update StartupJobRunner to handle auto-transition states",
    "blocking": [4],
    "blockedBy": [2],
    "atomic_commit": "feat(startup): recover tasks stuck in auto-transition states",
    "steps": [
      "Read specs/plans/fix_task_crash_recovery_gaps.md section 'Task 3'",
      "Open src-tauri/src/application/startup_jobs.rs",
      "Add use statement for AUTO_TRANSITION_STATES",
      "After the AGENT_ACTIVE_STATUSES loop, add the auto-transition recovery loop",
      "Loop through AUTO_TRANSITION_STATES, get tasks by status",
      "Check max_concurrent before triggering",
      "Call execute_entry_actions for each stuck task",
      "Add eprintln for startup visibility",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Run cargo test",
      "Commit: feat(startup): recover tasks stuck in auto-transition states"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Add tests for crash recovery scenarios",
    "plan_section": "Task 4: Add tests",
    "blocking": [],
    "blockedBy": [1, 2, 3],
    "atomic_commit": "test(startup): add crash recovery tests for auto-transition states",
    "steps": [
      "Read specs/plans/fix_task_crash_recovery_gaps.md section 'Task 4'",
      "Open src-tauri/src/application/startup_jobs.rs",
      "Add tests in the tests module for:",
      "  - Tasks in Merging get resumed (agent respawned)",
      "  - Tasks in PendingReview auto-transition to Reviewing",
      "  - Tasks in RevisionNeeded auto-transition to ReExecuting",
      "  - Tasks in Approved auto-transition to PendingMerge",
      "Run cargo test",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: test(startup): add crash recovery tests for auto-transition states"
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
| **Separate constants for agent-active vs auto-transition** | Agent-active states need agent respawn; auto-transition states need entry action re-trigger. Different recovery mechanisms. |
| **Re-execute entry actions (not direct transition)** | Entry actions contain the auto-transition logic. Re-executing ensures all side effects run correctly. |
| **Check max_concurrent for auto-transitions** | Some auto-transitions spawn agents; must respect concurrency limits. |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] All existing tests pass
- [ ] New crash recovery tests pass

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Build succeeds (`cargo build --release`)

### Manual Testing

#### Test 1: Merging State Recovery
1. Get a task to `Merging` state (e.g., via merge conflict)
2. Kill the app
3. Restart app
4. Verify terminal: `[STARTUP] Found 1 tasks in Merging status`
5. Verify merger agent respawns

#### Test 2: PendingReview Recovery
1. Set a task to `PendingReview` directly in DB:
   ```bash
   sqlite3 src-tauri/ralphx.db "UPDATE tasks SET internal_status='pending_review' WHERE id='<task-id>';"
   ```
2. Restart app
3. Verify terminal: `[STARTUP] Re-triggering auto-transition for task: <id>`
4. Verify task moves to `Reviewing` and reviewer agent spawns

#### Test 3: Approved Recovery (Merge Workflow)
1. Set a task to `Approved` directly in DB
2. Restart app
3. Verify auto-transition to `PendingMerge` triggers
4. Verify programmatic merge attempt runs

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `Merging` is in `AGENT_ACTIVE_STATUSES` and gets picked up by existing recovery loop
- [ ] `AUTO_TRANSITION_STATES` is imported and used in `StartupJobRunner`
- [ ] `execute_entry_actions` is called for auto-transition recovery
- [ ] Concurrency limits are respected

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
