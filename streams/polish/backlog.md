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

- [ ] ~~Implement TODO: Open diff viewer - src/App.tsx:784~~ (PRD:20:1:1:1:1:1:1:1:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Edit task modal - src/components/tasks/TaskFullView.tsx:213~~ (PRD:18:1:1:1:1:1:1:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Archive task - src/components/tasks/TaskFullView.tsx:217~~ (PRD:18:1:1:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Pause execution - src/components/tasks/TaskFullView.tsx:221~~ (PRD:21:1:1)
- [ ] ~~Implement TODO: Stop execution - src/components/tasks/TaskFullView.tsx:225~~ (PRD:21:1:1)
- [ ] ~~Implement TODO: File change handling in useEvents - src/hooks/useEvents.ts:88~~ (PRD:1:1)

### Backend

_No active P3 items. Completed items moved to archive._

## REFILL (Added 2026-01-28)

### P2 - Medium Impact

- [x] [P2] [Backend] Replace panic! with proper error handling - src-tauri/src/infrastructure/agents/claude/stream_processor.rs:432
- [x] [P2] [Backend] Replace .unwrap() calls with error handling - src-tauri/src/error.rs:95
- [x] [P2] [Backend] Replace .unwrap() calls with error handling - src-tauri/src/commands/artifact_commands.rs:452
- [x] [P2] [Backend] Replace .unwrap() calls with error handling - src-tauri/src/commands/review_commands.rs:375
- [x] [P2] [Frontend] Type safety: Replace z.unknown() with proper types - src/api/chat.ts:115
- [x] [P2] [Frontend] Refactor large API file (821 LOC → 473 LOC) - extracted schemas, transforms, types - src/api/ideation.ts:1

### P3 - Low Impact

- [ ] ~~[P3] [Frontend] Fast refresh warning - Extract badgeVariants constant to separate file - src/components/ui/badge.tsx:6~~ (excluded)
- [ ] ~~[P3] [Frontend] Fast refresh warning - Extract buttonVariants constant to separate file - src/components/ui/button.tsx:7~~ (excluded)
- [ ] ~~[P3] [Frontend] Fast refresh warning - Extract toggleVariants constant to separate file - src/components/ui/toggle.tsx:7~~ (excluded)
- [x] [P3] [Frontend] Replace promise chain .then() with async/await - src/hooks/useSupervisorAlerts.listener.ts:100
- [x] [P3] [Frontend] Error handling: Check empty catch blocks - src/components/Chat/ChatPanel.tsx:342

## REFILL (Added 2026-01-28 23:47)

### P2 - Medium Impact

- [x] [P2] [Frontend] Error handling: App.tsx catch blocks need proper user feedback via toast - src/App.tsx:330
- [x] [P2] [Frontend] Event listener cleanup: useResizePanel needs useEffect for document listener lifecycle - src/components/Chat/ResizeablePanel.tsx:63
- [x] [P2] [Frontend] Unnecessary useMemo: Multiple dependencies in ChatPanel could be optimized - src/components/Chat/ChatPanel.tsx:200

### P3 - Low Impact

- [ ] ~~[P3] [Frontend] Fast refresh warning: Extract constants from ResizeablePanel.tsx component export~~ (stale:1 - constants already extracted to ResizeablePanel.constants.ts)
- [ ] ~~[P3] [Frontend] Fast refresh warning: Extract badgeVariants from Badge component - src/components/ui/badge.tsx:30~~ (excluded)
- [ ] ~~[P3] [Frontend] Fast refresh warning: Extract buttonVariants from Button component - src/components/ui/button.tsx:44~~ (excluded)
- [ ] ~~[P3] [Frontend] Fast refresh warning: Extract toggleVariants from Toggle component - src/components/ui/toggle.tsx:29~~ (excluded)

## REFILL (Added 2026-01-29 00:00)

### P2 - Medium Impact

- [x] [P2] [Frontend] Extract ToolCallIndicator sub-functions - src/components/Chat/ToolCallIndicator.tsx:49-200
- [x] [P2] [Frontend] Consolidate ChatPanel console.error handlers into unified error handler - src/components/Chat/ChatPanel.tsx:332
- [x] [P2] [Frontend] Remove TODO comment for Tauri command integration - src/App.tsx:359
- [ ] [P2] [Frontend] Extract ExtensibilityView.panels large panel components (906 LOC) - src/components/ExtensibilityView.panels.tsx:1
- [ ] [P2] [Backend] Replace unwrap() calls with proper error handling in ideation_commands - src-tauri/src/commands/ideation_commands/mod.rs:45
- [ ] [P2] [Backend] Remove #[allow(dead_code)] suppression and verify actual usage - src-tauri/src/application/ideation_service/tests.rs:2

### P3 - Low Impact

- [ ] [P3] [Frontend] Remove TODO comment for diff viewer integration - src/components/tasks/detail-views/CompletedTaskDetail.tsx:257
- [ ] [P3] [Frontend] Remove TODO comments for unimplemented task actions - src/components/tasks/TaskFullView.tsx:213
- [ ] [P3] [Frontend] Remove TODO in HumanReviewTaskDetail - src/components/tasks/detail-views/HumanReviewTaskDetail.tsx:365
- [ ] [P3] [Frontend] Remove TODO in PlanTemplateSelector - src/components/Ideation/PlanTemplateSelector.tsx:94
- [ ] [P3] [Frontend] Clean up console.error in PlanEditor - src/components/Ideation/PlanEditor.tsx:215
- [ ] [P3] [Backend] Remove TODO placeholder search implementation - src-tauri/src/commands/task_context_commands.rs:113
- [ ] [P3] [Backend] Remove dead code suppression and clean - src-tauri/src/application/priority_service.rs:378
- [ ] [P3] [Backend] Clean up unused test fixtures in dependency_service - src-tauri/src/application/dependency_service.rs:530

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 17 (9 excluded, 10 deferred to PRD)
**Completed:** 9
**Validated:** 44 strikethroughs (2026-01-29 x29) - 6 archived, 1 reactivated, 37 incremented
**Last maintenance:** 2026-01-29 00:00 (refilled 15 P2/P3 items)
