# RalphX - Phase 54: Blocked Reason Feature

## Overview

This phase adds the ability to record and display why a task is blocked. Currently, blocked tasks disappear from the Kanban board (no column maps to the `blocked` status), and there's no way to capture the reason for blocking. This phase solves both issues by adding a "Blocked" group to the Ready column and a `blocked_reason` field to tasks.

**Reference Plan:**
- `specs/plans/blocked_reason_feature.md` - Detailed implementation plan with dependency graph and code snippets

## Goals

1. Display blocked tasks in the Ready column under a "Blocked" group
2. Add `blocked_reason` field to persist why a task is blocked
3. Show a dialog when blocking to capture the optional reason
4. Display the blocked reason on task cards (truncated with tooltip)

## Dependencies

### Phase 53 (Review Timeline Unification) - Required

| Dependency | Why Needed |
|------------|------------|
| Task state machine | Block/unblock transitions use existing TransitionHandler |
| TaskCard component | Will be extended to display blocked reason |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/blocked_reason_feature.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/blocked_reason_feature.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add blocked_reason column migration",
    "plan_section": "Task 1: Database Migration",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(backend): add blocked_reason column migration",
    "steps": [
      "Read specs/plans/blocked_reason_feature.md section 'Task 1: Database Migration'",
      "Create src-tauri/src/infrastructure/sqlite/migrations/v4_add_blocked_reason.rs",
      "Add mod v4_add_blocked_reason to mod.rs",
      "Add migration to MIGRATIONS array",
      "Bump SCHEMA_VERSION to 4",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(backend): add blocked_reason column migration"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add blocked_reason to task entity and repository",
    "plan_section": "Task 2: Backend Entity + Repository",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(backend): add blocked_reason to task entity and repository",
    "steps": [
      "Read specs/plans/blocked_reason_feature.md section 'Task 2: Backend Entity + Repository'",
      "Add blocked_reason: Option<String> to Task entity in task.rs",
      "Update SELECT queries in sqlite_task_repo.rs to include blocked_reason",
      "Update INSERT/UPDATE queries to handle blocked_reason",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(backend): add blocked_reason to task entity and repository"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add block_task and unblock_task commands",
    "plan_section": "Task 3: New Commands (block_task, unblock_task)",
    "blocking": [4],
    "blockedBy": [2],
    "atomic_commit": "feat(backend): add block_task and unblock_task commands",
    "steps": [
      "Read specs/plans/blocked_reason_feature.md section 'Task 3: New Commands'",
      "Add block_task command in mutation.rs (transitions to Blocked + sets reason)",
      "Add unblock_task command in mutation.rs (transitions to Ready + clears reason)",
      "Add blocked_reason to TaskResponse in types.rs",
      "Register new commands in lib.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(backend): add block_task and unblock_task commands"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Add blockedReason to task types",
    "plan_section": "Task 4: Frontend Types",
    "blocking": [5, 9],
    "blockedBy": [3],
    "atomic_commit": "feat(frontend): add blockedReason to task types",
    "steps": [
      "Read specs/plans/blocked_reason_feature.md section 'Task 4: Frontend Types'",
      "Add blocked_reason: z.string().nullable() to task schema (snake_case)",
      "Add blockedReason transform in the type transformation",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(frontend): add blockedReason to task types"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Add blockTask and unblockTask API functions",
    "plan_section": "Task 5: API Wrappers",
    "blocking": [8, 10],
    "blockedBy": [4],
    "atomic_commit": "feat(frontend): add blockTask and unblockTask API functions",
    "steps": [
      "Read specs/plans/blocked_reason_feature.md section 'Task 5: API Wrappers'",
      "Add blockTask(taskId, reason?) function in src/api/tasks.ts",
      "Add unblockTask(taskId) function in src/api/tasks.ts",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(frontend): add blockTask and unblockTask API functions"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Add blocked group to Ready column workflow",
    "plan_section": "Task 6: Add Blocked Group to Workflow",
    "blocking": [9],
    "blockedBy": [],
    "atomic_commit": "feat(frontend): add blocked group to Ready column workflow",
    "steps": [
      "Read specs/plans/blocked_reason_feature.md section 'Task 6: Add Blocked Group to Workflow'",
      "Add blocked group to defaultWorkflow.columns[1].groups in workflow.ts",
      "Include id, label, statuses, icon (Ban), accentColor, canDragFrom, canDropTo",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(frontend): add blocked group to Ready column workflow"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Create BlockReasonDialog component",
    "plan_section": "Task 7: BlockReasonDialog Component",
    "blocking": [8],
    "blockedBy": [],
    "atomic_commit": "feat(frontend): add BlockReasonDialog component",
    "steps": [
      "Read specs/plans/blocked_reason_feature.md section 'Task 7: BlockReasonDialog Component'",
      "Create src/components/tasks/BlockReasonDialog.tsx",
      "Add Dialog with title 'Block Task'",
      "Add Textarea for optional reason",
      "Add Cancel and Block buttons",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(frontend): add BlockReasonDialog component"
    ],
    "passes": false
  },
  {
    "id": 8,
    "category": "frontend",
    "description": "Integrate BlockReasonDialog in context menu",
    "plan_section": "Task 8: Update Context Menu",
    "blocking": [],
    "blockedBy": [5, 7],
    "atomic_commit": "feat(frontend): integrate BlockReasonDialog in context menu",
    "steps": [
      "Read specs/plans/blocked_reason_feature.md section 'Task 8: Update Context Menu'",
      "Modify TaskCardContextMenu.tsx to open BlockReasonDialog on Block action",
      "Add onBlockWithReason prop that accepts optional reason",
      "Wire the dialog submission to call the API",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(frontend): integrate BlockReasonDialog in context menu"
    ],
    "passes": false
  },
  {
    "id": 9,
    "category": "frontend",
    "description": "Display blocked reason on task cards",
    "plan_section": "Task 9: Display Blocked Reason on TaskCard",
    "blocking": [],
    "blockedBy": [4, 6],
    "atomic_commit": "feat(frontend): display blocked reason on task cards",
    "steps": [
      "Read specs/plans/blocked_reason_feature.md section 'Task 9: Display Blocked Reason on TaskCard'",
      "Modify TaskCard.tsx to show Ban icon when task.internalStatus === 'blocked'",
      "Display truncated reason text next to icon",
      "Add tooltip with full reason on hover",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(frontend): display blocked reason on task cards"
    ],
    "passes": false
  },
  {
    "id": 10,
    "category": "frontend",
    "description": "Add blockTask and unblockTask mutations",
    "plan_section": "Task 10: Hook/Mutation Updates",
    "blocking": [],
    "blockedBy": [5],
    "atomic_commit": "feat(frontend): add blockTask and unblockTask mutations",
    "steps": [
      "Read specs/plans/blocked_reason_feature.md section 'Task 10: Hook/Mutation Updates'",
      "Add blockTask mutation to useTaskMutation.ts (or similar hook)",
      "Add unblockTask mutation to useTaskMutation.ts",
      "Include query invalidation for task lists",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(frontend): add blockTask and unblockTask mutations"
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
| **Blocked group in Ready column** | Tasks are blocked before execution, so they belong with other pre-execution states |
| **Optional blocked_reason** | Allows quick blocking without requiring a reason, but supports documentation when needed |
| **TransitionHandler for state changes** | Consistent with existing state machine patterns, ensures side effects fire |
| **Parallel task execution** | Tasks 6 and 7 can run independently from the backend chain for efficiency |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] Migration adds blocked_reason column successfully
- [ ] block_task command transitions task to Blocked and sets reason
- [ ] unblock_task command transitions task to Ready and clears reason
- [ ] TaskResponse includes blocked_reason field

### Frontend - Run `npm run test`
- [ ] Task type includes blockedReason field
- [ ] BlockReasonDialog renders and submits correctly
- [ ] TaskCard displays blocked reason when present

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Block a Ready task - confirm dialog appears, enter reason
- [ ] Check Ready column - task appears in "Blocked" group
- [ ] Hover task card - tooltip shows full reason
- [ ] Unblock task - confirm it moves back to "Fresh Tasks" group
- [ ] Check database - blocked_reason column populated/cleared correctly

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] BlockReasonDialog is imported AND rendered (not behind disabled flag)
- [ ] Context menu Block action opens the dialog
- [ ] Dialog submission calls blockTask API
- [ ] blockTask API invokes Tauri command
- [ ] Task state updates in UI after blocking/unblocking

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
