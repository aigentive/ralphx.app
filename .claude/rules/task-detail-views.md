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
| `failed` | BasicTaskDetail | Execution failed |
| `cancelled` | BasicTaskDetail | Task cancelled |

## File Locations

| Component | Path |
|-----------|------|
| Registry definition | `src/components/tasks/TaskDetailPanel.tsx:74-100` |
| View selection logic | `src/components/tasks/TaskDetailPanel.tsx:308-315` |
| Entry point (Kanban) | `src/components/tasks/TaskDetailOverlay.tsx:628` |
| View components | `src/components/tasks/detail-views/*.tsx` |

## View Components (8 total)

| Component | States Handled | Key Features |
|-----------|----------------|--------------|
| BasicTaskDetail | backlog, ready, blocked, qa_*, failed, cancelled | Steps list, description |
| ExecutionTaskDetail | executing, re_executing | Live progress, step tracking, revision feedback |
| WaitingTaskDetail | pending_review | Work summary, completion stats |
| ReviewingTaskDetail | reviewing | AI review progress, step indicators |
| HumanReviewTaskDetail | review_passed | AI summary, approve/reject actions |
| EscalatedTaskDetail | escalated | Escalation reason, human decision buttons |
| RevisionTaskDetail | revision_needed | Review feedback, parsed issues, attempt count |
| CompletedTaskDetail | approved | Approval details, review history, diff viewer |

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
