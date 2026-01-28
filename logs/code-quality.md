# Code Quality Backlog

> Auto-maintained by quality improvement loop. Explore agent populates, iterations consume.

## How This Works
1. Each iteration reads this file first
2. Pick ONE item matching current task scope (small task = P3, large task = P1)
3. Execute the fix, mark `[x]`, commit with `refactor:` prefix
4. If all items done → Launch Explore agent to replenish

---

## Frontend (src/)

### P1 - High Impact
- [ ] Extract messagesData useMemo hook to avoid dependency chain issues - src/components/Chat/ChatPanel.tsx:473
- [ ] Extract messagesData useMemo hook to avoid dependency chain issues - src/components/Chat/IntegratedChatPanel.tsx:519
- [ ] Split ExtensibilityView (1239 LOC) - extract Workflows/Artifacts/Research/Methodologies into sub-components - src/components/ExtensibilityView.tsx:1-50
- [ ] Split IdeationView (1198 LOC) - extract ideation session and proposal panels into sub-components - src/components/Ideation/IdeationView.tsx:1-50
- [ ] Reduce ChatPanel component size (1044 LOC) - extract ResizeablePanel and message rendering logic - src/components/Chat/ChatPanel.tsx:1-100
- [ ] Reduce IntegratedChatPanel component size (1021 LOC) - extract scrolling logic and message rendering - src/components/Chat/IntegratedChatPanel.tsx:1-100
- [ ] Fix react-hooks/exhaustive-deps in ChatPanel - wrap messagesData in useMemo - src/components/Chat/ChatPanel.tsx:855

### P2 - Medium Impact
- [ ] Extract constants from ScreenshotGallery into separate file (react-refresh/only-export-components) - src/components/qa/ScreenshotGallery/ScreenshotGallery.tsx:693
- [ ] Extract constants from ScreenshotGallery/index into separate file - src/components/qa/ScreenshotGallery/index.tsx:3
- [ ] Extract constants from TaskQABadge into separate file - src/components/qa/TaskQABadge.tsx:103
- [ ] Extract constants from TaskBoard/index into separate file - src/components/tasks/TaskBoard/index.tsx:10
- [ ] Extract constants from TaskFormFields into separate file - src/components/tasks/TaskFormFields.tsx:18
- [ ] Extract form field constants from TaskFormFields - src/components/tasks/TaskFormFields.tsx:28-64
- [ ] Extract constants from ui/badge - src/components/ui/badge.tsx:36
- [ ] Extract constants from ui/button - src/components/ui/button.tsx:58
- [ ] Extract constants from ui/toggle - src/components/ui/toggle.tsx:45
- [ ] Fix useReviews hook with multiple useMemo hooks - wrap data derivation in useMemo - src/hooks/useReviews.ts:142
- [ ] Fix TaskChatPanel messagesData dependency issue in useMemo - src/components/tasks/TaskChatPanel.tsx:233
- [ ] Reduce DiffViewer component size (966 LOC) - extract diff rendering logic - src/components/diff/DiffViewer.tsx:1-50
- [ ] Reduce SettingsView size (827 LOC) - extract settings sections into sub-components - src/components/settings/SettingsView.tsx:1-50
- [ ] Replace type assertions (as unknown) in test files with proper types - src/test/setup.ts
- [ ] Fix type assertion in App.tsx (as unknown as TaskProposal[]) - src/App.tsx:1

### P3 - Low Impact
- [ ] Implement TODO: Call Tauri command for answer submission - src/App.tsx (line ~200)
- [ ] Implement TODO: Approve review modal - src/App.tsx (line ~400)
- [ ] Implement TODO: Request changes modal - src/App.tsx (line ~410)
- [ ] Implement TODO: Open diff viewer - src/App.tsx (line ~420)
- [ ] Implement TODO: Edit task modal - src/components/tasks/TaskFullView.tsx (line ~100)
- [ ] Implement TODO: Archive task - src/components/tasks/TaskFullView.tsx (line ~120)
- [ ] Implement TODO: Pause execution - src/components/tasks/TaskFullView.tsx (line ~130)
- [ ] Implement TODO: Stop execution - src/components/tasks/TaskFullView.tsx (line ~140)
- [ ] Implement TODO: File change handling in useEvents - src/hooks/useEvents.ts (line ~50)

---

## Backend (src-tauri/)

### P1 - High Impact
- [ ] Split ideation_commands.rs (2580 LOC) - extract session management and proposal handlers - src-tauri/src/commands/ideation_commands.rs:1-50
- [ ] Split task_commands.rs (1867 LOC) - extract task mutation and query handlers - src-tauri/src/commands/task_commands.rs:1-50
- [ ] Split chat_service.rs (2039 LOC) - extract message handling and streaming logic - src-tauri/src/application/chat_service.rs:1-50
- [ ] Split apply_service.rs (1833 LOC) - extract proposal application handlers - src-tauri/src/application/apply_service.rs:1-50
- [ ] Split ideation_service.rs (1666 LOC) - extract session and brainstorm logic - src-tauri/src/application/ideation_service.rs:1-50
- [ ] Split dependency_service.rs (1434 LOC) - extract dependency resolution logic - src-tauri/src/application/dependency_service.rs:1-50
- [ ] Split priority_service.rs (1299 LOC) - extract priority calculation logic - src-tauri/src/application/priority_service.rs:1-50
- [ ] Review unwrap/expect usage in migrations.rs (5658 LOC) - improve error handling patterns - src-tauri/src/infrastructure/sqlite/migrations.rs:1-50

### P2 - Medium Impact
- [ ] Implement TODO: Optimize with proper database search - src-tauri/src/http_server.rs:1951
- [ ] Implement TODO: Full-text search index for production - src-tauri/src/commands/task_context_commands.rs
- [ ] Implement TODO: Add ideation sessions to test data - src-tauri/src/commands/test_data_commands.rs
- [ ] Implement TODO: Store answer for agent context - src-tauri/src/commands/task_commands.rs:1867
- [ ] Implement TODO: Task dependencies wiring - src-tauri/src/application/task_transition_service.rs
- [ ] Replace TODO state mappings with proper state variants - src-tauri/src/application/task_transition_service.rs
- [ ] Implement TODO: Track start time for duration - src-tauri/src/infrastructure/agents/claude/claude_code_client.rs
- [ ] Implement TODO: Proper streaming implementation - src-tauri/src/infrastructure/agents/claude/claude_code_client.rs
- [ ] Reduce review_commands.rs size (790 LOC) - extract review handlers - src-tauri/src/commands/review_commands.rs:1-50
- [ ] Reduce task_step_commands.rs size (764 LOC) - extract step handlers - src-tauri/src/commands/task_step_commands.rs:1-50
- [ ] Extract task_qa_repo (repetitive CRUD patterns) - src-tauri/src/infrastructure/memory/memory_task_qa_repo.rs

### P3 - Low Impact
- [ ] Implement TODO: Handle tracking for specific agent - src-tauri/src/infrastructure/agents/spawner.rs
- [ ] Implement TODO: ChatContextType::Review in state transitions - src-tauri/src/domain/state_machine/transition_handler.rs
- [ ] Review methodology.rs entity type definitions (1363 LOC) for extraction opportunities - src-tauri/src/domain/entities/methodology.rs:1-50
- [ ] Review artifact_flow.rs entity type definitions (1389 LOC) - src-tauri/src/domain/entities/artifact_flow.rs:1-50
- [ ] Review research.rs entity type definitions (1398 LOC) - src-tauri/src/domain/entities/research.rs:1-50
- [ ] Review ideation.rs entity type definitions (3979 LOC) for module breakdown - src-tauri/src/domain/entities/ideation.rs:1-50
- [ ] Extract artifact_flow_service (1247 LOC) - split domain logic from queries - src-tauri/src/domain/services/artifact_flow_service.rs:1-50
- [ ] Extract artifact_service (1140 LOC) - separate concerns - src-tauri/src/domain/services/artifact_service.rs:1-50

---

## Last Explored
**Date:** 2026-01-28 12:30:00
**Areas:** src/, src-tauri/
**Agent:** a092c9d
**Total Issues:** 72 (P1: 23 | P2: 32 | P3: 17)
