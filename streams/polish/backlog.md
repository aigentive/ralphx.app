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

- [ ] ~~Implement TODO: Open diff viewer - src/App.tsx:791~~ (PRD:20:1:1:1:1:1:1:1:1:1:1:1:1:1:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Edit task modal - src/components/tasks/TaskFullView.tsx:213~~ (PRD:18:1:1:1:1:1:1:1:1:1:1:1:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Archive task - src/components/tasks/TaskFullView.tsx:217~~ (PRD:18:1:1:1:1:1:1:1:1:1:1:1)
- [ ] ~~Implement TODO: Pause execution - src/components/tasks/TaskFullView.tsx:221~~ (PRD:21:1:1:1)
- [ ] ~~Implement TODO: Stop execution - src/components/tasks/TaskFullView.tsx:225~~ (PRD:21:1:1:1)
- [ ] ~~Implement TODO: File change handling in useEvents - src/hooks/useEvents.ts:88~~ (PRD:1:1:1)

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


### P3 - Low Impact

- [x] [P3] [Backend] Code cleanup: Resolve TODO comment about task dependencies - src-tauri/src/application/task_transition_service.rs:104
- [x] [P3] [Backend] Code cleanup: Resolve TODO comment about ideation sessions - src-tauri/src/commands/test_data_commands.rs:206
- [x] [P3] [Backend] Code cleanup: Resolve TODO comment about streaming implementation - src-tauri/src/infrastructure/agents/claude/claude_code_client.rs:249
- [x] [P3] [Backend] Code cleanup: Resolve TODO comment about agent context - src-tauri/src/commands/task_commands/mutation.rs:353

---

## REFILL (Added 2026-01-29 18:30)

### P2 - Medium Impact

_No active P2 items. Completed items moved to archive._

### P3 - Low Impact

- [x] [P3] [Frontend] Remove console.error call - src/hooks/useAskUserQuestion.ts:95
- [x] [P3] [Frontend] Remove console.warn call - src/components/tasks/TaskFullView.tsx:213
- [x] [P3] [Frontend] Remove console.warn call - src/components/tasks/TaskFullView.tsx:217
- [x] [P3] [Frontend] Remove console.warn call - src/components/tasks/TaskFullView.tsx:221
- [x] [P3] [Frontend] Remove console.warn call - src/components/tasks/TaskFullView.tsx:225
- [x] [P3] [Frontend] Remove console.warn call - src/components/tasks/detail-views/CompletedTaskDetail.tsx:257
- [x] [P3] [Frontend] Remove console.warn call - src/components/tasks/detail-views/HumanReviewTaskDetail.tsx:365

---

## REFILL (Added 2026-01-29 20:30)

### P2 - Medium Impact

- [x] [P2] [Backend] Error logging suppression: Multiple `.map_err(|_|` patterns discard error details - src-tauri/src/http_server/handlers/steps.rs:20

### P3 - Low Impact

_No active P3 items. Completed items moved to archive._

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 0 (9 excluded, 10 deferred to PRD)
**Completed:** 11
**Validated:** 69 strikethroughs (2026-01-29 x54) - 14 archived, 1 reactivated (moved to refactor as P1), 54 incremented
**Last maintenance:** 2026-01-29 20:30 (completed 1 P2 item)
