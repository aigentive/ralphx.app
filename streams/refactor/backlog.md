# Refactor Backlog (P1 - Large Splits)

> P1 items are large file splits and architectural refactors.
> Files exceeding LOC limits: backend 500, frontend component 500, hook 300.
> Reference: `.claude/rules/code-quality-standards.md`

## Frontend (src/)

_No active P1 items. Completed items moved to archive._

## Backend (src-tauri/)
- [x] Split chat_service.rs (2109 LOC → 1263 LOC) - extracted to chat_service/{types.rs (135), helpers.rs (25), streaming.rs (255), mock.rs (259), mod.rs (1263)} - src-tauri/src/application/chat_service/mod.rs:1-50
- [x] Split apply_service.rs (1833 LOC → 309 LOC) - extracted to apply_service/{types.rs (60), helpers.rs (107), tests.rs (1408), mod.rs (309)} - src-tauri/src/application/apply_service/mod.rs:1-50
- [x] Split ideation_service.rs (1666 LOC → 423 LOC) - extracted to ideation_service/{types.rs (70), tests.rs (1198), mod.rs (423)} - src-tauri/src/application/ideation_service/mod.rs:1-50
- [x] Split dependency_service.rs (1435 LOC → 479 LOC) - extracted to dependency_service/{types.rs (57), tests.rs (908), mod.rs (479)} - src-tauri/src/application/dependency_service/mod.rs:1-50
- [x] Split priority_service.rs (1300 LOC → 379 LOC) - extracted to priority_service/{tests.rs (924), mod.rs (379)} - src-tauri/src/application/priority_service/mod.rs:1-50
- [x] Split migrations.rs (5694 LOC → 1304 LOC) - extracted to migrations/tests.rs (4390 LOC), migrations/mod.rs (1304 LOC) with all 25 migrations - src-tauri/src/infrastructure/sqlite/migrations/mod.rs:1-50
- [x] Split ideation.rs (3982 LOC → 426 LOC max) - extracted to ideation/{mod.rs (234), types.rs (365), proposal.rs (201), assessment.rs (426), chat.rs (258), graph.rs (198), tests.rs (2345)} - src-tauri/src/domain/entities/ideation/mod.rs:1-50
- [x] Split research.rs (1398 LOC → 351 LOC max) - extracted to research/{mod.rs (351), types.rs (324), tests.rs (740)} - src-tauri/src/domain/entities/research/mod.rs:1-50
- [x] Split artifact_flow.rs (1389 LOC → 434 LOC max) - extracted to artifact_flow/{mod.rs (160), types.rs (434), tests.rs (816)} - src-tauri/src/domain/entities/artifact_flow/mod.rs:1-50
- [x] Split methodology.rs (1363 LOC → 664 LOC max) - extracted to methodology/{mod.rs (664), tests.rs (698)} - src-tauri/src/domain/entities/methodology/mod.rs:1-50

## REFILL (Added 2026-01-29)

### Backend

- [ ] Split http_server/mod.rs (1515 LOC) - extract HTTP handler routes to separate handler modules (handlers/ideation.rs, artifacts.rs, tasks.rs, projects.rs, reviews.rs, worker.rs, permissions.rs, steps.rs) - src-tauri/src/http_server/mod.rs:1
- [x] Split transition_handler.rs (1474 LOC → 250 LOC max) - extracted to transition_handler/{mod.rs (160), side_effects.rs (250), tests.rs (1071)} - src-tauri/src/domain/state_machine/transition_handler.rs:1
- [ ] Split sqlite_task_repo.rs (1372 LOC) - extract query builders and mapping functions - src-tauri/src/infrastructure/sqlite/sqlite_task_repo.rs:1
- [ ] Split migrations/mod.rs (1304 LOC) - extract migration functions to migrations_v*.rs - src-tauri/src/infrastructure/sqlite/migrations/mod.rs:1
- [ ] Split chat_service/mod.rs (1263 LOC) - extract message queue and context routing - src-tauri/src/application/chat_service/mod.rs:1

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 5 | **Completed:** 10 | **Archived:** 7
**Last maintenance:** 2026-01-29 19:22 (archived 1 completed item)
