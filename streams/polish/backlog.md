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

- [ ] ~~Implement TODO: Pause execution - src/components/tasks/TaskFullView.tsx:221~~ (PRD:21:1:1:1:1:1)
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

## REFILL (Added 2026-01-29 20:43)

### P2 - Medium Impact

- [x] [P2] [Backend] Type safety: Use `format!("{:?}", ...)` for enum serialization instead of proper variants - src-tauri/src/http_server/types.rs:81
- [x] [P2] [Backend] Inconsistent error handling: Direct `.map_err(|_| StatusCode::...)` swallows error details; use tracing like in steps.rs - src-tauri/src/http_server/handlers/ideation.rs:58
- [x] [P2] [Backend] Inconsistent error handling: Missing error logging in artifact handlers despite having tracing in steps.rs - src-tauri/src/http_server/handlers/artifacts.rs:31
- [ ] ~~[P2] [Backend] Inconsistent error handling: Missing error logging in task handlers despite error pattern established elsewhere - src-tauri/src/http_server/handlers/tasks.rs:42~~ (stale - all error handlers now have proper logging)
- [ ] [P2] [Backend] Inconsistent error handling: Missing error logging in review handlers despite tracing available - src-tauri/src/http_server/handlers/reviews.rs:27

### P3 - Low Impact

- [ ] [P3] [Backend] Dead code: Unused variable `_rx` never referenced - src-tauri/src/http_server/handlers/permissions.rs:18
- [ ] [P3] [Backend] Dead code: Unused variable `_session_link` never referenced - src-tauri/src/http_server/handlers/artifacts.rs:34
- [ ] [P3] [Backend] Redundant clone: Using `.clone()` in serde_json::json! macro (owned values unnecessary) - src-tauri/src/http_server/handlers/steps.rs:75
- [ ] [P3] [Backend] Redundant clone: Unnecessary `response.clone()` when value is already moved - src-tauri/src/http_server/handlers/steps.rs:304
- [ ] [P3] [Backend] Redundant clones: Excessive `.clone()` calls in request parameters - src-tauri/src/http_server/handlers/permissions.rs:22

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 7 (9 excluded, 3 deferred to PRD)
**Completed:** 11
**Validated:** 72 strikethroughs (2026-01-29 x57) - 21 archived, 1 reactivated (moved to refactor as P1), 57 incremented
**Last maintenance:** 2026-01-29 22:15 (archived 2 items)
