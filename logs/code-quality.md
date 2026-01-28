# Code Quality Backlog

> Full protocol: `.claude/rules/quality-improvement.md` | LOC limits: `.claude/rules/code-quality-standards.md`

## Quick Reference
- **Pick:** P0 first → then by scope (small=P3, medium=P2, large=P1)
- **Verify:** Issue exists? Not in PRD? → Execute → `[x]` → `refactor:` commit
- **Defer:** Fixed=`(stale)` | Planned=`(PRD:N)` | Counter: `→:1→:2→archive`
- **Cleanup:** `[x]` > 10/section → `logs/code-quality-archive.md`

## Markers
`[ ]` Pending | `[x]` Done | `(stale[:N])` Fixed | `(PRD:N[:V])` Planned | `(excluded)` Permanent

## Exclusions
`src/components/ui/*` — shadcn/ui

---

## Frontend (src/)

### P0 - Critical (Phase Gaps)
<!-- Gaps found during phase verification go here - pick FIRST -->

### P1 - High Impact
- [x] Extract messagesData useMemo hook to avoid dependency chain issues - src/components/Chat/ChatPanel.tsx:473
- [x] Extract messagesData useMemo hook to avoid dependency chain issues - src/components/Chat/IntegratedChatPanel.tsx:519
- [ ] Split ExtensibilityView (1239 LOC) - extract Workflows/Artifacts/Research/Methodologies into sub-components - src/components/ExtensibilityView.tsx:1-50
- [ ] Split IdeationView (1105 LOC, was 1198) - extract ideation session and proposal panels into sub-components - src/components/Ideation/IdeationView.tsx:1-50
- [ ] Reduce ChatPanel component size (1044 LOC) - extract ResizeablePanel and message rendering logic - src/components/Chat/ChatPanel.tsx:1-100
- [ ] Reduce IntegratedChatPanel component size (1021 LOC) - extract scrolling logic and message rendering - src/components/Chat/IntegratedChatPanel.tsx:1-100
- [ ] ~~Fix react-hooks/exhaustive-deps in ChatPanel - wrap messagesData in useMemo - src/components/Chat/ChatPanel.tsx:855~~ (stale - messagesData already wrapped in useMemo at line 473)
- [x] Reduce TaskCard component size (531 LOC → 407 LOC) - extracted styling utilities to TaskCard.utils.ts - src/components/tasks/TaskBoard/TaskCard.tsx:1-100
- [x] Reduce ReviewsPanel component size (605 LOC → 233 LOC) - extracted sub-components to ReviewsPanel.utils.tsx - src/components/reviews/ReviewsPanel.tsx:1-50

### P2 - Medium Impact
- [x] Extract PRIORITY_CONFIG and animationStyles from IdeationView to IdeationView.constants.ts - src/components/Ideation/IdeationView.tsx
- [x] Remove duplicate PRIORITY_CONFIG from ProposalCard, import from shared constants - src/components/Ideation/ProposalCard.tsx
- [x] Extract constants from ScreenshotGallery into separate file (react-refresh/only-export-components) - src/components/qa/ScreenshotGallery/ScreenshotGallery.tsx:693
- [x] ~~Extract constants from ScreenshotGallery/index into separate file - src/components/qa/ScreenshotGallery/index.tsx:3~~ (fixed - removed utility re-export that caused react-refresh warning)
- [x] Remove useTaskBoard re-export from TaskBoard/index.tsx (react-refresh warning) - src/components/tasks/TaskBoard/index.tsx:10
- [x] Extract constants from TaskQABadge into separate file - src/components/qa/TaskQABadge.tsx:103
- [x] Remove duplicate workflowKeys from TaskBoard/hooks.ts, use canonical @/hooks/useWorkflows instead - src/components/tasks/TaskBoard/hooks.ts:26
- [x] Extract constants from TaskFormFields into separate file - src/components/tasks/TaskFormFields.tsx:18
- [x] Remove backward-compat re-exports from TaskFormFields.tsx (react-refresh lint) - updated imports in TaskCreationForm.tsx and TaskEditForm.tsx
- [ ] ~~Extract constants from ui/badge - src/components/ui/badge.tsx:36~~ (excluded)
- [ ] ~~Extract constants from ui/button - src/components/ui/button.tsx:58~~ (excluded)
- [ ] ~~Extract constants from ui/toggle - src/components/ui/toggle.tsx:45~~ (excluded)
- [x] Fix useReviews hook with multiple useMemo hooks - wrap data derivation in useMemo - src/hooks/useReviews.ts:142
- [x] Fix TaskChatPanel messagesData dependency issue in useMemo - src/components/tasks/TaskChatPanel.tsx:233
- [x] Reduce DiffViewer component size (966 LOC) - extract types/utils to DiffViewer.types.tsx (now 740 LOC) - src/components/diff/DiffViewer.tsx:1-50
- [x] Reduce SettingsView size (827 LOC → 449 LOC) - extracted shared components to SettingsView.shared.tsx - src/components/settings/SettingsView.tsx:1-50
- [x] Replace type assertions (as unknown) in test files with proper types - src/test/setup.ts
- [x] Fix type assertion in App.tsx (as unknown as TaskProposal[]) - src/App.tsx:1

### P3 - Low Impact
- [x] Extract duplicate SectionTitle component to shared.tsx in detail-views - src/components/tasks/detail-views/*.tsx
- [x] Extract MODEL_OPTIONS constant from SettingsView.shared.tsx to SettingsView.constants.ts (react-refresh lint) - src/components/settings/SettingsView.shared.tsx:30
- [ ] ~~Implement TODO: Call Tauri command for answer submission - src/App.tsx (line ~200)~~ (PRD:20)
- [ ] ~~Implement TODO: Approve review modal - src/App.tsx (line ~400)~~ (PRD:20)
- [ ] ~~Implement TODO: Request changes modal - src/App.tsx (line ~410)~~ (PRD:20)
- [ ] ~~Implement TODO: Open diff viewer - src/App.tsx (line ~420)~~ (PRD:20)
- [ ] ~~Implement TODO: Edit task modal - src/components/tasks/TaskFullView.tsx (line ~100)~~ (PRD:18)
- [ ] ~~Implement TODO: Archive task - src/components/tasks/TaskFullView.tsx (line ~120)~~ (PRD:18)
- [ ] ~~Implement TODO: Pause execution - src/components/tasks/TaskFullView.tsx (line ~130)~~ (PRD:21)
- [ ] ~~Implement TODO: Stop execution - src/components/tasks/TaskFullView.tsx (line ~140)~~ (PRD:21)
- [ ] ~~Implement TODO: File change handling in useEvents - src/hooks/useEvents.ts (line ~50)~~ (PRD:19)

---

## Backend (src-tauri/)

### P0 - Critical (Phase Gaps)
<!-- Gaps found during phase verification go here - pick FIRST -->
- [x] Fix direct status update in chat_service.rs (lines 824-830) - use TaskTransitionService instead of direct DB update - src-tauri/src/application/chat_service.rs:824

### P1 - High Impact
- [ ] Split ideation_commands.rs (2580 LOC) - extract session management and proposal handlers - src-tauri/src/commands/ideation_commands.rs:1-50
- [ ] Split task_commands.rs (1867 LOC) - extract task mutation and query handlers - src-tauri/src/commands/task_commands.rs:1-50
- [ ] Split chat_service.rs (2039 LOC) - extract message handling and streaming logic - src-tauri/src/application/chat_service.rs:1-50
- [ ] Split apply_service.rs (1833 LOC) - extract proposal application handlers - src-tauri/src/application/apply_service.rs:1-50
- [ ] Split ideation_service.rs (1666 LOC) - extract session and brainstorm logic - src-tauri/src/application/ideation_service.rs:1-50
- [ ] Split dependency_service.rs (1434 LOC) - extract dependency resolution logic - src-tauri/src/application/dependency_service.rs:1-50
- [ ] Split priority_service.rs (1299 LOC) - extract priority calculation logic - src-tauri/src/application/priority_service.rs:1-50
- [ ] Review unwrap/expect usage in migrations.rs (5658 LOC) - improve error handling patterns - src-tauri/src/infrastructure/sqlite/migrations.rs:1-50
- [ ] Split ideation.rs (3979 LOC) entity - break into sub-modules - src-tauri/src/domain/entities/ideation.rs:1-50
- [ ] Split research.rs (1398 LOC) entity - extract to focused modules - src-tauri/src/domain/entities/research.rs:1-50
- [ ] Split artifact_flow.rs (1389 LOC) entity - extract types/helpers - src-tauri/src/domain/entities/artifact_flow.rs:1-50
- [ ] Split methodology.rs (1363 LOC) entity - extract types/helpers - src-tauri/src/domain/entities/methodology.rs:1-50
- [x] Split artifact_flow_service.rs (1247 LOC → 304 LOC) - extracted tests to artifact_flow_service_tests.rs - src-tauri/src/domain/services/artifact_flow_service.rs:1-50
- [x] Split artifact_service.rs (1140 LOC → 266 LOC) - extracted tests to artifact_service_tests.rs - src-tauri/src/domain/services/artifact_service.rs:1-50

### P2 - Medium Impact
- [ ] Implement TODO: Optimize with proper database search - src-tauri/src/http_server.rs:1294
- [ ] ~~Implement TODO: Full-text search index for production - src-tauri/src/commands/task_context_commands.rs:113~~ (stale:1 - TODO exists at different line, feature-level work)
- [ ] ~~Implement TODO: Add ideation sessions to test data - src-tauri/src/commands/test_data_commands.rs:206~~ (stale:1 - TODO exists, feature-level work)
- [ ] ~~Implement TODO: Store answer for agent context - src-tauri/src/commands/task_commands.rs:530~~ (stale:1 - TODO exists, feature-level work)
- [ ] ~~Implement TODO: Task dependencies wiring - src-tauri/src/application/task_transition_service.rs:104~~ (stale:1 - TODO exists, feature-level work)
- [x] Fix incorrect InternalStatus→State mapping in execute_entry_actions (Reviewing→PendingReview, ReExecuting→Executing) - src-tauri/src/application/task_transition_service.rs:329-332
- [x] Implement TODO: Track start time for duration - src-tauri/src/infrastructure/agents/claude/claude_code_client.rs
- [ ] Implement TODO: Proper streaming implementation - src-tauri/src/infrastructure/agents/claude/claude_code_client.rs
- [x] Reduce review_commands.rs size (790 LOC → 663 LOC) - extracted types to review_commands_types.rs - src-tauri/src/commands/review_commands.rs:1-50
- [x] Reduce task_step_commands.rs size (764 LOC → 711 LOC) - extracted types to task_step_commands_types.rs - src-tauri/src/commands/task_step_commands.rs:1-50
- [x] Extract emit_step_updated helper in task_step_commands.rs (711 LOC → 689 LOC) - src-tauri/src/commands/task_step_commands.rs:16-26
- [x] Extract Column.utils.tsx from Column.tsx (392 LOC → 350 LOC) - src/components/tasks/TaskBoard/Column.tsx
- [x] Extract ReviewStateBadge from TaskCard.tsx (621 LOC → 531 LOC) - src/components/tasks/TaskBoard/TaskCard.tsx
- [ ] ~~Extract task_qa_repo (repetitive CRUD patterns) - src-tauri/src/infrastructure/memory/memory_task_qa_repo.rs~~ (stale:1 - file is 336 LOC, under 500 limit)

### P3 - Low Impact
- [x] Extract duplicate status-to-state conversion into internal_status_to_state helper - src-tauri/src/application/task_transition_service.rs:128-147
- [x] Remove unused TransitionObserver trait (dead code) - src-tauri/src/domain/state_machine/transition_handler.rs:393
- [x] Remove broad clippy allows (dead_code, unused_imports, unused_variables) from lib.rs - src-tauri/src/lib.rs:23-25
- [x] Implement TODO: Fetch maxRevisionCycles from review settings - src-tauri/src/http_server.rs:1115
- [x] Implement TODO: Handle tracking for specific agent - src-tauri/src/infrastructure/agents/spawner.rs:138,143
- [x] Implement TODO: ChatContextType::Review in state transitions - src-tauri/src/domain/state_machine/transition_handler.rs
- [x] Add StateGroup.locked() convenience method for system-managed groups - src-tauri/src/domain/entities/workflow.rs:67
- [x] Consolidate duplicate ExecutionState imports in spawner.rs tests - src-tauri/src/infrastructure/agents/spawner.rs:204
- [x] Reduce spawner.rs (529 LOC → 500 LOC) - consolidated role_from_string tests - src-tauri/src/infrastructure/agents/spawner.rs:1-50
- [x] Extract spawner.rs tests (560 LOC → 220 LOC) - extracted tests to spawner_tests.rs - src-tauri/src/infrastructure/agents/spawner.rs:1-50
- [x] Make AGENT_ACTIVE_STATUSES public for reuse by StartupJobRunner - src-tauri/src/commands/execution_commands.rs:12
- [x] Remove stale comment about execute_entry_actions being private - src-tauri/src/application/startup_jobs.rs:109
- [ ] ~~Add contextual error messages in artifact type parsing failures - src-tauri/src/commands/artifact_commands.rs:158,216,357~~ (stale:1 - all locations already have contextual messages)
- [ ] ~~Extract duplicate parse error handling pattern in workflow/ideation commands - src-tauri/src/commands/workflow_commands.rs:25~~ (stale:1 - not a duplicate pattern, each is specific to its context)
- [x] Use ProfileRole Display/FromStr traits instead of custom role_to_string/string_to_role helpers - src-tauri/src/infrastructure/sqlite/sqlite_agent_profile_repo.rs:34-55

---

## Last Explored
**Date:** 2026-01-28 14:00:00
**Areas:** src-tauri/src/commands/, src-tauri/src/application/
**Agent:** a98d79f
**Total Issues:** 74 (P1: 23 | P2: 32 | P3: 19)
