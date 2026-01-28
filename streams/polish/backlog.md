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

- [ ] ~~Implement TODO: Open diff viewer - src/App.tsx:794~~ (PRD:20:1:1:1:1:1:1:1:1:1:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Edit task modal - src/components/tasks/TaskFullView.tsx:213~~ (PRD:18:1:1:1:1:1:1:1:1:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Archive task - src/components/tasks/TaskFullView.tsx:217~~ (PRD:18:1:1:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Pause execution - src/components/tasks/TaskFullView.tsx:221~~ (PRD:21:1:1)
- [ ] ~~Implement TODO: Stop execution - src/components/tasks/TaskFullView.tsx:225~~ (PRD:21:1:1)
- [ ] ~~Implement TODO: File change handling in useEvents - src/hooks/useEvents.ts:88~~ (PRD:1:1)

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
- [x] [P3] [Frontend] Remove TODO comments for unimplemented task actions - src/components/tasks/TaskFullView.tsx:213
- [x] [P3] [Frontend] Remove TODO in HumanReviewTaskDetail - src/components/tasks/detail-views/HumanReviewTaskDetail.tsx:365
- [x] [P3] [Frontend] Clean up console.error in PlanEditor - src/components/Ideation/PlanEditor.tsx:215
- [x] [P3] [Backend] Remove TODO placeholder search implementation - src-tauri/src/commands/task_context_commands.rs:113
- [x] [P3] [Backend] Remove dead code suppression and clean - src-tauri/src/application/priority_service/tests.rs:11

---

## REFILL (Added 2026-01-29 00:43)

### P2 - Medium Impact

- [x] [P2] [Backend] Error handling: Replace `.unwrap()` with proper error handling - src-tauri/src/domain/supervisor/patterns.rs:146
- [x] [P2] [Backend] Error handling: Replace `.unwrap()` with proper error handling in serialization tests - src-tauri/src/domain/supervisor/patterns.rs:417
- [x] [P2] [Backend] Error handling: Replace `.unwrap()` calls in test assertions - src-tauri/src/domain/supervisor/patterns.rs:329
- [ ] ~~[P2] [Backend] Error handling: Replace `.expect()` calls with proper type-safe parsing - src-tauri/src/domain/entities/ideation.rs:171~~ (stale:1 - file no longer exists)
- [ ] ~~[P2] [Backend] Error handling: Replace `.parse().unwrap()` with Result handling - src-tauri/src/domain/entities/ideation.rs:1686~~ (stale:1 - file no longer exists)
- [x] [P2] [Backend] Error handling: Replace serde `.unwrap()` calls in tests - src-tauri/src/domain/supervisor/events.rs:361
- [x] [P2] [Backend] Error handling: Remove dead_code allow attribute - src-tauri/src/domain/agents/mod.rs:24
- [x] [P2] [Backend] Error handling: Remove dead_code allow attribute - src-tauri/src/application/dependency_service/mod.rs:12

### P3 - Low Impact

- [ ] [P3] [Backend] Code cleanup: Resolve TODO comment about database search optimization - src-tauri/src/http_server.rs:1296
- [ ] [P3] [Backend] Code cleanup: Resolve TODO comment about task dependencies - src-tauri/src/application/task_transition_service.rs:104
- [ ] [P3] [Backend] Code cleanup: Resolve TODO comment about ideation sessions - src-tauri/src/commands/test_data_commands.rs:206
- [ ] [P3] [Backend] Code cleanup: Resolve TODO comment about streaming implementation - src-tauri/src/infrastructure/agents/claude/claude_code_client.rs:249
- [ ] [P3] [Backend] Code cleanup: Resolve TODO comment about agent context - src-tauri/src/commands/task_commands/mutation.rs:353

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 6 (9 excluded, 10 deferred to PRD)
**Completed:** 10
**Validated:** 54 strikethroughs (2026-01-29 x39) - 10 archived, 1 reactivated (moved to refactor as P1), 43 incremented
**Last maintenance:** 2026-01-29 (archived 3 completed items, validated 3 strikethroughs → archived 3)
