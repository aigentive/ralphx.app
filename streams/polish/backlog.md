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

## P3 - Low Impact

### Frontend

- [x] Remove debug console.log from agent:run_started handler - src/hooks/useChat.ts:321

### Frontend

- [ ] ~~Implement TODO: Call Tauri command for answer submission - src/App.tsx (line ~200)~~ (PRD:20)
- [ ] ~~Implement TODO: Approve review modal - src/App.tsx (line ~400)~~ (PRD:20)
- [ ] ~~Implement TODO: Request changes modal - src/App.tsx (line ~410)~~ (PRD:20)
- [ ] ~~Implement TODO: Open diff viewer - src/App.tsx (line ~420)~~ (PRD:20)
- [ ] ~~Implement TODO: Edit task modal - src/components/tasks/TaskFullView.tsx (line ~100)~~ (PRD:18)
- [ ] ~~Implement TODO: Archive task - src/components/tasks/TaskFullView.tsx (line ~120)~~ (PRD:18)
- [ ] ~~Implement TODO: Pause execution - src/components/tasks/TaskFullView.tsx (line ~130)~~ (PRD:21)
- [ ] ~~Implement TODO: Stop execution - src/components/tasks/TaskFullView.tsx (line ~140)~~ (PRD:21)
- [ ] ~~Implement TODO: File change handling in useEvents - src/hooks/useEvents.ts (line ~50)~~ (PRD:19)

### Backend

_No active P3 items. Completed items moved to archive._

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 12 (3 excluded, 9 deferred to PRD)
**Completed:** 31 (moved to archive)
