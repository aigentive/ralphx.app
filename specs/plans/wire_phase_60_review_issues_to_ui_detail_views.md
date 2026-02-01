# Plan: Wire Phase 60 Review Issues to UI Detail Views

## Problem Summary

Phase 60 implemented a full **Review Issues** system with:
- Backend: `review_issues` table, CRUD operations, status lifecycle (open→in_progress→addressed→verified)
- API: `reviewIssuesApi` at `src/api/review-issues.ts`
- Components: `IssueList`, `IssueProgressBar` at `src/components/reviews/IssueList.tsx`

**But:** Detail views don't use this system properly:

| View | File | Current | Problem |
|------|------|---------|---------|
| HumanReviewTaskDetail | `:66-150` | `review?.issues` from ReviewNoteResponse | Legacy flat structure |
| RevisionTaskDetail | `:64-130` | `parseIssuesFromNotes()` text parsing | Manual parsing, loses types |
| ExecutionTaskDetail | `:163-172` | Only shows feedback text | No open issues for re-execution |

Only `StateHistoryTimeline` properly uses `reviewIssuesApi`.

## Critical Files

| File | Purpose |
|------|---------|
| `src/api/review-issues.ts` | API: `getByTaskId()`, `getProgress()` |
| `src/components/reviews/IssueList.tsx` | `IssueList`, `IssueProgressBar` components |
| `src/types/review-issue.ts` | `ReviewIssue`, `IssueProgressSummary` types |

## Tasks

### Task 1: HumanReviewTaskDetail
**Dependencies:** None
**Atomic Commit:** `feat(detail-views): wire HumanReviewTaskDetail to review issues API`

**File:** `src/components/tasks/detail-views/HumanReviewTaskDetail.tsx`

**Current (lines 61-210):**
- `AIReviewCard` uses `review?.issues` from `ReviewNoteResponse`
- Custom inline rendering with severity badges

**Changes:**
1. Add imports:
   ```tsx
   import { useQuery } from "@tanstack/react-query";
   import { reviewIssuesApi } from "@/api/review-issues";
   import { IssueList, IssueProgressBar } from "@/components/reviews/IssueList";
   ```
2. Add query in `HumanReviewTaskDetail` (line ~391):
   ```tsx
   const { data: issues = [] } = useQuery({
     queryKey: ["review-issues", task.id],
     queryFn: () => reviewIssuesApi.getByTaskId(task.id),
   });
   const { data: progress } = useQuery({
     queryKey: ["issue-progress", task.id],
     queryFn: () => reviewIssuesApi.getProgress(task.id),
   });
   ```
3. Pass `issues` to `AIReviewCard` instead of `review?.issues`
4. Replace custom issue rendering (lines 114-150) with `<IssueList issues={issues} compact />`
5. Add `<IssueProgressBar progress={progress} />` if progress exists

### Task 2: RevisionTaskDetail
**Dependencies:** None
**Atomic Commit:** `feat(detail-views): wire RevisionTaskDetail to review issues API`

**File:** `src/components/tasks/detail-views/RevisionTaskDetail.tsx`

**Current:**
- `parseIssuesFromNotes()` function (lines 64-85)
- Custom `IssueItem` component (lines 90-130)
- `FeedbackCard` manually parses and renders issues

**Changes:**
1. Add imports for `reviewIssuesApi`, `IssueList`
2. Add query:
   ```tsx
   const { data: issues = [] } = useQuery({
     queryKey: ["review-issues", task.id],
     queryFn: () => reviewIssuesApi.getByTaskId(task.id),
   });
   ```
3. Keep `FeedbackCard` for reviewer context (who requested changes, when)
4. Replace `parseIssuesFromNotes()` usage with `<IssueList issues={issues} groupBy="status" />`
5. Remove `parseIssuesFromNotes` function and `IssueItem` component

### Task 3: ExecutionTaskDetail - Open Issues for Re-Execution
**Dependencies:** None
**Atomic Commit:** `feat(detail-views): show open issues in ExecutionTaskDetail for re-execution`

**File:** `src/components/tasks/detail-views/ExecutionTaskDetail.tsx`

**Current (lines 163-172):**
- Only shows `RevisionFeedbackCard` with plain text `feedback.notes`
- Worker doesn't see structured issues to address

**Changes:**
1. Add imports for `reviewIssuesApi`, `IssueList`
2. Add conditional query (only when re-executing):
   ```tsx
   const { data: openIssues = [] } = useQuery({
     queryKey: ["review-issues", task.id, "open"],
     queryFn: () => reviewIssuesApi.getByTaskId(task.id, "open"),
     enabled: task.internalStatus === "re_executing",
   });
   ```
3. Add new section after `RevisionFeedbackCard` (around line 172):
   ```tsx
   {isReExecuting && openIssues.length > 0 && (
     <section data-testid="open-issues-section">
       <SectionTitle>Issues to Address ({openIssues.length})</SectionTitle>
       <DetailCard>
         <IssueList issues={openIssues} groupBy="severity" compact />
       </DetailCard>
     </section>
   )}
   ```

### Task 4: EscalatedTaskDetail - Use Structured Review Issues
**Dependencies:** None
**Atomic Commit:** `feat(detail-views): wire EscalatedTaskDetail to review issues API`

**File:** `src/components/tasks/detail-views/EscalatedTaskDetail.tsx`

**Current:**
- Line 97: `const issues = review?.issues ?? [];` - uses legacy `ReviewIssue` from `ReviewNoteResponse`
- Lines 52-91: Custom `IssueCard` component with severity rendering
- Lines 130-142: Renders issues using custom component

**Changes:**
1. Add imports for `reviewIssuesApi`, `IssueList`
2. Add query:
   ```tsx
   const { data: issues = [] } = useQuery({
     queryKey: ["review-issues", task.id],
     queryFn: () => reviewIssuesApi.getByTaskId(task.id),
   });
   ```
3. Replace custom `IssueCard` component usage with `<IssueList issues={issues} groupBy="severity" />`
4. Keep `EscalationReasonCard` for the escalation reason text, but use `IssueList` for issues display
5. Remove the local `IssueCard` component (lines 52-91)

### Task 5 (P1): WaitingTaskDetail - Issue Progress Summary
**Dependencies:** None
**Atomic Commit:** `feat(detail-views): add issue progress bar to WaitingTaskDetail`
**Priority:** P1 (lower priority)

**File:** `src/components/tasks/detail-views/WaitingTaskDetail.tsx`

Lower priority - add `IssueProgressBar` to show resolution status while waiting for review.

## Verification

1. `npm run lint && npm run typecheck`
2. `npm run build`
3. Manual test in web mode:
   - Task in `review_passed` → verify HumanReviewTaskDetail shows structured issues
   - Task in `revision_needed` → verify RevisionTaskDetail shows issues with status grouping
   - Task in `re_executing` → verify ExecutionTaskDetail shows open issues to address
   - Task in `escalated` → verify EscalatedTaskDetail shows structured issues

## Dependencies

Tasks 1-4 are independent, can be done in parallel or sequence.
Task 5 is P1, lower priority.

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Compilation Unit Analysis

All tasks in this plan are **additive changes** - they add imports, add queries, and replace JSX elements. No renames, signature changes, or removals that would break compilation.

| Task | Files Modified | Compilation Unit | Status |
|------|----------------|------------------|--------|
| 1 | HumanReviewTaskDetail.tsx | Self-contained | ✅ Valid |
| 2 | RevisionTaskDetail.tsx | Self-contained | ✅ Valid |
| 3 | ExecutionTaskDetail.tsx | Self-contained | ✅ Valid |
| 4 | EscalatedTaskDetail.tsx | Self-contained | ✅ Valid |
| 5 | WaitingTaskDetail.tsx | Self-contained | ✅ Valid |

### Parallel Execution

Tasks 1-4 can be executed in parallel by multiple agents since they modify different files with no interdependencies.
