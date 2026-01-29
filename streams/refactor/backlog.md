# Refactor Backlog (P1 - Large Splits)

> P1 items are large file splits and architectural refactors.
> Files exceeding LOC limits: backend 500, frontend component 500, hook 300.
> Reference: `.claude/rules/code-quality-standards.md`

## Frontend (src/)

_No active P1 items. Completed items moved to archive._

## Backend (src-tauri/)

_Completed items moved to archive._

## REFILL (Added 2026-01-29)

### Backend

- [x] Split chat_service/mod.rs (1081 LOC → 611 LOC) - extracted background processing to chat_service_send_background.rs (524 LOC) - src-tauri/src/application/chat_service/mod.rs:1

---

## REFILL (Added 2026-01-29 21:07)

### Frontend

- [x] Split App.tsx (855 LOC → 721 LOC) - extracted Navigation.tsx (88 LOC), useAppKeyboardShortcuts.ts (102 LOC) - src/App.tsx:1
- [x] Split ChatPanel.tsx (776 LOC → 480 LOC) - extracted useChatPanelHandlers.ts (368 LOC) - src/components/Chat/ChatPanel.tsx:1
- [x] Split DiffViewer.tsx (740 LOC → 284 LOC) - extracted sub-components to DiffViewer.components.tsx (502 LOC) - src/components/diff/DiffViewer.tsx:1
- [x] Split TaskDetailModal.tsx (690 LOC → 487 LOC) - extracted TaskDetailModal.constants.ts (116 LOC), TaskDetailModal.components.tsx (108 LOC) - src/components/tasks/TaskDetailModal.tsx:1
- [x] Split ProjectCreationWizard.tsx (688 LOC → 535 LOC) - extracted ProjectCreationWizard.helpers.ts (87 LOC), ProjectCreationWizard.components.tsx (86 LOC) - src/components/projects/ProjectCreationWizard/ProjectCreationWizard.tsx:1

### Backend

- [x] Split sqlite_task_proposal_repo.rs (1190 LOC → 352 LOC) - extracted to sqlite_task_proposal_repo/{mod.rs (352), tests.rs (838)} - src-tauri/src/infrastructure/sqlite/sqlite_task_proposal_repo/mod.rs:1
- [x] Split artifact.rs (1147 LOC → 671 LOC) - extracted to artifact/{mod.rs (7), types.rs (671), tests.rs (480)} - src-tauri/src/domain/entities/artifact/mod.rs:1
- [x] Split machine.rs (1114 LOC → 485 LOC) - extracted to machine/{mod.rs (11), types.rs (242), transitions.rs (243), tests.rs (633)} - src-tauri/src/domain/state_machine/machine/mod.rs:1

---

## REFILL (Added 2026-01-30)

### Backend

- [x] Split memory_task_repo.rs (1149 LOC → 402 LOC) - extracted to memory_task_repo/{mod.rs (402), tests.rs (747)} - src-tauri/src/infrastructure/memory/memory_task_repo/mod.rs:1
- [ ] Split research_service.rs (1109 LOC) - extract helper functions and validation - src-tauri/src/domain/services/research_service.rs:1
- [ ] Split sqlite_chat_message_repo.rs (1065 LOC) - extract query builders and result mapping - src-tauri/src/infrastructure/sqlite/sqlite_chat_message_repo.rs:1
- [ ] Split sqlite_proposal_dependency_repo.rs (1062 LOC) - extract query logic and helpers - src-tauri/src/infrastructure/sqlite/sqlite_proposal_dependency_repo.rs:1

### Frontend

- [ ] Split lib/tauri.ts (858 LOC) - extract response schemas into separate modules - src/lib/tauri.ts:1
- [ ] Split App.tsx (714 LOC) - extract view components and handlers - src/App.tsx:1
- [ ] Split ScreenshotGallery.tsx (681 LOC) - extract gallery controls and image rendering - src/components/qa/ScreenshotGallery/ScreenshotGallery.tsx:1
- [ ] Split ActivityView.tsx (641 LOC) - extract activity filtering and rendering logic - src/components/activity/ActivityView.tsx:1
- [ ] Split useChatPanelHandlers.ts (368 LOC) - extract message handling into separate hook - src/hooks/useChatPanelHandlers.ts:1

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 8 | **Completed:** 10 | **Archived:** 18
**Last maintenance:** 2026-01-30 (archived 2 items)
