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

- [ ] ~~Implement TODO: Pause execution - src/components/tasks/TaskFullView.tsx:221~~ (PRD:21:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Stop execution - src/components/tasks/TaskFullView.tsx:225~~ (PRD:21:1:1:1:1:1)
- [ ] ~~Implement TODO: File change handling in useEvents - src/hooks/useEvents.ts:88~~ (PRD:1:1:1:1:1)

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

- [x] [P2] [Backend] Type safety: Use `format!("{:?}", ...)` for enum serialization instead of proper variants - src-tauri/src/http_server/types.rs:81
- [x] [P2] [Backend] Inconsistent error handling: Direct `.map_err(|_| StatusCode::...)` swallows error details; use tracing like in steps.rs - src-tauri/src/http_server/handlers/ideation.rs:58
- [x] [P2] [Backend] Inconsistent error handling: Missing error logging in artifact handlers despite having tracing in steps.rs - src-tauri/src/http_server/handlers/artifacts.rs:31
- [x] [P2] [Backend] Inconsistent error handling: Missing error logging in task handlers despite error pattern established elsewhere - src-tauri/src/http_server/handlers/tasks.rs:42

### P3 - Low Impact

- [x] [P3] [Backend] Dead code: Unused variable `_rx` never referenced - src-tauri/src/http_server/handlers/permissions.rs:18
- [ ] ~~[P3] [Backend] Dead code: Unused variable `_session_link` never referenced~~ (stale:1 - variable already removed)
- [ ] ~~[P3] [Backend] Redundant clone: Using `.clone()` in serde_json::json! macro (owned values unnecessary)~~ (stale:1 - already using &response)
- [x] [P3] [Backend] Redundant clone: Unnecessary `response.clone()` when value is already moved - src-tauri/src/http_server/handlers/steps.rs:304
- [x] [P3] [Backend] Redundant clones: Excessive `.clone()` calls in request parameters - src-tauri/src/http_server/handlers/permissions.rs:22

---

## REFILL (Added 2026-01-29 22:50)

### P2 - Medium Impact

- [ ] ~~[P2] [Frontend] Error handling: Empty catch blocks without error logging - src/App.tsx:241~~ (stale - already has toast.error)
- [ ] ~~[P2] [Frontend] Error handling: Silent catch block in pause/stop handlers - src/App.tsx:257~~ (stale - already has toast.error)
- [x] [P2] [Frontend] Type safety: Unused parameter with underscore prefix - src/App.tsx:264
- [x] [P2] [Backend] Naming: non_snake_case suppressions in command handlers - src-tauri/src/commands/task_commands/mutation.rs:71
- [x] [P2] [Frontend] Lint suppression: eslint-disable-next-line for hook dependencies - src/hooks/useTaskExecutionState.ts:141
- [x] [P2] [Frontend] Dead code: Prepared variables marked with @ts-expect-error never used - src/components/ExtensibilityView.ResearchPanel.tsx:38

### P3 - Low Impact

- [ ] [P3] [Frontend] Console.error cleanup: Multiple console.error calls in error handlers - src/App.tsx:290
- [ ] [P3] [Frontend] Console.error cleanup: Error logging in chat handlers - src/hooks/useIntegratedChatHandlers.ts:98
- [ ] [P3] [Frontend] Console.error cleanup: Multiple console.error in event hooks - src/hooks/useSupervisorAlerts.listener.ts:44
- [ ] [P3] [Frontend] Code organization: Large component needing extraction - src/components/tasks/TaskDetailModal.tsx:690
- [ ] [P3] [Backend] Dead code marker: Query functions marked #[allow(dead_code)] - src-tauri/src/infrastructure/sqlite/sqlite_task_repo/queries.rs:20
- [ ] [P3] [Frontend] Commented example code: console.log example in docs - src/hooks/useFileDrop.ts:74

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 12 (9 excluded, 3 deferred to PRD)
**Completed:** 10
**Validated:** 75 strikethroughs (2026-01-29 x60) - 22 archived, 1 reactivated (moved to refactor as P1), 60 incremented
**Last maintenance:** 2026-01-29 02:05 (archived 1 item, validated 3 strikethroughs: 1 archived, 2 incremented)
