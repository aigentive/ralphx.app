# Polish Backlog (P2/P3 - Cleanup)

> P2 (Medium): Type safety, small extractions, error handling (50-150 LOC)
> P3 (Low): Lint fixes, naming, cleanup, dead code removal (<50 LOC)
> Exclusions: `src/components/ui/*` (shadcn/ui)

## P2 - Medium Impact

### Frontend

- [ ] ~~Extract constants from ui/badge - src/components/ui/badge.tsx:36~~ (excluded)
- [ ] ~~Extract constants from ui/button - src/components/ui/button.tsx:58~~ (excluded)
- [ ] ~~Extract constants from ui/toggle - src/components/ui/toggle.tsx:45~~ (excluded)

### Backend

_No active P2 items. Completed items moved to archive._

## NEW ITEMS (Added 2026-01-28)

### P2 - Medium Impact

- [x] [P2] [Frontend] Extract hook logic from useChat (528 LOC → 344 LOC) - extracted event handling to useAgentEvents - src/hooks/useChat.ts:1-528
- [x] [P2] [Frontend] Extract hook logic from useEvents (417 LOC → 102 LOC) - split by event type - src/hooks/useEvents.ts:1-417
- [x] [P2] [Frontend] Extract hook logic from useSupervisorAlerts (409 LOC → 184 LOC) - split alert management into store and listener modules - src/hooks/useSupervisorAlerts.ts:1-409
- [ ] ~~[P2] [Frontend] Error handling: console.error in useChat lacks structured error reporting~~ (stale:1 - removed during refactor)
- [ ] ~~[P2] [Frontend] Error handling: console.error in useEvents lacks structured error reporting~~ (stale:1 - removed during refactor)
- [x] [P2] [Frontend] Unused parameter: defaultStatus in TaskCreationForm prop defaults to undefined - src/components/tasks/TaskCreationForm.tsx:59

### P3 - Low Impact

- [ ] ~~[P3] [Frontend] Remove console.debug statements from useChat (agent event tracing) - src/hooks/useChat.ts:368,404,431~~ (stale - removed during refactor)
- [x] [P3] [Frontend] Remove console.warn from App.tsx global shortcut registration - src/App.tsx:283
- [ ] [P3] [Frontend] Remove eslint-disable comment from useTaskExecutionState exhaustive-deps - src/hooks/useTaskExecutionState.ts:141
- [ ] [P3] [Frontend] Remove eslint-disable comments from useChat.test.ts (6 occurrences) - src/hooks/useChat.test.ts:8,29,37,42,58,103

## P3 - Low Impact

### Frontend

- [x] Remove debug console.log from agent:run_started handler - src/hooks/useChat.ts:321
- [x] Remove console.log statements in TaskChatPanel event listeners - src/components/tasks/TaskChatPanel.tsx:351,366,390,405

### Frontend (PRD-deferred)

- [ ] ~~Implement TODO: Approve review modal - src/App.tsx (line ~400)~~ (PRD:20:1:1:1:1)
- [ ] ~~Implement TODO: Request changes modal - src/App.tsx (line ~410)~~ (PRD:20:1:1:1)
- [ ] ~~Implement TODO: Open diff viewer - src/App.tsx (line ~420)~~ (PRD:20:1)
- [ ] ~~Implement TODO: Edit task modal - src/components/tasks/TaskFullView.tsx (line ~100)~~ (PRD:18:1:1)
- [ ] ~~Implement TODO: Archive task - src/components/tasks/TaskFullView.tsx (line ~120)~~ (PRD:18:1)
- [ ] ~~Implement TODO: Pause execution - src/components/tasks/TaskFullView.tsx (line ~130)~~ (PRD:21:1)
- [ ] ~~Implement TODO: Stop execution - src/components/tasks/TaskFullView.tsx (line ~140)~~ (PRD:21:1)
- [ ] Implement TODO: File change handling in useEvents - src/hooks/useEvents.ts:88 (was line ~50)

### Backend

_No active P3 items. Completed items moved to archive._

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 12 (3 excluded, 9 deferred to PRD)
**Completed:** 31 (moved to archive)
