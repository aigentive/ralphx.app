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

_No active P3 items. Completed items moved to archive._

### Frontend (PRD-deferred)

- [ ] ~~Implement TODO: Open diff viewer - src/App.tsx:784~~ (PRD:20:1:1:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Edit task modal - src/components/tasks/TaskFullView.tsx:213~~ (PRD:18:1:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Archive task - src/components/tasks/TaskFullView.tsx:217~~ (PRD:18:1:1:1:1)
- [ ] ~~Implement TODO: Pause execution - src/components/tasks/TaskFullView.tsx (line ~130)~~ (PRD:21:1)
- [ ] ~~Implement TODO: Stop execution - src/components/tasks/TaskFullView.tsx (line ~140)~~ (PRD:21:1)
- [ ] Implement TODO: File change handling in useEvents - src/hooks/useEvents.ts:88 (was line ~50)

### Backend

_No active P3 items. Completed items moved to archive._

## REFILL (Added 2026-01-28)

### P2 - Medium Impact

- [x] [P2] [Backend] Replace panic! with proper error handling - src-tauri/src/infrastructure/agents/claude/stream_processor.rs:432
- [x] [P2] [Backend] Replace .unwrap() calls with error handling - src-tauri/src/error.rs:95
- [ ] [P2] [Backend] Replace .unwrap() calls with error handling - src-tauri/src/commands/artifact_commands.rs:452
- [ ] [P2] [Backend] Replace .unwrap() calls with error handling - src-tauri/src/commands/review_commands.rs:375
- [ ] [P2] [Frontend] Type safety: Replace z.unknown() with proper types - src/api/chat.ts:115
- [ ] [P2] [Frontend] Refactor large API file (821 LOC) - extract helpers - src/api/ideation.ts:1

### P3 - Low Impact

- [ ] [P3] [Frontend] Fast refresh warning - Extract badgeVariants constant to separate file - src/components/ui/badge.tsx:6
- [ ] [P3] [Frontend] Fast refresh warning - Extract buttonVariants constant to separate file - src/components/ui/button.tsx:7
- [ ] [P3] [Frontend] Fast refresh warning - Extract toggleVariants constant to separate file - src/components/ui/toggle.tsx:7
- [ ] [P3] [Frontend] Replace promise chain .then() with async/await - src/hooks/useSupervisorAlerts.listener.ts:100
- [ ] [P3] [Frontend] Error handling: Check empty catch blocks - src/components/Chat/ChatPanel.tsx:342

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 17 (3 excluded, 9 deferred to PRD)
**Completed:** 2
**Validated:** 26 strikethroughs (2026-01-28 x11) - 6 archived, 1 reactivated, 19 incremented
**Last maintenance:** 2026-01-28 (validated 3 strikethroughs, incremented 3 counters)
