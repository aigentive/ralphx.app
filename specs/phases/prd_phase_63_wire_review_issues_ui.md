# RalphX - Phase 63: Wire Review Issues to UI Detail Views

## Overview

Phase 60 implemented a full **Review Issues** system with backend CRUD operations, lifecycle management, and reusable UI components. However, the detail views still use legacy patterns: `ReviewNoteResponse.issues` or manual text parsing. This phase wires the existing `reviewIssuesApi` and `IssueList`/`IssueProgressBar` components to all relevant detail views for consistent issue display.

**Reference Plan:**
- `specs/plans/wire_phase_60_review_issues_to_ui_detail_views.md` - Detailed implementation steps for wiring Phase 60 Review Issues to UI

## Goals

1. Replace legacy `review?.issues` usage with `reviewIssuesApi.getByTaskId()` queries
2. Remove manual `parseIssuesFromNotes()` parsing in favor of structured API data
3. Display open issues during re-execution for worker visibility
4. Unify issue rendering using the shared `IssueList` component

## Dependencies

### Phase 60 (Review Issues as First-Class Entities) - Required

| Dependency | Why Needed |
|------------|------------|
| `review_issues` table | Backend data source for structured issues |
| `reviewIssuesApi` | Frontend API for fetching issues by task |
| `IssueList` component | Reusable UI for rendering issues with grouping |
| `IssueProgressBar` component | Visual progress indicator for issue resolution |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/wire_phase_60_review_issues_to_ui_detail_views.md`
2. Understand the existing issue rendering in each detail view
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Add imports for `reviewIssuesApi`, `IssueList`, `useQuery`
3. Add query hook to fetch issues from API
4. Replace legacy issue rendering with `IssueList` component
5. Run linters: `npm run lint && npm run typecheck`
6. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`

**Task Execution Order:**
- All tasks are independent and can be executed in parallel
- Tasks 1-4 have no dependencies on each other
- Task 5 is P1 (lower priority)

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/wire_phase_60_review_issues_to_ui_detail_views.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Wire HumanReviewTaskDetail to review issues API",
    "plan_section": "Task 1: HumanReviewTaskDetail",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(detail-views): wire HumanReviewTaskDetail to review issues API",
    "steps": [
      "Read specs/plans/wire_phase_60_review_issues_to_ui_detail_views.md section 'Task 1'",
      "Add imports for useQuery, reviewIssuesApi, IssueList, IssueProgressBar",
      "Add queries to fetch issues and progress by task.id",
      "Pass fetched issues to AIReviewCard instead of review?.issues",
      "Replace custom issue rendering with <IssueList issues={issues} compact />",
      "Add <IssueProgressBar progress={progress} /> where appropriate",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(detail-views): wire HumanReviewTaskDetail to review issues API"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Wire RevisionTaskDetail to review issues API",
    "plan_section": "Task 2: RevisionTaskDetail",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(detail-views): wire RevisionTaskDetail to review issues API",
    "steps": [
      "Read specs/plans/wire_phase_60_review_issues_to_ui_detail_views.md section 'Task 2'",
      "Add imports for useQuery, reviewIssuesApi, IssueList",
      "Add query to fetch issues by task.id",
      "Keep FeedbackCard for reviewer context (who requested changes, when)",
      "Replace parseIssuesFromNotes() usage with <IssueList issues={issues} groupBy=\"status\" />",
      "Remove parseIssuesFromNotes function and IssueItem component",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(detail-views): wire RevisionTaskDetail to review issues API"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Show open issues in ExecutionTaskDetail for re-execution",
    "plan_section": "Task 3: ExecutionTaskDetail - Open Issues for Re-Execution",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(detail-views): show open issues in ExecutionTaskDetail for re-execution",
    "steps": [
      "Read specs/plans/wire_phase_60_review_issues_to_ui_detail_views.md section 'Task 3'",
      "Add imports for useQuery, reviewIssuesApi, IssueList",
      "Add conditional query for open issues (only when re-executing)",
      "Add new section after RevisionFeedbackCard showing issues to address",
      "Use <IssueList issues={openIssues} groupBy=\"severity\" compact />",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(detail-views): show open issues in ExecutionTaskDetail for re-execution"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Wire EscalatedTaskDetail to review issues API",
    "plan_section": "Task 4: EscalatedTaskDetail - Use Structured Review Issues",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(detail-views): wire EscalatedTaskDetail to review issues API",
    "steps": [
      "Read specs/plans/wire_phase_60_review_issues_to_ui_detail_views.md section 'Task 4'",
      "Add imports for useQuery, reviewIssuesApi, IssueList",
      "Add query to fetch issues by task.id",
      "Replace custom IssueCard component usage with <IssueList issues={issues} groupBy=\"severity\" />",
      "Keep EscalationReasonCard for escalation reason text",
      "Remove local IssueCard component",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(detail-views): wire EscalatedTaskDetail to review issues API"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Add issue progress bar to WaitingTaskDetail (P1)",
    "plan_section": "Task 5 (P1): WaitingTaskDetail - Issue Progress Summary",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(detail-views): add issue progress bar to WaitingTaskDetail",
    "steps": [
      "Read specs/plans/wire_phase_60_review_issues_to_ui_detail_views.md section 'Task 5'",
      "Add imports for useQuery, reviewIssuesApi, IssueProgressBar",
      "Add query to fetch issue progress by task.id",
      "Add <IssueProgressBar progress={progress} /> to show resolution status",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(detail-views): add issue progress bar to WaitingTaskDetail"
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
| **Use existing IssueList component** | Consistent UI across all views, already handles grouping and styling |
| **Query-based data fetching** | React Query handles caching, refetching, and loading states |
| **Additive changes only** | Each task is self-contained, no breaking changes to other views |
| **Keep context cards** | FeedbackCard, EscalationReasonCard provide reviewer context beyond just issues |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] No regressions in existing tests

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Task in `review_passed` → HumanReviewTaskDetail shows structured issues via IssueList
- [ ] Task in `revision_needed` → RevisionTaskDetail shows issues with status grouping
- [ ] Task in `re_executing` → ExecutionTaskDetail shows open issues to address
- [ ] Task in `escalated` → EscalatedTaskDetail shows structured issues with severity grouping

### Wiring Verification

**For each modified component, verify the full path from user action to code:**

- [ ] IssueList renders issues from reviewIssuesApi (not legacy review?.issues)
- [ ] useQuery hooks are properly keyed by task.id
- [ ] No duplicate issue fetching across components

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
