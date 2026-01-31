# RalphX - Phase 53: Review Timeline Unification

## Overview

Extract a shared `ReviewTimeline` component from CompletedTaskDetail for use in both state-specific detail views (CompletedTaskDetail and EscalatedTaskDetail). This reduces code duplication from 3 implementations to 2, while keeping `StateHistoryTimeline` separate for generic task views.

The unification uses a filter pattern to support different UX needs: EscalatedTaskDetail shows only `changes_requested` entries with numbered attempts, while CompletedTaskDetail shows the complete review history timeline.

**Reference Plan:**
- `specs/plans/review_history_widget_unification.md` - Full analysis of current implementations, comparison matrix, and implementation steps

## Goals

1. Extract shared `ReviewTimeline` component with configurable filtering
2. Update CompletedTaskDetail to use shared component (no functional change)
3. Update EscalatedTaskDetail to use shared component with filter for `changes_requested` entries
4. Maintain visual consistency with existing detail view styling

## Dependencies

### Phase 20 (Review System) - Required

| Dependency | Why Needed |
|------------|------------|
| State-specific detail views | CompletedTaskDetail and EscalatedTaskDetail already exist |
| ReviewNoteResponse type | Used for review history data structure |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/review_history_widget_unification.md`
2. Understand the component structure and filter pattern
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

**Parallel Execution:** Tasks 2 and 3 can run in parallel after Task 1 completes.

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/review_history_widget_unification.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Extract ReviewTimeline component to shared location with filter props",
    "plan_section": "Task 1: Extract ReviewTimeline to shared location",
    "blocking": [2, 3],
    "blockedBy": [],
    "atomic_commit": "refactor(detail-views): extract ReviewTimeline to shared",
    "steps": [
      "Read specs/plans/review_history_widget_unification.md section 'Task 1'",
      "Create src/components/tasks/detail-views/shared/ReviewTimeline.tsx",
      "Move HistoryTimelineItem and ReviewHistoryTimeline from CompletedTaskDetail.tsx",
      "Add props interface: filter?, emptyMessage?, showAttemptNumbers?",
      "Implement filter logic in ReviewTimeline component",
      "Add showAttemptNumbers support for escalated view's '#1, #2' pattern",
      "Export ReviewTimeline from src/components/tasks/detail-views/shared/index.ts",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(detail-views): extract ReviewTimeline to shared"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Update CompletedTaskDetail to import ReviewTimeline from shared",
    "plan_section": "Task 2: Update CompletedTaskDetail",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "refactor(detail-views): use shared ReviewTimeline in CompletedTaskDetail",
    "steps": [
      "Read specs/plans/review_history_widget_unification.md section 'Task 2'",
      "Remove HistoryTimelineItem and ReviewHistoryTimeline from CompletedTaskDetail.tsx",
      "Import ReviewTimeline from './shared'",
      "Use <ReviewTimeline history={history} /> (no filter, default behavior)",
      "Verify no functional change - all entries still displayed",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(detail-views): use shared ReviewTimeline in CompletedTaskDetail"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Update EscalatedTaskDetail to use ReviewTimeline with changes_requested filter",
    "plan_section": "Task 3: Update EscalatedTaskDetail",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "refactor(detail-views): use shared ReviewTimeline in EscalatedTaskDetail",
    "steps": [
      "Read specs/plans/review_history_widget_unification.md section 'Task 3'",
      "Remove PreviousAttemptsSection from EscalatedTaskDetail.tsx",
      "Import ReviewTimeline from './shared'",
      "Use <ReviewTimeline history={history} filter={(e) => e.outcome === 'changes_requested'} showAttemptNumbers emptyMessage='No previous attempts' />",
      "Verify escalated view shows only changes_requested entries with numbered attempts",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(detail-views): use shared ReviewTimeline in EscalatedTaskDetail"
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
| **Keep StateHistoryTimeline separate** | Different context (generic views vs state-specific), self-fetching data, premium styling that suits modal/panel context |
| **Use filter prop pattern** | Allows same component to serve both EscalatedTaskDetail (filtered) and CompletedTaskDetail (all entries) |
| **Base on CompletedTaskDetail's implementation** | Already uses consistent styling tokens (rgba white pattern), good timeline visual, medium complexity |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] ReviewTimeline renders all entries when no filter
- [ ] ReviewTimeline filters entries correctly when filter prop provided
- [ ] ReviewTimeline shows attempt numbers when showAttemptNumbers=true
- [ ] ReviewTimeline shows empty message when filtered list is empty

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] CompletedTaskDetail shows full review history timeline (all entries)
- [ ] EscalatedTaskDetail shows only changes_requested entries with "#1", "#2" numbering
- [ ] Empty state displays correctly in both views
- [ ] Notes display correctly (full notes in both views)

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] ReviewTimeline imported AND used in CompletedTaskDetail
- [ ] ReviewTimeline imported AND used in EscalatedTaskDetail
- [ ] ReviewTimeline exported from shared/index.ts

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
