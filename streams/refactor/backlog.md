# Refactor Backlog (P1 - Large Splits)

> P1 items are large file splits and architectural refactors.
> Files exceeding LOC limits: backend 500, frontend component 500, hook 300.
> Reference: `.claude/rules/code-quality-standards.md`

## Frontend (src/)

- [x] Split ExtensibilityView.panels (906 LOC → 382 LOC max) - extracted to ExtensibilityView.WorkflowsPanel.tsx (192 LOC), ExtensibilityView.ArtifactsPanel.tsx (272 LOC), ExtensibilityView.ResearchPanel.tsx (382 LOC), ExtensibilityView.utils.tsx (70 LOC) - src/components/ExtensibilityView.panels.tsx:1-50
- [x] Split IdeationView (1105 LOC → 438 LOC) - extracted SessionBrowser, StartSessionPanel, ProposalCard, ProposalsToolbar, ProactiveSyncNotification, ProposalsEmptyState, and useIdeationHandlers hook - src/components/Ideation/IdeationView.tsx:1-50
- [x] Reduce ChatPanel component size (1041 LOC → 774 LOC) - extracted ResizeablePanel and ChatMessages components - src/components/Chat/ChatPanel.tsx:1-100
- [x] Reduce IntegratedChatPanel component size (1025 LOC → 498 LOC) - extracted useIntegratedChatScroll, useIntegratedChatHandlers, useIntegratedChatEvents hooks and IntegratedChatPanel.components.tsx - src/components/Chat/IntegratedChatPanel.tsx:1-100

## Backend (src-tauri/)

- [x] Split ideation_commands.rs (2595 LOC → 1660 LOC excluding tests) - extracted to 7 focused modules: types, session, proposals, dependencies, apply, chat, orchestrator - src-tauri/src/commands/ideation_commands/mod.rs:1-50
- [x] Split task_commands.rs (1992 LOC → 496 LOC max) - extracted to task_commands/{types.rs (137), helpers.rs (78), query.rs (203), mutation.rs (496), tests.rs (1094), mod.rs (53)} - src-tauri/src/commands/task_commands/mod.rs:1-50
- [x] Split chat_service.rs (2109 LOC → 1263 LOC) - extracted to chat_service/{types.rs (135), helpers.rs (25), streaming.rs (255), mock.rs (259), mod.rs (1263)} - src-tauri/src/application/chat_service/mod.rs:1-50
- [x] Split apply_service.rs (1833 LOC → 309 LOC) - extracted to apply_service/{types.rs (60), helpers.rs (107), tests.rs (1408), mod.rs (309)} - src-tauri/src/application/apply_service/mod.rs:1-50
- [x] Split ideation_service.rs (1666 LOC → 423 LOC) - extracted to ideation_service/{types.rs (70), tests.rs (1198), mod.rs (423)} - src-tauri/src/application/ideation_service/mod.rs:1-50
- [x] Split dependency_service.rs (1435 LOC → 479 LOC) - extracted to dependency_service/{types.rs (57), tests.rs (908), mod.rs (479)} - src-tauri/src/application/dependency_service/mod.rs:1-50
- [x] Split priority_service.rs (1300 LOC → 379 LOC) - extracted to priority_service/{tests.rs (924), mod.rs (379)} - src-tauri/src/application/priority_service/mod.rs:1-50
- [ ] Review unwrap/expect usage in migrations.rs (5658 LOC) - improve error handling patterns - src-tauri/src/infrastructure/sqlite/migrations.rs:1-50
- [ ] Split ideation.rs (3979 LOC) entity - break into sub-modules - src-tauri/src/domain/entities/ideation.rs:1-50
- [ ] Split research.rs (1398 LOC) entity - extract to focused modules - src-tauri/src/domain/entities/research.rs:1-50
- [ ] Split artifact_flow.rs (1389 LOC) entity - extract types/helpers - src-tauri/src/domain/entities/artifact_flow.rs:1-50
- [ ] Split methodology.rs (1363 LOC) entity - extract types/helpers - src-tauri/src/domain/entities/methodology.rs:1-50

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**Active items:** 5 | **Completed:** 8 | **Archived:** 1
**Last maintenance:** 2026-01-29 (added 1 item from polish strikethrough validation)
