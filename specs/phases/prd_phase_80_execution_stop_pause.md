# RalphX - Phase 80: Execution Stop/Pause Semantics

## Overview

Implement definitive stop/pause semantics so stop is a permanent kill requiring manual restart, while pause allows auto-recovery on resume with clear UI messaging.

Currently, both Stop and Pause transition agent-active tasks to `Failed` status. This phase introduces two new statuses (`Stopped` and `Paused`) with distinct semantics:
- **Stopped**: Terminal state requiring manual restart. Used by "Stop All" action.
- **Paused**: Non-terminal state with auto-recovery on resume. Used by "Pause" action.

**Reference Plan:**
- `specs/plans/execution_stop_pause_hard_stop_plan.md` - Detailed implementation plan with task breakdown and dependency graph

## Goals

1. Add `Paused` and `Stopped` as explicit `InternalStatus` variants with correct terminal/active semantics
2. Update stop_execution to transition tasks to `Stopped` (permanent, manual restart required)
3. Update pause_execution to transition tasks to `Paused` (recoverable on resume)
4. Update resume_execution to restore only `Paused` tasks using status history
5. Prevent auto-recovery/unblock when blockers are `Paused` or `Stopped`
6. Update frontend UI with new status display and clear messaging

## Dependencies

### Phase 78 (Git Merge Verification Fix) - Required

| Dependency | Why Needed |
|------------|------------|
| Stable execution control | Stop/pause builds on existing ExecutionState infrastructure |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/execution_stop_pause_hard_stop_plan.md`
2. Understand the task dependency graph and parallel execution strategy
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write/update tests for the changes
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
- Task 1 must complete first (BLOCKING)
- Tasks 2, 3, 5, 6 can run in parallel after Task 1
- Task 4 requires both Task 2 and Task 3 to complete

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false` where all `blockedBy` tasks have `"passes": true`
2. **Read the ENTIRE implementation plan** at `specs/plans/execution_stop_pause_hard_stop_plan.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add Paused and Stopped status variants to InternalStatus (BE + FE types)",
    "plan_section": "Task 1: Add Paused/Stopped statuses to InternalStatus",
    "blocking": [2, 3, 5, 6],
    "blockedBy": [],
    "atomic_commit": "feat(status): add Paused and Stopped status variants",
    "steps": [
      "Read specs/plans/execution_stop_pause_hard_stop_plan.md section 'Task 1'",
      "Add Paused and Stopped variants to InternalStatus enum in src-tauri/src/domain/entities/status.rs",
      "Update is_terminal() - Stopped is terminal, Paused is NOT",
      "Update is_agent_active() - neither is agent-active",
      "Update allowed_transitions() for new statuses",
      "Add tests for new variants",
      "Update frontend src/types/status.ts - add to InternalStatus type and Zod schema",
      "Update TERMINAL_STATUSES to include 'stopped'",
      "Update ACTIVE_STATUSES - neither included",
      "Run cargo clippy && cargo test",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(status): add Paused and Stopped status variants"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Update stop_execution to transition to Stopped status",
    "plan_section": "Task 2: Update stop_execution to transition to Stopped status",
    "blocking": [4],
    "blockedBy": [1],
    "atomic_commit": "feat(execution): transition to Stopped status on stop_execution",
    "steps": [
      "Read specs/plans/execution_stop_pause_hard_stop_plan.md section 'Task 2'",
      "Modify stop_execution in src-tauri/src/commands/execution_commands.rs",
      "Kill running agent processes immediately (registry stop)",
      "Transition all agent-active tasks to Stopped (not Failed)",
      "Keep execution paused",
      "Emit status updates after count reaches 0",
      "Add tests for stop behavior",
      "Run cargo clippy && cargo test",
      "Commit: feat(execution): transition to Stopped status on stop_execution"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Update pause_execution to transition to Paused status",
    "plan_section": "Task 3: Update pause_execution to transition to Paused status",
    "blocking": [4],
    "blockedBy": [1],
    "atomic_commit": "feat(execution): transition to Paused status on pause_execution",
    "steps": [
      "Read specs/plans/execution_stop_pause_hard_stop_plan.md section 'Task 3'",
      "Modify pause_execution in src-tauri/src/commands/execution_commands.rs",
      "Stop running agents",
      "Transition agent-active tasks to Paused",
      "Set execution paused",
      "Add tests for pause behavior",
      "Run cargo clippy && cargo test",
      "Commit: feat(execution): transition to Paused status on pause_execution"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Update resume_execution to restore only Paused tasks",
    "plan_section": "Task 4: Update resume_execution to restore only Paused tasks",
    "blocking": [],
    "blockedBy": [2, 3],
    "atomic_commit": "feat(execution): restore Paused tasks on resume, not Stopped",
    "steps": [
      "Read specs/plans/execution_stop_pause_hard_stop_plan.md section 'Task 4'",
      "Modify resume_execution in src-tauri/src/commands/execution_commands.rs",
      "Clear pause state",
      "Restore only Paused tasks to their last pre-pause status using status history",
      "Re-run entry actions for restored status via TaskTransitionService::execute_entry_actions()",
      "Do NOT restore Stopped tasks automatically",
      "Add tests for resume behavior with both Paused and Stopped tasks",
      "Run cargo clippy && cargo test",
      "Commit: feat(execution): restore Paused tasks on resume, not Stopped"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Prevent auto-recovery/unblock for Paused/Stopped blockers",
    "plan_section": "Task 5: Prevent auto-recovery/unblock for Paused/Stopped blockers",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "fix(recovery): prevent auto-unblock for Paused/Stopped blockers",
    "steps": [
      "Read specs/plans/execution_stop_pause_hard_stop_plan.md section 'Task 5'",
      "Update StartupJobRunner::run() in src-tauri/src/application/startup_jobs.rs - ignore paused/stopped",
      "Update ReconciliationRunner::recover_execution_stop() - no-op for paused/stopped tasks",
      "Update RepoBackedDependencyManager::is_blocker_complete() - Paused/Stopped are NOT complete",
      "Update StartupJobRunner::all_blockers_complete() - exclude Paused/Stopped",
      "Add tests for blocker behavior",
      "Run cargo clippy && cargo test",
      "Commit: fix(recovery): prevent auto-unblock for Paused/Stopped blockers"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Update frontend UI for Paused/Stopped statuses",
    "plan_section": "Task 6: Update frontend UI for Paused/Stopped statuses",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(ui): add Paused/Stopped status display and execution bar messaging",
    "steps": [
      "Read specs/plans/execution_stop_pause_hard_stop_plan.md section 'Task 6'",
      "Update src/components/tasks/TaskDetailView.tsx - status badges",
      "Update src/components/tasks/TaskDetailPanel.tsx - status display",
      "Update src/components/TaskGraph/nodes/nodeStyles.ts - node colors",
      "Update src/types/status-icons.ts - status icons",
      "Update src/types/workflow.ts - workflow mappings",
      "Update execution bar tooltips/confirmation copy to distinguish Stop vs Pause",
      "Ensure filters/counts include stopped in terminal but NOT paused",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ui): add Paused/Stopped status display and execution bar messaging"
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
| **Stopped is terminal, Paused is not** | Stopped = permanent kill (like Failed/Cancelled), Paused = temporary suspension (can resume) |
| **Resume only restores Paused tasks** | Clear semantic distinction - Stop All means "I want to manually restart these", Pause means "I want to continue later" |
| **Both BE + FE types in Task 1** | Adding enum variants is additive (doesn't break existing code) and cross-layer type parity is required |
| **Use status history for resume** | Find the last transition where `to == Paused`, restore to `from` status, re-run entry actions |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] InternalStatus includes Paused and Stopped variants
- [ ] is_terminal() returns true for Stopped, false for Paused
- [ ] is_agent_active() returns false for both
- [ ] stop_execution transitions to Stopped
- [ ] pause_execution transitions to Paused
- [ ] resume_execution restores only Paused tasks
- [ ] Startup recovery ignores Paused/Stopped tasks
- [ ] Blocker completion excludes Paused/Stopped

### Frontend - Run `npm run test`
- [ ] InternalStatus type includes 'paused' and 'stopped'
- [ ] TERMINAL_STATUSES includes 'stopped'
- [ ] Status icons and labels exist for new statuses

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Click Stop All → running tasks show "Stopped" status
- [ ] Click Resume → Stopped tasks remain Stopped
- [ ] Click Pause → running tasks show "Paused" status
- [ ] Click Resume → Paused tasks resume to previous status
- [ ] Blocked tasks don't auto-unblock when blocker is Paused/Stopped
- [ ] App restart → Paused/Stopped tasks don't auto-recover

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (click handler, route, event listener)
- [ ] New component is imported AND rendered (not behind disabled flag)
- [ ] API wrappers call backend commands
- [ ] State changes reflect in UI

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
