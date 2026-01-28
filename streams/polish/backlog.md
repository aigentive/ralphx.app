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
- [x] [P2] [Frontend] Unused parameter: defaultStatus in TaskCreationForm prop defaults to undefined - src/components/tasks/TaskCreationForm.tsx:59

### P3 - Low Impact

- [x] [P3] [Frontend] Remove console.warn from App.tsx global shortcut registration - src/App.tsx:283
- [x] [P3] [Frontend] Remove eslint-disable comments from useChat.test.ts (6 occurrences) - properly typed zustand mock - src/hooks/useChat.test.ts:8,29,37,42,58,103

## P3 - Low Impact

### Frontend

- [x] Remove debug console.log from agent:run_started handler - src/hooks/useChat.ts:321
- [x] Remove console.log statements in TaskChatPanel event listeners - src/components/tasks/TaskChatPanel.tsx:351,366,390,405

### Frontend (PRD-deferred)

- [ ] ~~Implement TODO: Open diff viewer - src/App.tsx (line ~420)~~ (PRD:20:1:1:1)
- [ ] ~~Implement TODO: Edit task modal - src/components/tasks/TaskFullView.tsx (line ~100)~~ (PRD:18:1:1:1)
- [ ] ~~Implement TODO: Archive task - src/components/tasks/TaskFullView.tsx (line ~120)~~ (PRD:18:1:1)
- [ ] ~~Implement TODO: Pause execution - src/components/tasks/TaskFullView.tsx (line ~130)~~ (PRD:21:1)
- [ ] ~~Implement TODO: Stop execution - src/components/tasks/TaskFullView.tsx (line ~140)~~ (PRD:21:1)
- [ ] Implement TODO: File change handling in useEvents - src/hooks/useEvents.ts:88 (was line ~50)

### Backend

_No active P3 items. Completed items moved to archive._

## REFILL (Added 2026-01-28)

### P2 - Medium Impact

_No active P2 items._

### P3 - Low Impact

- [x] [P3] [Frontend] Remove console.log from IntegratedChatPanel debug statement - src/components/Chat/IntegratedChatPanel.tsx:124
- [x] [P3] [Frontend] Remove console.log statements from production code - src/components/Chat/ChatPanel.tsx:414,440,482,533,593,613
- [x] [P3] [Frontend] Remove console.log stub from event handler - src/components/tasks/detail-views/CompletedTaskDetail.tsx:258
- [ ] ~~[P3] [Frontend] Remove console.log stub from event handler~~ (stale - no console.log present at line 263)
- [x] [P3] [Frontend] Remove console.log stub from inline handler - src/components/Ideation/IdeationView.tsx:336
- [x] [P3] [Frontend] Remove console.log statement - src/components/Ideation/useIdeationHandlers.ts:74
- [ ] [P3] [Frontend] Remove console.debug statements from production code - src/hooks/useIntegratedChatHandlers.ts:97,132,172
- [ ] [P3] [Frontend] Remove console.debug statements from production code - src/hooks/useAgentEvents.ts:123,159,186
- [ ] [P3] [Frontend] Remove console.log statements from production code - src/hooks/useIntegratedChatEvents.ts:71,119
- [ ] [P3] [Frontend] Fast refresh warning - Extract constant from component export - src/components/Chat/ResizeablePanel.tsx:52
- [ ] [P3] [Frontend] Fast refresh warning - Extract badgeVariants constant to separate file - src/components/ui/badge.tsx:6
- [ ] [P3] [Frontend] Fast refresh warning - Extract buttonVariants constant to separate file - src/components/ui/button.tsx:7
- [ ] [P3] [Frontend] Fast refresh warning - Extract toggleVariants constant to separate file - src/components/ui/toggle.tsx:7

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 12 (3 excluded, 9 deferred to PRD)
**Completed:** 7
**Validated:** 11 strikethroughs (2026-01-28) - 4 archived, 1 reactivated, 6 incremented
