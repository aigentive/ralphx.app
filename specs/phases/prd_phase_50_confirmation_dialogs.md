# RalphX - Phase 50: Add Confirmation Dialogs to All Task State Transitions

## Overview

This phase adds confirmation dialogs to all task state transition actions across the application, ensuring users can confirm before making changes. The implementation uses the existing `useConfirmation` hook consistently across 6 components that handle task state transitions.

**Reference Plan:**
- `specs/plans/add_confirmation_dialogs_to_all_task_state_transitions.md` - Implementation pattern, confirmation messages table, and file-by-file changes

## Goals

1. Add confirmation dialogs to StatusDropdown for all status transitions
2. Add confirmation dialogs to TaskDetailOverlay for archive/restore actions
3. Add confirmation dialogs to TaskCardContextMenu for all context menu actions
4. Add confirmation dialogs to review components (ReviewDetailModal, HumanReviewTaskDetail, EscalatedTaskDetail) for approve actions

## Dependencies

### Phase 47 (Review System Fix) - Required

| Dependency | Why Needed |
|------------|------------|
| Status constants consolidation | Confirmation dialogs need consistent status labels |
| Review panel functionality | Approve button must work before adding confirmation |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/add_confirmation_dialogs_to_all_task_state_transitions.md`
2. Understand the useConfirmation hook pattern
3. Reference the Confirmation Messages table for each action
4. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Import and use `useConfirmation` hook
3. Wrap action handlers with confirmation
4. Add `<ConfirmationDialog {...confirmationDialogProps} />` to render
5. Run linters for modified code only (frontend: `npm run lint && npm run typecheck`)
6. Commit with descriptive message

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
- All tasks have `"blockedBy": []` - they can start immediately and in parallel
- Each modifies a separate component file with no shared dependencies

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/add_confirmation_dialogs_to_all_task_state_transitions.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Add confirmation dialog to StatusDropdown for status transitions",
    "plan_section": "Task 1: StatusDropdown - Add confirmation for status transitions",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(tasks): add confirmation dialog to StatusDropdown",
    "steps": [
      "Read specs/plans/add_confirmation_dialogs_to_all_task_state_transitions.md section 'Task 1'",
      "Import useConfirmation hook in StatusDropdown.tsx",
      "Add confirm wrapper to onTransition callback in DropdownMenuItem onClick",
      "Use message pattern: 'Change status to {status}?' / 'This will move the task to {status}.'",
      "Add ConfirmationDialog to component render",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tasks): add confirmation dialog to StatusDropdown"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Add confirmation dialogs to TaskDetailOverlay for archive/restore actions",
    "plan_section": "Task 2: TaskDetailOverlay - Add confirmation for archive/restore",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(tasks): add confirmation dialogs to TaskDetailOverlay archive/restore",
    "steps": [
      "Read specs/plans/add_confirmation_dialogs_to_all_task_state_transitions.md section 'Task 2'",
      "Import useConfirmation hook in TaskDetailOverlay.tsx",
      "Wrap archive button handler with confirmation: 'Archive this task?'",
      "Wrap restore button handler with confirmation: 'Restore this task?'",
      "Note: Delete already has confirmation - no change needed",
      "Add ConfirmationDialog to component render",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tasks): add confirmation dialogs to TaskDetailOverlay archive/restore"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Add confirmation dialogs to TaskCardContextMenu for all context menu actions",
    "plan_section": "Task 3: TaskCardContextMenu - Add confirmation for all context menu actions",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(tasks): add confirmation dialogs to TaskCardContextMenu",
    "steps": [
      "Read specs/plans/add_confirmation_dialogs_to_all_task_state_transitions.md section 'Task 3'",
      "Import useConfirmation hook in TaskCardContextMenu.tsx",
      "Wrap each action with confirmation using messages from plan table:",
      "  - Cancel: destructive variant",
      "  - Block/Unblock/Re-open/Retry: default variant",
      "  - Archive/Restore: default variant",
      "  - Delete: destructive variant",
      "Add ConfirmationDialog to component render",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tasks): add confirmation dialogs to TaskCardContextMenu"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Add confirmation dialog to ReviewDetailModal for approve action",
    "plan_section": "Task 4: ReviewDetailModal - Add confirmation for approve action",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(reviews): add confirmation dialog to ReviewDetailModal approve",
    "steps": [
      "Read specs/plans/add_confirmation_dialogs_to_all_task_state_transitions.md section 'Task 4'",
      "Import useConfirmation hook in ReviewDetailModal.tsx",
      "Wrap approve button handler with confirmation: 'Approve this task?'",
      "Note: Request Changes already has two-step process - no change needed",
      "Add ConfirmationDialog to component render",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(reviews): add confirmation dialog to ReviewDetailModal approve"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Add confirmation dialog to HumanReviewTaskDetail for approve action",
    "plan_section": "Task 5: HumanReviewTaskDetail - Add confirmation for approve action",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(tasks): add confirmation dialog to HumanReviewTaskDetail approve",
    "steps": [
      "Read specs/plans/add_confirmation_dialogs_to_all_task_state_transitions.md section 'Task 5'",
      "Import useConfirmation hook in HumanReviewTaskDetail.tsx",
      "Wrap approve button handler with confirmation: 'Approve this task?'",
      "Note: Request Changes already has two-step process - no change needed",
      "Add ConfirmationDialog to component render",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tasks): add confirmation dialog to HumanReviewTaskDetail approve"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Add confirmation dialog to EscalatedTaskDetail for approve action",
    "plan_section": "Task 6: EscalatedTaskDetail - Add confirmation for approve action",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(tasks): add confirmation dialog to EscalatedTaskDetail approve",
    "steps": [
      "Read specs/plans/add_confirmation_dialogs_to_all_task_state_transitions.md section 'Task 6'",
      "Import useConfirmation hook in EscalatedTaskDetail.tsx",
      "Wrap approve button handler with confirmation: 'Approve this task?'",
      "Note: Request Changes already has two-step process - no change needed",
      "Add ConfirmationDialog to component render",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tasks): add confirmation dialog to EscalatedTaskDetail approve"
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
| **Use existing useConfirmation hook** | Consistent UX, already battle-tested in delete confirmations |
| **All tasks independent** | Each component is self-contained, allows parallel execution |
| **Skip Request Changes confirmation** | Already has two-step process (click → textarea → submit) |
| **Destructive variant for cancel/delete** | Visual distinction for irreversible or significant actions |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] All confirmation dialogs render correctly
- [ ] Cancel button dismisses without action
- [ ] Confirm button proceeds with action

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] StatusDropdown: Change status from task detail → confirm dialog appears
- [ ] TaskDetailOverlay: Click archive → confirm dialog appears
- [ ] TaskDetailOverlay: Click restore → confirm dialog appears
- [ ] TaskCardContextMenu: Right-click task → each action shows confirmation
- [ ] ReviewDetailModal: Click approve → confirm dialog appears
- [ ] HumanReviewTaskDetail: Click approve → confirm dialog appears
- [ ] EscalatedTaskDetail: Click approve → confirm dialog appears
- [ ] All dialogs: Cancel dismisses without action
- [ ] All dialogs: Confirm proceeds with action

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] useConfirmation hook imported in each component
- [ ] ConfirmationDialog component rendered in each component
- [ ] confirm() called before each action handler
- [ ] Action only executes after confirmation returns true

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
