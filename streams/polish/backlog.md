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

_Completed validations moved to archive._

### Backend

_No active P3 items. Completed items moved to archive._

## REFILL (Added 2026-01-28)

### P2 - Medium Impact

_No active P2 items. Completed items moved to archive._

### P3 - Low Impact

- [ ] ~~[P3] [Frontend] Fast refresh warning - Extract badgeVariants constant to separate file - src/components/ui/badge.tsx:6~~ (excluded)
- [ ] ~~[P3] [Frontend] Fast refresh warning - Extract buttonVariants constant to separate file - src/components/ui/button.tsx:7~~ (excluded)
- [ ] ~~[P3] [Frontend] Fast refresh warning - Extract toggleVariants constant to separate file - src/components/ui/toggle.tsx:7~~ (excluded)

## REFILL (Added 2026-01-28 23:47)

### P2 - Medium Impact

_No active P2 items. Completed items moved to archive._

### P3 - Low Impact

- [ ] ~~[P3] [Frontend] Fast refresh warning: Extract badgeVariants from Badge component - src/components/ui/badge.tsx:30~~ (excluded)
- [ ] ~~[P3] [Frontend] Fast refresh warning: Extract buttonVariants from Button component - src/components/ui/button.tsx:44~~ (excluded)
- [ ] ~~[P3] [Frontend] Fast refresh warning: Extract toggleVariants from Toggle component - src/components/ui/toggle.tsx:29~~ (excluded)

## REFILL (Added 2026-01-29 00:00)

### P2 - Medium Impact

_No active P2 items. Completed items moved to archive._

### P3 - Low Impact

_No active P3 items. Completed items moved to archive._

---

## REFILL (Added 2026-01-29 00:43)

### P2 - Medium Impact

_No active P2 items. Completed items moved to archive._

### P3 - Low Impact

_Completed items moved to archive._

---

## REFILL (Added 2026-01-29 18:30)

### P2 - Medium Impact

_No active P2 items. Completed items moved to archive._

### P3 - Low Impact

_Completed items moved to archive._

---

## REFILL (Added 2026-01-29 20:30)

### P2 - Medium Impact

### P3 - Low Impact

_No active P3 items. Completed items moved to archive._

---

## REFILL (Added 2026-01-29 20:43)

### P2 - Medium Impact

_Completed items moved to archive._

### P3 - Low Impact

_Completed items moved to archive._

---

## REFILL (Added 2026-01-29 22:50)

### P2 - Medium Impact

- [x] [P2] [Frontend] Type safety: Unused parameter with underscore prefix - src/App.tsx:264
- [x] [P2] [Backend] Naming: non_snake_case suppressions in command handlers - src-tauri/src/commands/task_commands/mutation.rs:71
- [x] [P2] [Frontend] Lint suppression: eslint-disable-next-line for hook dependencies - src/hooks/useTaskExecutionState.ts:141
- [x] [P2] [Frontend] Dead code: Prepared variables marked with @ts-expect-error never used - src/components/ExtensibilityView.ResearchPanel.tsx:38

### P3 - Low Impact

- [x] [P3] [Frontend] Console.error cleanup: Multiple console.error calls in error handlers - src/App.tsx:290
- [x] [P3] [Frontend] Console.error cleanup: Error logging in chat handlers - src/hooks/useIntegratedChatHandlers.ts:98
- [x] [P3] [Frontend] Console.error cleanup: Multiple console.error in event hooks - src/hooks/useSupervisorAlerts.listener.ts:44
- [x] [P3] [Frontend] Commented example code: console.log example in docs - src/hooks/useFileDrop.ts:74

---

## REFILL (Added 2026-01-29 23:15)

### P2 - Medium Impact

- [x] [P2] [Frontend] Extract constants from ResizeablePanel.tsx - `useResizePanel` hook exported from component file violates react-refresh rule - src/components/Chat/ResizeablePanel.tsx:50
- [x] [P2] [Backend] Replace panic! with Result in supervisor events - Test assertions use panic! for pattern matching - src-tauri/src/domain/supervisor/events.rs:304

### P3 - Low Impact

- [x] [P3] [Frontend] Remove console.error calls from useIntegratedChatHandlers - Error logging without proper error handling context - src/hooks/useIntegratedChatHandlers.ts:131
- [x] [P3] [Frontend] Remove console.error from useEvents.task - Debug logging left in event handler - src/hooks/useEvents.task.ts:39
- [x] [P3] [Frontend] Remove console.error from useAgentEvents - Debug logging in agent event handler - src/hooks/useAgentEvents.ts:208
- [ ] ~~[P3] [Backend] State machine file exceeds recommended size - machine.rs at 1114 LOC, consider extracting transition helpers~~ (stale:1 - file refactored into module)
- [ ] ~~[P3] [Backend] Remove test panics in supervisor actions - Unwrap calls in test serialization~~ (stale:1 - unwrap in tests is acceptable)

---

## REFILL (Added 2026-01-30)

### P2 - Medium Impact

- [ ] ~~[P2] [Backend] Error handling: Replace unwrap() in test setup~~ (stale - unwrap in test assertions is acceptable)
- [ ] ~~[P2] [Backend] Error handling: Multiple unwrap() calls in test helpers~~ (stale - unwrap in test helpers is acceptable)
- [ ] [P2] [Frontend] Type safety: Replace `any` type with proper DTO type - src/App.tsx:378
- [ ] [P2] [Backend] Debug output cleanup: Remove eprintln! statements - src-tauri/src/application/task_transition_service.rs:429,432
- [ ] [P2] [Backend] Debug output cleanup: Remove eprintln! logging - src-tauri/src/application/chat_service/chat_service_context.rs:127
- [ ] [P2] [Backend] Debug output cleanup: Remove eprintln! in startup - src-tauri/src/application/startup_jobs.rs:multiple

### P3 - Low Impact

- [ ] [P3] [Frontend] Dead code: TODO comment (deferred feature) - src/App.tsx:column group diff viewer task
- [ ] [P3] [Frontend] Dead code: TODO comment - src/components/tasks/detail-views/HumanReviewTaskDetail.tsx:review modal task
- [ ] [P3] [Frontend] Dead code: TODO comment - src/components/ExtensibilityView.ResearchPanel.tsx:command implementation
- [ ] [P3] [Frontend] Dead code: TODO comment - src/hooks/useEvents.ts:file change handling
- [ ] [P3] [Frontend] File size: TaskDetailOverlay exceeds 500 LOC limit (604 LOC) - src/components/tasks/TaskDetailOverlay.tsx:entire file
- [ ] [P3] [Frontend] File size: TaskDetailPanel near limit (446 LOC) - src/components/tasks/TaskDetailPanel.tsx:entire file
- [ ] [P3] [Backend] File size: SqliteArtifactRepository exceeds 500 LOC (1137 LOC) - src-tauri/src/infrastructure/sqlite/sqlite_artifact_repo.rs:extract helpers
- [ ] [P3] [Backend] File size: SqliteProposalDependencyRepository exceeds 500 LOC (1078 LOC) - src-tauri/src/infrastructure/sqlite/sqlite_proposal_dependency_repo.rs:extract helpers

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 19 (9 excluded, 0 deferred to PRD)
**Completed:** 10
**Validated:** 90 strikethroughs (2026-01-29 x60, 2026-01-30 x15) - 32 archived, 1 reactivated (moved to refactor as P1), 69 incremented
**Last maintenance:** 2026-01-30 (refilled polish backlog with 14 items, validated 3 strikethroughs, archived 2)
