# Plan: Add Confirmation Dialogs to All Task State Transitions

## Overview
Add confirmation dialogs using the existing `useConfirmation` hook for all task state transition actions across the application.

## Implementation Tasks

### Task 1: StatusDropdown - Add confirmation for status transitions
**Dependencies:** None
**Atomic Commit:** `feat(tasks): add confirmation dialog to StatusDropdown`

**File:** `src/components/tasks/StatusDropdown.tsx`

- **Current:** Calls `onTransition(newStatus)` directly on select
- **Change:** Add `useConfirmation`, confirm before calling `onTransition`
- **Message pattern:** "Change status to {newStatus}?" / "This will move the task to {status}."

### Task 2: TaskDetailOverlay - Add confirmation for archive/restore
**Dependencies:** None
**Atomic Commit:** `feat(tasks): add confirmation dialogs to TaskDetailOverlay archive/restore`

**File:** `src/components/tasks/TaskDetailOverlay.tsx`

- **Archive button (line 483):** Add confirmation - "Archive this task?"
- **Restore button (line 501):** Add confirmation - "Restore this task?"
- **Delete already has confirmation** - no change needed

### Task 3: TaskCardContextMenu - Add confirmation for all context menu actions (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(tasks): add confirmation dialogs to TaskCardContextMenu`

**File:** `src/components/tasks/TaskCardContextMenu.tsx`

- **Current:** All actions directly call callbacks
- **Changes needed for:**
  - Cancel → "Cancel this task?" (destructive)
  - Block → "Block this task?"
  - Unblock → "Unblock this task?"
  - Re-open → "Re-open this task?"
  - Retry → "Retry this task?"
  - Archive → "Archive this task?"
  - Restore → "Restore this task?"
  - Delete → "Permanently delete this task?" (destructive)

### Task 4: ReviewDetailModal - Add confirmation for approve action
**Dependencies:** None
**Atomic Commit:** `feat(reviews): add confirmation dialog to ReviewDetailModal approve`

**File:** `src/components/reviews/ReviewDetailModal.tsx`

- **Approve button (line 573):** Add confirmation - "Approve this task?"
- **Request Changes:** No change needed (already two-step with feedback textarea)

### Task 5: HumanReviewTaskDetail - Add confirmation for approve action
**Dependencies:** None
**Atomic Commit:** `feat(tasks): add confirmation dialog to HumanReviewTaskDetail approve`

**File:** `src/components/tasks/detail-views/HumanReviewTaskDetail.tsx`

- **Approve (line 278):** Add confirmation - "Approve this task?"
- **Request Changes:** No change needed (already two-step with feedback textarea)

### Task 6: EscalatedTaskDetail - Add confirmation for approve action
**Dependencies:** None
**Atomic Commit:** `feat(tasks): add confirmation dialog to EscalatedTaskDetail approve`

**File:** `src/components/tasks/detail-views/EscalatedTaskDetail.tsx`

- **Approve (line 315):** Add confirmation - "Approve this task?"
- **Request Changes:** No change needed (already two-step with feedback textarea)

## Implementation Pattern

Use the existing `useConfirmation` hook consistently:

```tsx
import { useConfirmation } from "@/hooks/useConfirmation";

// In component:
const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();

// In handler:
const handleAction = async () => {
  const confirmed = await confirm({
    title: "Action Title?",
    description: "Description of what will happen.",
    confirmText: "Confirm",
    variant: "default", // or "destructive" for delete/cancel
  });
  if (!confirmed) return;
  // proceed with action
};

// In render (once per component):
<ConfirmationDialog {...confirmationDialogProps} />
```

## Confirmation Messages

| Action | Title | Description | Variant |
|--------|-------|-------------|---------|
| Status change | "Change status to {status}?" | "This will move the task to {status}." | default |
| Archive | "Archive this task?" | "The task will be moved to the archive." | default |
| Restore | "Restore this task?" | "The task will be restored to the backlog." | default |
| Cancel | "Cancel this task?" | "The task will be marked as cancelled." | destructive |
| Block | "Block this task?" | "The task will be marked as blocked." | default |
| Unblock | "Unblock this task?" | "The task will be moved back to ready." | default |
| Re-open | "Re-open this task?" | "The task will be moved to backlog." | default |
| Retry | "Retry this task?" | "The task will be queued for re-execution." | default |
| Delete | "Delete permanently?" | "This will permanently delete the task. This action cannot be undone." | destructive |
| Approve | "Approve this task?" | "The task will be marked as approved and completed." | default |

**Note:** Request Changes skipped - already has two-step process (click → feedback textarea → submit).

## Critical Files

- `src/hooks/useConfirmation.tsx` (reference only)
- `src/components/tasks/StatusDropdown.tsx`
- `src/components/tasks/TaskDetailOverlay.tsx`
- `src/components/tasks/TaskCardContextMenu.tsx`
- `src/components/reviews/ReviewDetailModal.tsx`
- `src/components/tasks/detail-views/HumanReviewTaskDetail.tsx`
- `src/components/tasks/detail-views/EscalatedTaskDetail.tsx`

## Verification

1. Open app with dev server running
2. Test each component:
   - StatusDropdown: Change status from task detail → confirm dialog appears
   - TaskDetailOverlay: Click archive/restore → confirm dialog appears
   - TaskCardContextMenu: Right-click task → each action shows confirmation
   - ReviewDetailModal: Open review → approve/request changes shows confirmation
   - Detail views: Check approve/request changes in each review state
3. Verify cancel dismisses dialog without action
4. Verify confirm proceeds with action
5. Run `npm run lint && npm run typecheck`

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Parallel Execution Note

All 6 tasks in this plan are **independent** and can be executed in parallel:
- Each modifies a separate component file
- No cross-component type changes
- No shared state modifications
- Each task compiles independently after completion
