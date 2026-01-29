# Refactor Backlog (P1 - Large Splits)

> P1 items are large file splits and architectural refactors.
> Files exceeding LOC limits: backend 500, frontend component 500, hook 300.
> Reference: `.claude/rules/code-quality-standards.md`

## Frontend (src/)

_No active P1 items. Completed items moved to archive._

## Backend (src-tauri/)
- [x] Split migrations.rs (5694 LOC → 1304 LOC) - extracted to migrations/tests.rs (4390 LOC), migrations/mod.rs (1304 LOC) with all 25 migrations - src-tauri/src/infrastructure/sqlite/migrations/mod.rs:1-50
- [x] Split ideation.rs (3982 LOC → 426 LOC max) - extracted to ideation/{mod.rs (234), types.rs (365), proposal.rs (201), assessment.rs (426), chat.rs (258), graph.rs (198), tests.rs (2345)} - src-tauri/src/domain/entities/ideation/mod.rs:1-50
- [x] Split research.rs (1398 LOC → 351 LOC max) - extracted to research/{mod.rs (351), types.rs (324), tests.rs (740)} - src-tauri/src/domain/entities/research/mod.rs:1-50
- [x] Split artifact_flow.rs (1389 LOC → 434 LOC max) - extracted to artifact_flow/{mod.rs (160), types.rs (434), tests.rs (816)} - src-tauri/src/domain/entities/artifact_flow/mod.rs:1-50
- [x] Split methodology.rs (1363 LOC → 664 LOC max) - extracted to methodology/{mod.rs (664), tests.rs (698)} - src-tauri/src/domain/entities/methodology/mod.rs:1-50

## REFILL (Added 2026-01-29)

### Backend

- [x] Split transition_handler.rs (1474 LOC → 250 LOC max) - extracted to transition_handler/{mod.rs (160), side_effects.rs (250), tests.rs (1071)} - src-tauri/src/domain/state_machine/transition_handler.rs:1
- [x] Split sqlite_task_repo.rs (1372 LOC → 466 LOC) - extracted to sqlite_task_repo/{mod.rs (466), helpers.rs (58), queries.rs (49), query_builder.rs (57), tests.rs (796)} - src-tauri/src/infrastructure/sqlite/sqlite_task_repo/mod.rs:1
- [x] Split migrations/mod.rs (1324 LOC → 200 LOC) - extracted to migrations/{migrations_v1_v10.rs (276), migrations_v11_v20.rs (561), migrations_v21_v26.rs (201), mod.rs (200)} - src-tauri/src/infrastructure/sqlite/migrations/mod.rs:1
- [ ] Split chat_service/mod.rs (1081 LOC) - extract send_message background task logic and repository operations - src-tauri/src/application/chat_service/mod.rs:1 (Note: Previously extracted context (241 LOC) and queue (246 LOC), still exceeds 500 LOC limit)

---

## REFILL (Added 2026-01-29 21:07)

### Frontend

- [x] Split App.tsx (855 LOC → 721 LOC) - extracted Navigation.tsx (88 LOC), useAppKeyboardShortcuts.ts (102 LOC) - src/App.tsx:1
- [x] Split ChatPanel.tsx (776 LOC → 480 LOC) - extracted useChatPanelHandlers.ts (368 LOC) - src/components/Chat/ChatPanel.tsx:1
- [ ] Split DiffViewer.tsx (740 LOC) - extract diff formatting utilities - src/components/diff/DiffViewer.tsx:1
- [ ] Split TaskDetailModal.tsx (690 LOC) - extract form and step management - src/components/tasks/TaskDetailModal.tsx:1
- [ ] Split ProjectCreationWizard.tsx (688 LOC) - extract wizard steps into separate components - src/components/projects/ProjectCreationWizard/ProjectCreationWizard.tsx:1

### Backend

- [ ] Split sqlite_task_proposal_repo.rs (1190 LOC) - extract query operations to helpers - src-tauri/src/infrastructure/sqlite/sqlite_task_proposal_repo.rs:1
- [ ] Split artifact.rs (1147 LOC) - extract entity methods to artifact_impl.rs - src-tauri/src/domain/entities/artifact.rs:1
- [ ] Split machine.rs (1114 LOC) - extract transition logic to machine_transitions.rs - src-tauri/src/domain/state_machine/machine.rs:1

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 8 | **Completed:** 10 | **Archived:** 10
**Last maintenance:** 2026-01-29 02:05 (archived 1 item)
