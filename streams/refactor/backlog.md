# Refactor Backlog (P1 - Large Splits)

> P1 items are large file splits and architectural refactors.
> Files exceeding LOC limits: backend 500, frontend component 500, hook 300.
> Reference: `.claude/rules/code-quality-standards.md`

## Frontend (src/)

- [x] Split IdeationView (1105 LOC → 438 LOC) - extracted SessionBrowser, StartSessionPanel, ProposalCard, ProposalsToolbar, ProactiveSyncNotification, ProposalsEmptyState, and useIdeationHandlers hook - src/components/Ideation/IdeationView.tsx:1-50
- [x] Reduce ChatPanel component size (1041 LOC → 774 LOC) - extracted ResizeablePanel and ChatMessages components - src/components/Chat/ChatPanel.tsx:1-100
- [x] Reduce IntegratedChatPanel component size (1025 LOC → 498 LOC) - extracted useIntegratedChatScroll, useIntegratedChatHandlers, useIntegratedChatEvents hooks and IntegratedChatPanel.components.tsx - src/components/Chat/IntegratedChatPanel.tsx:1-100

## Backend (src-tauri/)

- [x] Split ideation_commands.rs (2595 LOC → 1660 LOC excluding tests) - extracted to 7 focused modules: types, session, proposals, dependencies, apply, chat, orchestrator - src-tauri/src/commands/ideation_commands/mod.rs:1-50
- [x] Split task_commands.rs (1992 LOC → 496 LOC max) - extracted to task_commands/{types.rs (137), helpers.rs (78), query.rs (203), mutation.rs (496), tests.rs (1094), mod.rs (53)} - src-tauri/src/commands/task_commands/mod.rs:1-50
- [ ] Split chat_service.rs (2109 LOC) - extract message handling and streaming logic (COMPLEX: requires multiple extractions - streaming ~365 LOC, mock ~474 LOC, helpers, then main impl) - src-tauri/src/application/chat_service.rs:1-50
- [ ] Split apply_service.rs (1833 LOC) - extract proposal application handlers - src-tauri/src/application/apply_service.rs:1-50
- [ ] Split ideation_service.rs (1666 LOC) - extract session and brainstorm logic - src-tauri/src/application/ideation_service.rs:1-50
- [ ] Split dependency_service.rs (1434 LOC) - extract dependency resolution logic - src-tauri/src/application/dependency_service.rs:1-50
- [ ] Split priority_service.rs (1299 LOC) - extract priority calculation logic - src-tauri/src/application/priority_service.rs:1-50
- [ ] Review unwrap/expect usage in migrations.rs (5658 LOC) - improve error handling patterns - src-tauri/src/infrastructure/sqlite/migrations.rs:1-50
- [ ] Split ideation.rs (3979 LOC) entity - break into sub-modules - src-tauri/src/domain/entities/ideation.rs:1-50
- [ ] Split research.rs (1398 LOC) entity - extract to focused modules - src-tauri/src/domain/entities/research.rs:1-50
- [ ] Split artifact_flow.rs (1389 LOC) entity - extract types/helpers - src-tauri/src/domain/entities/artifact_flow.rs:1-50
- [ ] Split methodology.rs (1363 LOC) entity - extract types/helpers - src-tauri/src/domain/entities/methodology.rs:1-50

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 11 | **Completed:** 4 | **Archived:** 1
