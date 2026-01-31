# Review History Widget Unification Analysis

## Summary

**Question:** Should we unify the review history widgets in EscalatedTaskDetail and CompletedTaskDetail?

**Answer: Yes, partially.** Extract a shared `ReviewTimeline` component from CompletedTaskDetail for use in both state-specific views. Keep `StateHistoryTimeline` separate as it serves the generic task views (Modal/Panel/View).

**Rationale:**
| Factor | Recommendation |
|--------|---------------|
| Code quality | Unify - reduces duplication from 3 to 2 implementations |
| Maintainability | Unify - single source of truth for state-specific views |
| UX | Keep filter prop - Escalated shows only "changes_requested", Completed shows all |
| StateHistoryTimeline | Keep separate - serves different context (generic views, self-fetching) |

**Effort:** Small (~50-100 LOC moved, props added)

---

## Current State: 3 Implementations

### 1. EscalatedTaskDetail - `PreviousAttemptsSection`
**File:** `src/components/tasks/detail-views/EscalatedTaskDetail.tsx:195-234`

```tsx
// Filters to changes_requested only, numbered attempts, truncated notes
function PreviousAttemptsSection({ history }: { history: ReviewNoteResponse[] }) {
  const changesRequestedEntries = history.filter(e => e.outcome === "changes_requested");
  // Shows: RotateCcw icon + "Attempt #N: Changes requested" + truncated notes
}
```

**UX Focus:** "What went wrong" - shows iteration cycles before escalation

### 2. CompletedTaskDetail - `ReviewHistoryTimeline`
**File:** `src/components/tasks/detail-views/CompletedTaskDetail.tsx:91-202`

```tsx
// Full timeline with vertical lines, shows ALL entries
function HistoryTimelineItem({ entry, isLast }) {
  // Shows: Colored dot + line connector + reviewer icon + outcome label + full notes
}
function ReviewHistoryTimeline({ history }) {
  // Renders all entries in timeline format
}
```

**UX Focus:** "Complete journey" - shows how task reached completion

### 3. StateHistoryTimeline (Standalone)
**File:** `src/components/tasks/StateHistoryTimeline.tsx` (167 lines)

```tsx
// Self-contained, fetches own data, premium ring effects
function StateHistoryTimeline({ taskId }: { taskId: string }) {
  const { data, isLoading, isEmpty } = useTaskStateHistory(taskId);
  // Shows: Ring effect dots + quoted notes + "by: Human/AI Reviewer"
}
```

**UX Focus:** Generic history display with premium styling

---

## Comparison Matrix

| Aspect | PreviousAttempts | ReviewHistoryTimeline | StateHistoryTimeline |
|--------|------------------|----------------------|----------------------|
| Data scope | `changes_requested` only | ALL entries | ALL entries |
| Data source | Prop (parent fetches) | Prop (parent fetches) | Self-fetches via taskId |
| Visual style | Simple list | Vertical timeline + lines | Timeline + ring effects |
| Notes | Truncated | Full | Full (italic, quoted) |
| Actor display | Implicit | Icon + label | "by: X Reviewer" |
| Connector lines | None | Yes (rgba white) | Yes (border-subtle) |
| Lines of code | 40 | 112 | 167 |

---

## Why Both Exist (Contextual UX)

**EscalatedTaskDetail:**
- User needs to decide: approve or request changes
- Focus on **iteration history** (what was tried before)
- Filtering to `changes_requested` removes noise
- Numbered attempts (`#1`, `#2`) show iteration count clearly

**CompletedTaskDetail:**
- User is reviewing a finished task
- Focus on **complete audit trail**
- All entries matter for understanding the journey
- Timeline visualization shows temporal flow

---

## Recommendation: Unify with Filter Pattern

**Extract a shared `ReviewTimeline` component** with configurable filtering.

### Why Unify:
1. **DRY** - Timeline rendering logic duplicated 3 times
2. **Bug surface** - Fix in one place, not three
3. **Consistency** - Same visual language across views
4. **Maintainability** - Single component to test and update

### Why Keep Contextual Filtering:
1. **UX appropriate** - Escalated view needs filtered data
2. **Mental model** - Different contexts, different needs
3. **Flexibility** - Filter prop allows any use case

### Proposed Interface:
```tsx
interface ReviewTimelineProps {
  history: ReviewNoteResponse[];
  filter?: (entry: ReviewNoteResponse) => boolean;
  emptyMessage?: string;
  showAttemptNumbers?: boolean;  // For escalated view's "#1, #2" pattern
}
```

### Usage Examples:
```tsx
// CompletedTaskDetail - show all
<ReviewTimeline history={history} />

// EscalatedTaskDetail - show previous attempts only
<ReviewTimeline
  history={history}
  filter={(e) => e.outcome === "changes_requested"}
  showAttemptNumbers
  emptyMessage="No previous attempts"
/>
```

---

## Which Style to Use as Base

**Choose: CompletedTaskDetail's `ReviewHistoryTimeline`**

| Criteria | PreviousAttempts | ReviewHistoryTimeline | StateHistoryTimeline |
|----------|------------------|----------------------|----------------------|
| Visual polish | Basic list | Good timeline | Best (ring effects) |
| Match detail views | No | Yes | No (different tokens) |
| Flexibility | Limited | Good | Self-contained |
| LOC (simpler better) | 40 | 112 | 167 |

**Reasoning:**
1. `ReviewHistoryTimeline` already in CompletedTaskDetail - minimal change
2. Visual style consistent with other detail view sections
3. Uses CSS variables that match (`rgba(255,255,255,0.x)` pattern)
4. `StateHistoryTimeline` uses different tokens (`var(--bg-surface)`, ring effects) that don't match detail view aesthetic
5. Medium complexity - not too simple (PreviousAttempts), not too complex (StateHistoryTimeline)

---

## Implementation Plan

### Task 1: Extract ReviewTimeline to shared location (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `refactor(detail-views): extract ReviewTimeline to shared`

1. Move `HistoryTimelineItem` and `ReviewHistoryTimeline` to `src/components/tasks/detail-views/shared/ReviewTimeline.tsx`
2. Add props: `filter`, `emptyMessage`, `showAttemptNumbers`
3. Export from `src/components/tasks/detail-views/shared/index.ts`

### Task 2: Update CompletedTaskDetail
**Dependencies:** Task 1
**Atomic Commit:** `refactor(detail-views): use shared ReviewTimeline in CompletedTaskDetail`

1. Import from shared
2. No functional change (passes no filter)

### Task 3: Update EscalatedTaskDetail
**Dependencies:** Task 1
**Atomic Commit:** `refactor(detail-views): use shared ReviewTimeline in EscalatedTaskDetail`

1. Replace `PreviousAttemptsSection` with `ReviewTimeline`
2. Pass filter: `(e) => e.outcome === "changes_requested"`
3. Pass `showAttemptNumbers={true}`

### Task 4: StateHistoryTimeline - Keep Separate (NO CODE CHANGES)
**Dependencies:** None
**Atomic Commit:** N/A - review only

**Usage found:**
- `TaskDetailModal.tsx:448` - Generic task modal
- `TaskDetailPanel.tsx:444` - Generic task panel
- `TaskDetailView.tsx:190` - Generic task view

**Key insight:** Two-tier architecture exists:
1. **Generic task views** (TaskDetailModal/Panel/View) → use `StateHistoryTimeline` for ALL tasks
2. **State-specific views** (CompletedTaskDetail, EscalatedTaskDetail) → show when task is in specific state

**Recommendation: Keep `StateHistoryTimeline` separate.**

Reasons:
- Different context: Generic views show history for ANY task state
- Self-fetching: Takes `taskId`, fetches own data (good for generic context)
- Premium styling: Ring effects suit the modal/panel context
- Well-tested: 338 lines of tests

The unification should only affect the state-specific detail views (Completed/Escalated), not the generic task views.

---

## Files to Modify

| File | Action |
|------|--------|
| `src/components/tasks/detail-views/shared/ReviewTimeline.tsx` | CREATE - extracted component |
| `src/components/tasks/detail-views/shared/index.ts` | EDIT - add export |
| `src/components/tasks/detail-views/CompletedTaskDetail.tsx` | EDIT - import from shared |
| `src/components/tasks/detail-views/EscalatedTaskDetail.tsx` | EDIT - use ReviewTimeline, remove PreviousAttemptsSection |
| `src/components/tasks/StateHistoryTimeline.tsx` | REVIEW - check usage, possibly deprecate |

---

## Verification

1. Visual comparison: Screenshot both views before/after
2. Test: Escalated view shows only `changes_requested` entries
3. Test: Completed view shows all entries
4. Test: Empty state displays correctly in both
5. Test: Notes display correctly (not truncated in completed, appropriate in escalated)

---

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Task Dependency Graph

```
Task 1 (Extract) ──┬──► Task 2 (Update CompletedTaskDetail)
                   │
                   └──► Task 3 (Update EscalatedTaskDetail)

Task 4 (Review only - no code changes)
```

Tasks 2 and 3 can run in parallel after Task 1 completes.
