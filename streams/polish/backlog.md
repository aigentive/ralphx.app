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
- [x] Remove console.log statements in TaskChatPanel event listeners - src/components/tasks/TaskChatPanel.tsx:351,366,390,405

### Frontend (PRD-deferred)

- [ ] ~~Implement TODO: Open diff viewer - src/App.tsx:784~~ (PRD:20:1:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Edit task modal - src/components/tasks/TaskFullView.tsx:213~~ (PRD:18:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Archive task - src/components/tasks/TaskFullView.tsx:217~~ (PRD:18:1:1:1)
- [ ] ~~Implement TODO: Pause execution - src/components/tasks/TaskFullView.tsx (line ~130)~~ (PRD:21:1)
- [ ] ~~Implement TODO: Stop execution - src/components/tasks/TaskFullView.tsx (line ~140)~~ (PRD:21:1)
- [ ] Implement TODO: File change handling in useEvents - src/hooks/useEvents.ts:88 (was line ~50)

### Backend

_No active P3 items. Completed items moved to archive._

## REFILL (Added 2026-01-28)

### P2 - Medium Impact

- [ ] [P2] [Backend] Replace panic! with proper error handling - src-tauri/src/infrastructure/agents/claude/stream_processor.rs:432
- [ ] [P2] [Backend] Replace .unwrap() calls with error handling - src-tauri/src/error.rs:95
- [ ] [P2] [Backend] Replace .unwrap() calls with error handling - src-tauri/src/commands/artifact_commands.rs:452
- [ ] [P2] [Backend] Replace .unwrap() calls with error handling - src-tauri/src/commands/review_commands.rs:375
- [x] [P2] [Backend] Replace .expect() with error handling - src-tauri/src/http_server.rs:395
- [x] [P2] [Frontend] Type safety: Replace z.any() with specific type - src/types/task-context.ts:56
- [ ] [P2] [Frontend] Type safety: Replace z.unknown() with proper types - src/api/chat.ts:115
- [ ] [P2] [Frontend] Refactor large API file (821 LOC) - extract helpers - src/api/ideation.ts:1

### P3 - Low Impact

- [x] [P3] [Frontend] Remove console.log from IntegratedChatPanel debug statement - src/components/Chat/IntegratedChatPanel.tsx:124
- [x] [P3] [Frontend] Remove console.log statements from production code - src/components/Chat/ChatPanel.tsx:414,440,482,533,593,613
- [x] [P3] [Frontend] Remove console.log stub from event handler - src/components/tasks/detail-views/CompletedTaskDetail.tsx:258
- [x] [P3] [Frontend] Remove console.log stub from inline handler - src/components/Ideation/IdeationView.tsx:336
- [x] [P3] [Frontend] Remove console.log statement - src/components/Ideation/useIdeationHandlers.ts:74
- [x] [P3] [Frontend] Remove console.debug statements from production code - src/hooks/useIntegratedChatHandlers.ts:97,132,172
- [ ] ~~[P3] [Frontend] Remove console.debug statements from production code~~ (stale:1 - no console.debug present, only console.error at line 208 which is appropriate)
- [x] [P3] [Frontend] Remove console.log statements from production code - src/hooks/useIntegratedChatEvents.ts:71,119
- [x] [P3] [Frontend] Fast refresh warning - Extract constant from component export - src/components/Chat/ResizeablePanel.tsx:52
- [ ] [P3] [Frontend] Fast refresh warning - Extract badgeVariants constant to separate file - src/components/ui/badge.tsx:6
- [ ] [P3] [Frontend] Fast refresh warning - Extract buttonVariants constant to separate file - src/components/ui/button.tsx:7
- [ ] [P3] [Frontend] Fast refresh warning - Extract toggleVariants constant to separate file - src/components/ui/toggle.tsx:7
- [x] [P3] [Frontend] Replace promise chain .then() with async/await - src/hooks/useStepEvents.ts:82
- [ ] [P3] [Frontend] Replace promise chain .then() with async/await - src/hooks/useSupervisorAlerts.listener.ts:100
- [ ] [P3] [Frontend] Error handling: Check empty catch blocks - src/components/Chat/ChatPanel.tsx:342

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 23 (3 excluded, 9 deferred to PRD)
**Completed:** 10
**Validated:** 22 strikethroughs (2026-01-28 x7) - 5 archived, 1 reactivated, 16 incremented
