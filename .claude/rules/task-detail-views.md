# Task Detail Views Registry

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

## InternalStatus → View Mapping

| InternalStatus | View Component | Purpose |
|----------------|----------------|---------|
| `backlog` | BasicTaskDetail | Idle task in backlog |
| `ready` | BasicTaskDetail | Ready for execution |
| `blocked` | BasicTaskDetail | Waiting on dependency |
| `executing` | ExecutionTaskDetail | Live AI execution progress |
| `re_executing` | ExecutionTaskDetail | Re-execution after revision |
| `qa_refining` | BasicTaskDetail | QA refinement (no specialized view) |
| `qa_testing` | BasicTaskDetail | QA testing (no specialized view) |
| `qa_passed` | BasicTaskDetail | QA passed (no specialized view) |
| `qa_failed` | BasicTaskDetail | QA failed (no specialized view) |
| `pending_review` | WaitingTaskDetail | Work done, awaiting AI review |
| `reviewing` | ReviewingTaskDetail | AI review in progress |
| `review_passed` | HumanReviewTaskDetail | AI approved, human confirmation |
| `escalated` | EscalatedTaskDetail | AI escalated to human |
| `revision_needed` | RevisionTaskDetail | Changes requested |
| `approved` | CompletedTaskDetail | Task completed |
| `pending_merge` | MergingTaskDetail | Programmatic merge in progress |
| `merging` | MergingTaskDetail | Agent-assisted merge in progress |
| `merge_incomplete` | MergeIncompleteTaskDetail | Non-conflict merge failure, retry/resolve |
| `merge_conflict` | MergeConflictTaskDetail | Merge conflicts, manual resolution |
| `merged` | MergedTaskDetail | Successfully merged |
| `failed` | BasicTaskDetail | Execution failed |
| `cancelled` | BasicTaskDetail | Task cancelled |
| `paused` | BasicTaskDetail | Execution paused |
| `stopped` | BasicTaskDetail | Execution stopped |

## File Locations

| Component | Path |
|-----------|------|
| Registry definition | `src/components/tasks/TaskDetailPanel.tsx:74-100` |
| View selection logic | `src/components/tasks/TaskDetailPanel.tsx:308-315` |
| Entry point (Kanban) | `src/components/tasks/TaskDetailOverlay.tsx:628` |
| View components | `src/components/tasks/detail-views/*.tsx` |

## View Components (12 total)

| Component | States Handled | Key Features |
|-----------|----------------|--------------|
| BasicTaskDetail | backlog, ready, blocked, qa_*, failed, cancelled, paused, stopped | Steps list, description |
| ExecutionTaskDetail | executing, re_executing | Live progress, step tracking, revision feedback |
| WaitingTaskDetail | pending_review | Work summary, completion stats |
| ReviewingTaskDetail | reviewing | AI review progress, step indicators |
| HumanReviewTaskDetail | review_passed | AI summary, approve/reject actions |
| EscalatedTaskDetail | escalated | Escalation reason, human decision buttons |
| RevisionTaskDetail | revision_needed | Review feedback, parsed issues, attempt count |
| CompletedTaskDetail | approved | Approval details, review history, diff viewer |
| MergingTaskDetail | pending_merge, merging | Agent merge progress spinner |
| MergeConflictTaskDetail | merge_conflict | Conflict files, resolution steps, resolve button |
| MergeIncompleteTaskDetail | merge_incomplete | Error context, recovery steps, retry/resolve buttons |
| MergedTaskDetail | merged | Merge completion details |

## Wiring

```
TaskDetailOverlay (useViewRegistry={true})
  → TaskDetailPanel (viewAsStatus? for history)
    → TASK_DETAIL_VIEWS[status] ?? BasicTaskDetail
```

**Props:** `useViewRegistry` activates registry | `viewAsStatus` enables historical state viewing | Views receive `isHistorical` flag

## Adding New Views

1. Create `src/components/tasks/detail-views/NewStatusTaskDetail.tsx`
2. Implement `TaskDetailProps` interface: `{ task: Task; isHistorical?: boolean }`
3. Add to `TASK_DETAIL_VIEWS` map in `TaskDetailPanel.tsx`
