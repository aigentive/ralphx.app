# Plan: Fix Review System - Buttons, Reviews Panel, and Status Constants

## Problem Summary

Three issues to address:

1. **Approve/Request Changes buttons disabled** - The buttons in `HumanReviewTaskDetail` and `EscalatedTaskDetail` are disabled because they look for a review with `status === "pending"`, but no such review exists after AI completes its review.

2. **Reviews panel may not show tasks awaiting human review** - The panel fetches reviews with `status === "pending"`, but after AI completes review, the review status changes to approved/rejected.

3. **Hardcoded status strings scattered throughout codebase** - Status filtering/grouping logic is duplicated in many files making maintenance difficult.

## Root Cause Analysis

### Button Issue
- Frontend uses `api.reviews.approve({ review_id })` which calls `approve_review` Tauri command
- This command requires a review with `status === "pending"` (see `review_commands.rs:101-107`)
- When AI completes review (`complete_review` HTTP handler), it changes review status to `approved`/`rejected`/`changes_requested`
- So there's no pending review for human to approve

### Backend Has Correct Endpoints (But Only for HTTP/MCP)
- `approve_task` (HTTP, line 260-339 in `http_server/handlers/reviews.rs`)
  - Takes `task_id` (not review_id)
  - Validates task is in `ReviewPassed` or `Escalated`
  - Creates `ReviewNote` with human approval
  - Transitions task to `Approved`
- `request_task_changes` (HTTP, line 343-423)
  - Takes `task_id`
  - Creates `ReviewNote` with changes_requested
  - Transitions task to `RevisionNeeded`

**Problem:** These endpoints exist for MCP but NOT as Tauri commands for the frontend UI.

### Status Constants Issue
Hardcoded status arrays found in multiple files:
- `src/types/status.ts`: Has `IDLE_STATUSES`, `ACTIVE_STATUSES`, `TERMINAL_STATUSES`, `REVIEW_STATUSES`
- `src/components/tasks/TaskBoard/TaskCard.utils.ts`: `NON_DRAGGABLE_STATUSES`, `REVIEW_STATE_STATUSES`
- `src/components/Chat/IntegratedChatPanel.tsx`: `executionStatuses`, `reviewStatuses`
- `src/components/tasks/TaskFullView.tsx`: `reviewStatuses`, `executingStatuses`
- `src/types/workflow.ts`: Status lists in column definitions

**Problem:** Same status groupings defined in multiple places. Changes require updating multiple files.

## Solution

### Part 1: Add Tauri Commands for Task Approval

**Files to modify:**
- `src-tauri/src/commands/review_commands.rs` - Add new commands
- `src-tauri/src/commands/review_commands_types.rs` - Add input types
- `src-tauri/src/commands/mod.rs` - Export new commands
- `src-tauri/src/lib.rs` - Register new commands

**New commands:**
```rust
#[tauri::command]
pub async fn approve_task_for_review(
    input: ApproveTaskInput,
    state: State<'_, AppState>,
) -> Result<(), String>

#[tauri::command]
pub async fn request_task_changes_for_review(
    input: RequestTaskChangesInput,
    state: State<'_, AppState>,
) -> Result<(), String>
```

### Part 2: Add Frontend API Wrappers

**Files to modify:**
- `src/api/reviews-api.ts` - Add new API functions

**New API:**
```typescript
export interface ApproveTaskInput {
  task_id: string;
  notes?: string;
}

export interface RequestTaskChangesInput {
  task_id: string;
  feedback: string;
}

export const reviewsApi = {
  // ... existing methods
  approveTask: (input: ApproveTaskInput) =>
    typedInvoke("approve_task_for_review", { input }, z.void()),
  requestTaskChanges: (input: RequestTaskChangesInput) =>
    typedInvoke("request_task_changes_for_review", { input }, z.void()),
}
```

### Part 3: Update Task Detail Views

**Files to modify:**
- `src/components/tasks/detail-views/HumanReviewTaskDetail.tsx`
- `src/components/tasks/detail-views/EscalatedTaskDetail.tsx`

**Changes:**
1. Remove `pendingReview` lookup - not needed
2. Remove `reviewId` prop from `ActionButtons`
3. Pass `task.id` to `ActionButtons` instead
4. Update mutations to use new task-based API:
   - `api.reviews.approveTask({ task_id: taskId, notes })`
   - `api.reviews.requestTaskChanges({ task_id: taskId, feedback })`
5. Remove disabled condition for `!reviewId`

### Part 4: Fix Reviews Panel (Confirmed Empty)

**Issue:** The Reviews panel is completely empty because it fetches reviews with `status === "pending"`, but after AI completes its review, no pending reviews exist.

**Solution:** Add a new endpoint to fetch tasks awaiting review (both AI and human).

**Files to modify:**
- `src-tauri/src/commands/task_commands/query.rs` - Add new query command
- `src-tauri/src/commands/task_commands/mod.rs` - Export command
- `src-tauri/src/lib.rs` - Register command
- `src/api/tasks.ts` - Add frontend API wrapper
- `src/hooks/useReviews.ts` - Update to use task-based query
- `src/components/reviews/ReviewsPanel.tsx` - Update to use tasks instead of reviews

**New endpoint:**
```rust
#[tauri::command]
pub async fn get_tasks_awaiting_review(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TaskResponse>, String>
```

This returns tasks in:
- `pending_review` - Waiting for AI (AI tab)
- `reviewing` - AI is reviewing (AI tab)
- `review_passed` - AI approved, awaiting human (Human tab)
- `escalated` - AI escalated, awaiting human (Human tab)

**Tab filtering:**
- AI tab: `pending_review`, `reviewing`
- Human tab: `review_passed`, `escalated`

### Part 5: Consolidate Status Constants

**Goal:** Single source of truth for all status groupings.

**File to modify:**
- `src/types/status.ts` - Add all status groupings here

**New constants to add:**
```typescript
// ============================================================================
// Status Groups for UI Features
// ============================================================================

/** Statuses where task is in execution phase (worker running) */
export const EXECUTION_STATUSES = [
  "executing",
  "re_executing",
  "qa_refining",
  "qa_testing",
  "qa_passed",
  "qa_failed",
] as const satisfies readonly InternalStatus[];

/** Statuses where task is in AI review phase */
export const AI_REVIEW_STATUSES = [
  "pending_review",
  "reviewing",
] as const satisfies readonly InternalStatus[];

/** Statuses where task awaits human review decision */
export const HUMAN_REVIEW_STATUSES = [
  "review_passed",
  "escalated",
] as const satisfies readonly InternalStatus[];

/** All review-related statuses (AI + Human) */
export const ALL_REVIEW_STATUSES = [
  ...AI_REVIEW_STATUSES,
  ...HUMAN_REVIEW_STATUSES,
] as const;

/** Statuses where drag-drop is disabled (system-managed states) */
export const NON_DRAGGABLE_STATUSES = [
  "executing",
  "re_executing",
  "qa_refining",
  "qa_testing",
  "qa_passed",
  "qa_failed",
  "pending_review",
  "reviewing",
  "review_passed",
  "escalated",
  "revision_needed",
] as const satisfies readonly InternalStatus[];

// ============================================================================
// Helper Functions
// ============================================================================

export function isExecutionStatus(status: InternalStatus): boolean {
  return (EXECUTION_STATUSES as readonly string[]).includes(status);
}

export function isAiReviewStatus(status: InternalStatus): boolean {
  return (AI_REVIEW_STATUSES as readonly string[]).includes(status);
}

export function isHumanReviewStatus(status: InternalStatus): boolean {
  return (HUMAN_REVIEW_STATUSES as readonly string[]).includes(status);
}

export function isNonDraggableStatus(status: InternalStatus): boolean {
  return (NON_DRAGGABLE_STATUSES as readonly string[]).includes(status);
}
```

**Files to update (import from status.ts instead of defining locally):**
- `src/components/tasks/TaskBoard/TaskCard.utils.ts` - Import `NON_DRAGGABLE_STATUSES`
- `src/components/Chat/IntegratedChatPanel.tsx` - Import `EXECUTION_STATUSES`, `HUMAN_REVIEW_STATUSES`
- `src/components/tasks/TaskFullView.tsx` - Import status constants
- `src/hooks/useTaskExecutionState.ts` - Use helper functions

## Critical Files

| File | Purpose |
|------|---------|
| `src-tauri/src/commands/review_commands.rs` | Add new Tauri commands |
| `src-tauri/src/commands/review_commands_types.rs` | Add input types |
| `src-tauri/src/commands/task_commands/query.rs` | Add tasks awaiting review query |
| `src-tauri/src/lib.rs` | Register commands |
| `src/api/reviews-api.ts` | Add frontend API |
| `src/hooks/useReviews.ts` | Update hook to fetch tasks |
| `src/components/reviews/ReviewsPanel.tsx` | Update to use tasks |
| `src/components/tasks/detail-views/HumanReviewTaskDetail.tsx` | Fix buttons |
| `src/components/tasks/detail-views/EscalatedTaskDetail.tsx` | Fix buttons |
| `src/types/status.ts` | Add consolidated status constants |
| `src/components/tasks/TaskBoard/TaskCard.utils.ts` | Import status constants |
| `src/components/Chat/IntegratedChatPanel.tsx` | Import status constants |
| `src/components/tasks/TaskFullView.tsx` | Import status constants |

## Task Breakdown

### Part A: Fix Approve/Request Changes Buttons

### Task 1: Backend - Add approve_task and request_task_changes Tauri commands (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(review_commands): add approve_task_for_review and request_task_changes_for_review commands`

- Add `ApproveTaskInput` and `RequestTaskChangesInput` to `review_commands_types.rs`
- Add `approve_task_for_review` command to `review_commands.rs`
- Add `request_task_changes_for_review` command to `review_commands.rs`
- Export in `mod.rs` and register in `lib.rs`

**Compilation unit analysis:** All backend changes are additive (new types, new functions). No existing code modified. Compiles independently.

### Task 2: Frontend - Add API wrappers and update task detail views
**Dependencies:** Task 1
**Atomic Commit:** `feat(reviews): add task-based approval API and update detail views`

- Add input types to `reviews-api.ts`
- Add `approveTask` and `requestTaskChanges` methods
- Update `HumanReviewTaskDetail.tsx`:
  - Refactor ActionButtons to use task ID instead of review ID
  - Update mutations to use new task-based API
  - Remove `pendingReview` lookup and `!reviewId` disabled condition
- Update `EscalatedTaskDetail.tsx`:
  - Same changes as HumanReviewTaskDetail

**Compilation unit analysis:** API wrappers + consumers must be in same task. Adding new API methods (additive) + updating call sites to use new methods. All TypeScript changes compile together.

### Part B: Fix Reviews Panel

### Task 3: Backend - Add get_tasks_awaiting_review command (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(task_commands): add get_tasks_awaiting_review query`

- Add to `task_commands/query.rs`
- Query tasks where status IN ('pending_review', 'reviewing', 'review_passed', 'escalated')
- Export in `task_commands/mod.rs` and register in `lib.rs`

**Compilation unit analysis:** Additive backend change. New command, new export. Compiles independently.

### Task 4: Frontend - Add API wrapper for tasks awaiting review
**Dependencies:** Task 3
**Atomic Commit:** `feat(tasks-api): add getTasksAwaitingReview wrapper`

- Add `getTasksAwaitingReview` to `src/api/tasks.ts`

**Compilation unit analysis:** Additive. New function export. Compiles independently.

### Task 5: Frontend - Update useReviews hook and ReviewsPanel
**Dependencies:** Task 4
**Atomic Commit:** `feat(reviews): update panel to use task-based query`

- Update or create `useTasksAwaitingReview` hook in `src/hooks/useReviews.ts`
- Returns tasks grouped by review type (AI/Human)
- Update `ReviewsPanel.tsx`:
  - Replace `usePendingReviews` with task-based hook
  - Update tab filtering logic:
    - AI tab: tasks in `pending_review`, `reviewing`
    - Human tab: tasks in `review_passed`, `escalated`
  - Update card rendering to use task data

**Compilation unit analysis:** Hook change + consumer update. Must be same task since ReviewsPanel depends on hook's return type.

### Part C: Consolidate Status Constants

### Task 6: Add status constants to status.ts (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(status): add consolidated status group constants`

- Add `EXECUTION_STATUSES`, `AI_REVIEW_STATUSES`, `HUMAN_REVIEW_STATUSES`
- Add `ALL_REVIEW_STATUSES`, `NON_DRAGGABLE_STATUSES`
- Add helper functions: `isExecutionStatus()`, `isAiReviewStatus()`, `isHumanReviewStatus()`, `isNonDraggableStatus()`

**Compilation unit analysis:** Additive exports. New constants and functions. Compiles independently.

### Task 7: Update components to import from status.ts
**Dependencies:** Task 6
**Atomic Commit:** `refactor(components): use centralized status constants`

- Update `TaskCard.utils.ts`:
  - Import `NON_DRAGGABLE_STATUSES` from `@/types/status`
  - Remove local `NON_DRAGGABLE_STATUSES` definition
  - Use imported `REVIEW_STATUSES` for `REVIEW_STATE_STATUSES`
- Update `IntegratedChatPanel.tsx`:
  - Import `EXECUTION_STATUSES`, `HUMAN_REVIEW_STATUSES` from `@/types/status`
  - Remove local `executionStatuses`, `reviewStatuses` definitions
- Update `TaskFullView.tsx`:
  - Import status constants from `@/types/status`
  - Remove local status array definitions

**Compilation unit analysis:** All import changes can happen together. Each file independently imports from the new central location. Compiles after Task 6.

### Testing

### Task 8: Run tests and verify
**Dependencies:** Tasks 1-7
**Atomic Commit:** N/A (verification only)

- Backend: `cargo clippy && cargo test`
- Frontend: `npm run lint && npm run typecheck`
- Manual test: Full review flow

## Verification

1. **Backend tests pass:** `cargo clippy && cargo test`
2. **Frontend lint passes:** `npm run lint && npm run typecheck`
3. **Manual verification - Buttons:**
   - Task in `review_passed` state: Approve button works, task moves to `approved`
   - Task in `escalated` state: Approve button works, task moves to `approved`
   - Request Changes button works, task moves to `revision_needed` and re-executes
4. **Manual verification - Reviews Panel:**
   - Open Reviews panel from navbar
   - AI tab shows tasks in `pending_review` and `reviewing` states
   - Human tab shows tasks in `review_passed` and `escalated` states
   - Clicking on a task opens the detail view with working buttons
5. **Status constants verification:**
   - Grep for duplicate status arrays - should only exist in `types/status.ts`
   - All components import from central location
   - TypeScript compilation passes (ensures type safety)

## Task Dependency Graph

```
Part A (Buttons):
  Task 1 (Backend) ──► Task 2 (Frontend)

Part B (Reviews Panel):
  Task 3 (Backend) ──► Task 4 (Frontend API) ──► Task 5 (Hook + Panel)

Part C (Status Constants):
  Task 6 (Add constants) ──► Task 7 (Update imports)

Verification:
  Tasks 1-7 ──► Task 8 (Testing)
```

**Parallelization opportunities:**
- Tasks 1, 3, 6 can run in parallel (independent backend/frontend additive changes)
- Tasks 2, 4, 7 must wait for their dependencies
- Task 5 must wait for Task 4
- Task 8 runs last

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
