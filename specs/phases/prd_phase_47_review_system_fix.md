# RalphX - Phase 47: Review System Fix

## Overview

This phase fixes three critical issues in the review system:

1. **Approve/Request Changes buttons are disabled** - The buttons in `HumanReviewTaskDetail` and `EscalatedTaskDetail` look for a review with `status === "pending"`, but after AI completes its review, no such review exists. The backend has HTTP endpoints (`approve_task`, `request_task_changes`) but these aren't exposed as Tauri commands.

2. **Reviews panel is empty** - The panel fetches reviews with `status === "pending"`, but after AI completes review, the status changes to approved/rejected. Need a task-based query instead.

3. **Hardcoded status strings scattered throughout codebase** - Status filtering/grouping logic is duplicated in many files. Consolidating to a single source of truth improves maintainability.

**Reference Plan:**
- `specs/plans/fix_review_system_buttons_reviews_panel_and_status_constants.md` - Detailed implementation plan with root cause analysis and solution design

## Goals

1. Enable human reviewers to approve tasks or request changes via working UI buttons
2. Populate the Reviews panel with tasks awaiting review (both AI and human)
3. Consolidate status constants to a single source of truth in `src/types/status.ts`

## Dependencies

### Phase 46 (Escalation Data Fix) - Required

| Dependency | Why Needed |
|------------|------------|
| Escalated state | Phase 45-46 added the escalated state that this phase's buttons must handle |
| ReviewIssue schema | The escalation issues display was fixed, now we fix the action buttons |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_review_system_buttons_reviews_panel_and_status_constants.md`
2. Understand the root cause analysis and solution design
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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_review_system_buttons_reviews_panel_and_status_constants.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add approve_task_for_review and request_task_changes_for_review Tauri commands",
    "plan_section": "Task 1: Backend - Add approve_task and request_task_changes Tauri commands",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(review_commands): add approve_task_for_review and request_task_changes_for_review commands",
    "steps": [
      "Read specs/plans/fix_review_system_buttons_reviews_panel_and_status_constants.md section 'Task 1'",
      "Add ApproveTaskInput and RequestTaskChangesInput structs to review_commands_types.rs",
      "Add approve_task_for_review command to review_commands.rs (reuse logic from HTTP handler)",
      "Add request_task_changes_for_review command to review_commands.rs (reuse logic from HTTP handler)",
      "Export new commands in commands/mod.rs",
      "Register new commands in lib.rs invoke_handler",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(review_commands): add approve_task_for_review and request_task_changes_for_review commands"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Add task-based approval API wrappers and update detail views",
    "plan_section": "Task 2: Frontend - Add API wrappers and update task detail views",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(reviews): add task-based approval API and update detail views",
    "steps": [
      "Read specs/plans/fix_review_system_buttons_reviews_panel_and_status_constants.md section 'Task 2'",
      "Add ApproveTaskInput and RequestTaskChangesInput interfaces to reviews-api.ts",
      "Add approveTask and requestTaskChanges methods to reviewsApi",
      "Update HumanReviewTaskDetail.tsx: remove pendingReview lookup, use task.id, update mutations",
      "Update EscalatedTaskDetail.tsx: same changes as HumanReviewTaskDetail",
      "Remove !reviewId disabled condition from ActionButtons",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(reviews): add task-based approval API and update detail views"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add get_tasks_awaiting_review query command",
    "plan_section": "Task 3: Backend - Add get_tasks_awaiting_review command",
    "blocking": [4],
    "blockedBy": [],
    "atomic_commit": "feat(task_commands): add get_tasks_awaiting_review query",
    "steps": [
      "Read specs/plans/fix_review_system_buttons_reviews_panel_and_status_constants.md section 'Task 3'",
      "Add get_tasks_awaiting_review command to task_commands/query.rs",
      "Query tasks where status IN ('pending_review', 'reviewing', 'review_passed', 'escalated')",
      "Export in task_commands/mod.rs",
      "Register in lib.rs invoke_handler",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(task_commands): add get_tasks_awaiting_review query"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Add getTasksAwaitingReview API wrapper",
    "plan_section": "Task 4: Frontend - Add API wrapper for tasks awaiting review",
    "blocking": [5],
    "blockedBy": [3],
    "atomic_commit": "feat(tasks-api): add getTasksAwaitingReview wrapper",
    "steps": [
      "Read specs/plans/fix_review_system_buttons_reviews_panel_and_status_constants.md section 'Task 4'",
      "Add getTasksAwaitingReview function to src/api/tasks.ts",
      "Use typedInvoke with appropriate schema (array of TaskResponse)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tasks-api): add getTasksAwaitingReview wrapper"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Update useReviews hook and ReviewsPanel to use task-based query",
    "plan_section": "Task 5: Frontend - Update useReviews hook and ReviewsPanel",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(reviews): update panel to use task-based query",
    "steps": [
      "Read specs/plans/fix_review_system_buttons_reviews_panel_and_status_constants.md section 'Task 5'",
      "Update or create useTasksAwaitingReview hook in src/hooks/useReviews.ts",
      "Return tasks grouped by review type: AI (pending_review, reviewing) and Human (review_passed, escalated)",
      "Update ReviewsPanel.tsx to use task-based hook instead of usePendingReviews",
      "Update tab filtering: AI tab shows pending_review/reviewing, Human tab shows review_passed/escalated",
      "Update card rendering to use task data",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(reviews): update panel to use task-based query"
    ],
    "passes": true
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Add consolidated status group constants to status.ts",
    "plan_section": "Task 6: Add status constants to status.ts",
    "blocking": [7],
    "blockedBy": [],
    "atomic_commit": "feat(status): add consolidated status group constants",
    "steps": [
      "Read specs/plans/fix_review_system_buttons_reviews_panel_and_status_constants.md section 'Task 6'",
      "Add EXECUTION_STATUSES, AI_REVIEW_STATUSES, HUMAN_REVIEW_STATUSES to src/types/status.ts",
      "Add ALL_REVIEW_STATUSES (spread of AI + Human)",
      "Add NON_DRAGGABLE_STATUSES",
      "Add helper functions: isExecutionStatus(), isAiReviewStatus(), isHumanReviewStatus(), isNonDraggableStatus()",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(status): add consolidated status group constants"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Update components to import from centralized status.ts",
    "plan_section": "Task 7: Update components to import from status.ts",
    "blocking": [],
    "blockedBy": [6],
    "atomic_commit": "refactor(components): use centralized status constants",
    "steps": [
      "Read specs/plans/fix_review_system_buttons_reviews_panel_and_status_constants.md section 'Task 7'",
      "Update TaskCard.utils.ts: import NON_DRAGGABLE_STATUSES, remove local definition",
      "Update IntegratedChatPanel.tsx: import EXECUTION_STATUSES, HUMAN_REVIEW_STATUSES, remove local definitions",
      "Update TaskFullView.tsx: import status constants, remove local definitions",
      "Verify no duplicate status arrays remain (grep for local definitions)",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(components): use centralized status constants"
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
| **Task-based approval instead of review-based** | HTTP endpoints already use task_id, and the review status changes after AI completion making review_id unreliable |
| **Task query for Reviews panel** | Fetching tasks by status is more reliable than fetching reviews by status since task status is the source of truth |
| **Centralized status constants** | Single source of truth prevents inconsistencies when status groups need to change |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] approve_task_for_review transitions task to Approved
- [ ] request_task_changes_for_review transitions task to RevisionNeeded
- [ ] get_tasks_awaiting_review returns tasks in correct statuses

### Frontend - Run `npm run test`
- [ ] reviewsApi.approveTask calls correct command
- [ ] reviewsApi.requestTaskChanges calls correct command
- [ ] tasksApi.getTasksAwaitingReview calls correct command

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Task in `review_passed` state: Approve button works, task moves to `approved`
- [ ] Task in `escalated` state: Approve button works, task moves to `approved`
- [ ] Request Changes button works, task moves to `revision_needed`
- [ ] Reviews panel AI tab shows tasks in `pending_review`, `reviewing`
- [ ] Reviews panel Human tab shows tasks in `review_passed`, `escalated`
- [ ] Clicking task in Reviews panel opens detail view with working buttons

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Approve button click → api.reviews.approveTask → approve_task_for_review command → task status change
- [ ] Request Changes button click → api.reviews.requestTaskChanges → request_task_changes_for_review command → task status change
- [ ] Reviews panel mount → useTasksAwaitingReview → get_tasks_awaiting_review → task list

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

### Status Constants Verification
- [ ] Grep for duplicate status arrays - should only exist in `types/status.ts`
- [ ] All components import from central location
- [ ] TypeScript compilation passes (ensures type safety)

See `.claude/rules/gap-verification.md` for full verification workflow.
