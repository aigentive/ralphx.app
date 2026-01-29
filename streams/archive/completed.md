# Archived Completed Items

> Completed items moved from stream backlogs when count exceeds 10.
> Archive format: date, original section, item details.

---

## Archived 2026-01-30

### From refactor/backlog.md (Backend)

- [x] Split transition_handler.rs (1474 LOC → 250 LOC max) - extracted to transition_handler/{mod.rs (160), side_effects.rs (250), tests.rs (1071)} - src-tauri/src/domain/state_machine/transition_handler.rs:1

### From polish/backlog.md (REFILL - Added 2026-01-29 20:43)

- [x] [P3] [Backend] Redundant clone: Unnecessary `response.clone()` when value is already moved - src-tauri/src/http_server/handlers/steps.rs:304
- [x] [P3] [Backend] Redundant clones: Excessive `.clone()` calls in request parameters - src-tauri/src/http_server/handlers/permissions.rs:22

---

## Archived 2026-01-29 (Sixteenth Batch)

### From refactor/backlog.md (Backend)

- [x] Split ideation.rs (3982 LOC → 426 LOC max) - extracted to ideation/{mod.rs (234), types.rs (365), proposal.rs (201), assessment.rs (426), chat.rs (258), graph.rs (198), tests.rs (2345)} - src-tauri/src/domain/entities/ideation/mod.rs:1-50

### From polish/backlog.md (REFILL - Added 2026-01-29 20:43)

- [x] [P3] [Backend] Dead code: Unused variable `_rx` never referenced - src-tauri/src/http_server/handlers/permissions.rs:18

---

## Archived 2026-01-29 (Fifteenth Batch)

### From refactor/backlog.md (Backend)

- [x] Split migrations.rs (5694 LOC → 1304 LOC) - extracted to migrations/tests.rs (4390 LOC), migrations/mod.rs (1304 LOC) with all 25 migrations - src-tauri/src/infrastructure/sqlite/migrations/mod.rs:1-50

### From polish/backlog.md (REFILL - Added 2026-01-29 20:43)

- [x] [P2] [Backend] Inconsistent error handling: Missing error logging in task handlers despite error pattern established elsewhere - src-tauri/src/http_server/handlers/tasks.rs:42

---

## Archived 2026-01-29 (Fourteenth Batch)

### From polish/backlog.md (REFILL - Added 2026-01-29 20:43)

- [x] [P2] [Backend] Type safety: Use `format!("{:?}", ...)` for enum serialization instead of proper variants - src-tauri/src/http_server/types.rs:81
- [x] [P2] [Backend] Inconsistent error handling: Direct `.map_err(|_| StatusCode::...)` swallows error details; use tracing like in steps.rs - src-tauri/src/http_server/handlers/ideation.rs:58
- [x] [P2] [Backend] Inconsistent error handling: Missing error logging in artifact handlers despite having tracing in steps.rs - src-tauri/src/http_server/handlers/artifacts.rs:31

### From polish/backlog.md (Strikethrough Validation - stale:2)

- [ ] ~~[P3] [Backend] Dead code: Unused variable `_session_link` never referenced~~ (stale:2 - variable already removed)
- [ ] ~~[P3] [Backend] Redundant clone: Using `.clone()` in serde_json::json! macro (owned values unnecessary)~~ (stale:2 - already using &response)

---

## Archived 2026-01-29 (Thirteenth Batch)

### From refactor/backlog.md (Backend)

- [x] Split priority_service.rs (1300 LOC → 379 LOC) - extracted to priority_service/{tests.rs (924), mod.rs (379)} - src-tauri/src/application/priority_service/mod.rs:1-50

### From polish/backlog.md (REFILL - Added 2026-01-29 20:43)

- [x] [P2] [Backend] Error logging suppression: Multiple `.map_err(|_|` patterns discard error details - src-tauri/src/http_server/handlers/steps.rs:20

### From polish/backlog.md (Strikethrough Validation - stale:2)

- [ ] ~~[P2] [Backend] Inconsistent error handling: Missing error logging in review handlers despite tracing available - src-tauri/src/http_server/handlers/reviews.rs:27~~ (stale:2 - handlers return error messages via Result<T, (StatusCode, String)> pattern)

---

## Archived 2026-01-29 (Twelfth Batch)

### From refactor/backlog.md (Strikethrough Validation - stale:2)

- [ ] ~~Split http_server/mod.rs (1515 LOC) - extract HTTP handler routes to separate handler modules~~ (stale:2 - now 84 LOC, already extracted to handlers/ directory)

### From polish/backlog.md (REFILL - Added 2026-01-29 18:30)

- [x] [P3] [Frontend] Remove console.warn call - src/components/tasks/detail-views/CompletedTaskDetail.tsx:257
- [x] [P3] [Frontend] Remove console.warn call - src/components/tasks/detail-views/HumanReviewTaskDetail.tsx:365

---

## Archived 2026-01-29 (Eleventh Batch)

### From refactor/backlog.md (Backend)

- [x] Split dependency_service.rs (1435 LOC → 479 LOC) - extracted to dependency_service/{types.rs (57), tests.rs (908), mod.rs (479)} - src-tauri/src/application/dependency_service/mod.rs:1-50

### From polish/backlog.md (REFILL - Added 2026-01-29 18:30)

- [x] [P3] [Frontend] Remove console.error call - src/hooks/useAskUserQuestion.ts:95
- [x] [P3] [Frontend] Remove console.warn call - src/components/tasks/TaskFullView.tsx:213
- [x] [P3] [Frontend] Remove console.warn call - src/components/tasks/TaskFullView.tsx:217
- [x] [P3] [Frontend] Remove console.warn call - src/components/tasks/TaskFullView.tsx:221
- [x] [P3] [Frontend] Remove console.warn call - src/components/tasks/TaskFullView.tsx:225

---

## Archived 2026-01-29 (Tenth Batch)

### From polish/backlog.md (REFILL - Added 2026-01-29 00:43)

- [x] [P3] [Backend] Code cleanup: Resolve TODO comment about streaming implementation - src-tauri/src/infrastructure/agents/claude/claude_code_client.rs:249
- [x] [P3] [Backend] Code cleanup: Resolve TODO comment about agent context - src-tauri/src/commands/task_commands/mutation.rs:353

---

## Archived 2026-01-29 (Ninth Batch)

### From refactor/backlog.md (Backend)

- [x] Split apply_service.rs (1833 LOC → 309 LOC) - extracted to apply_service/{types.rs (60), helpers.rs (107), tests.rs (1408), mod.rs (309)} - src-tauri/src/application/apply_service/mod.rs:1-50
- [x] Split ideation_service.rs (1666 LOC → 423 LOC) - extracted to ideation_service/{types.rs (70), tests.rs (1198), mod.rs (423)} - src-tauri/src/application/ideation_service/mod.rs:1-50

---

## Archived 2026-01-29 (Eighth Batch)

### From refactor/backlog.md (Backend)

- [x] Split chat_service.rs (2109 LOC → 1263 LOC) - extracted to chat_service/{types.rs (135), helpers.rs (25), streaming.rs (255), mock.rs (259), mod.rs (1263)} - src-tauri/src/application/chat_service/mod.rs:1-50

### From polish/backlog.md (REFILL - Added 2026-01-29 00:43)

- [x] [P3] [Backend] Code cleanup: Resolve TODO comment about task dependencies - src-tauri/src/application/task_transition_service.rs:104
- [x] [P3] [Backend] Code cleanup: Resolve TODO comment about ideation sessions - src-tauri/src/commands/test_data_commands.rs:206

### From polish/backlog.md (PRD-deferred - Validated 20+ times)

- [ ] ~~Implement TODO: Open diff viewer - src/App.tsx:791~~ (PRD:20 - validated 20+ times, stable as future work)
- [ ] ~~Implement TODO: Edit task modal - src/components/tasks/TaskFullView.tsx:213~~ (PRD:18 - validated 18+ times, stable as future work)
- [ ] ~~Implement TODO: Archive task - src/components/tasks/TaskFullView.tsx:217~~ (PRD:18 - validated 12+ times, stable as future work)

---

## Archived 2026-01-29 (Seventh Batch)

### From refactor/backlog.md (Backend)

- [x] Split task_commands.rs (1992 LOC → 496 LOC max) - extracted to task_commands/{types.rs (137), helpers.rs (78), query.rs (203), mutation.rs (496), tests.rs (1094), mod.rs (53)} - src-tauri/src/commands/task_commands/mod.rs:1-50

### From polish/backlog.md (REFILL - Added 2026-01-29 00:43)

- [x] [P2] [Backend] Error handling: Replace `.unwrap()` calls in test assertions - src-tauri/src/domain/supervisor/patterns.rs:329
- [x] [P2] [Backend] Error handling: Replace serde `.unwrap()` calls in tests - src-tauri/src/domain/supervisor/events.rs:361
- [x] [P2] [Backend] Error handling: Remove dead_code allow attribute - src-tauri/src/domain/agents/mod.rs:24
- [x] [P2] [Backend] Error handling: Remove dead_code allow attribute - src-tauri/src/application/dependency_service/mod.rs:12
- [x] [P3] [Backend] Code cleanup: Resolve TODO comment about database search optimization - src-tauri/src/http_server.rs:1296

### From polish/backlog.md (Strikethrough Validation - stale:2)

- [ ] ~~[P2] [Backend] Error handling: Replace `.expect()` calls with proper type-safe parsing - src-tauri/src/domain/entities/ideation.rs:171~~ (stale:2 - file no longer exists, split into module)
- [ ] ~~[P2] [Backend] Error handling: Replace `.parse().unwrap()` with Result handling - src-tauri/src/domain/entities/ideation.rs:1686~~ (stale:2 - file no longer exists, split into module)

---

## Archived 2026-01-29 (Sixth Batch)

### From polish/backlog.md (REFILL - Added 2026-01-29 00:43)

- [x] [P2] [Backend] Error handling: Replace `.unwrap()` with proper error handling - src-tauri/src/domain/supervisor/patterns.rs:146
- [x] [P2] [Backend] Error handling: Replace `.unwrap()` with proper error handling in serialization tests - src-tauri/src/domain/supervisor/patterns.rs:417

---

## Archived 2026-01-29 (Fifth Batch)

### From polish/backlog.md (REFILL - Added 2026-01-29 00:00)

- [x] [P3] [Backend] Remove dead code suppression and clean - src-tauri/src/application/priority_service/tests.rs:11

---

## Archived 2026-01-29 (Fourth Batch)

### From refactor/backlog.md (Frontend + Backend)

- [x] Reduce IntegratedChatPanel component size (1025 LOC → 498 LOC) - extracted useIntegratedChatScroll, useIntegratedChatHandlers, useIntegratedChatEvents hooks and IntegratedChatPanel.components.tsx - src/components/Chat/IntegratedChatPanel.tsx:1-100
- [x] Split ideation_commands.rs (2595 LOC → 1660 LOC excluding tests) - extracted to 7 focused modules: types, session, proposals, dependencies, apply, chat, orchestrator - src-tauri/src/commands/ideation_commands/mod.rs:1-50

### From polish/backlog.md (REFILL - Added 2026-01-29 00:00)

- [x] [P3] [Frontend] Remove TODO comments for unimplemented task actions - src/components/tasks/TaskFullView.tsx:213
- [x] [P3] [Frontend] Remove TODO in HumanReviewTaskDetail - src/components/tasks/detail-views/HumanReviewTaskDetail.tsx:365
- [x] [P3] [Backend] Remove TODO placeholder search implementation - src-tauri/src/commands/task_context_commands.rs:113

---

## Archived 2026-01-29 (Third Batch)

### From refactor/backlog.md (Frontend)

- [x] Reduce ChatPanel component size (1041 LOC → 774 LOC) - extracted ResizeablePanel and ChatMessages components - src/components/Chat/ChatPanel.tsx:1-100

### From polish/backlog.md (REFILL - Added 2026-01-29 00:00)

- [x] [P2] [Backend] Replace unwrap() calls with proper error handling in ideation_commands - src-tauri/src/commands/ideation_commands/mod.rs:45
- [x] [P2] [Backend] Remove #[allow(dead_code)] suppression and verify actual usage - src-tauri/src/application/ideation_service/tests.rs:2
- [x] [P3] [Frontend] Remove TODO comment for diff viewer integration - src/components/tasks/detail-views/CompletedTaskDetail.tsx:257

### From polish/backlog.md (Strikethrough Validation - stale:2)

- [ ] ~~[P3] [Frontend] Fast refresh warning: Extract constants from ResizeablePanel.tsx component export~~ (stale:2 - constants already extracted to ResizeablePanel.constants.ts)
- [ ] ~~[P3] [Frontend] Remove TODO in PlanTemplateSelector - src/components/Ideation/PlanTemplateSelector.tsx:94~~ (stale:2 - TODO not found at line 94)
- [ ] ~~[P3] [Backend] Clean up unused test fixtures in dependency_service - src-tauri/src/application/dependency_service.rs:530~~ (stale:2 - file no longer exists)

---

## Archived 2026-01-29 (Second Batch)

### From refactor/backlog.md (Frontend)

- [x] Split IdeationView (1105 LOC → 438 LOC) - extracted SessionBrowser, StartSessionPanel, ProposalCard, ProposalsToolbar, ProactiveSyncNotification, ProposalsEmptyState, and useIdeationHandlers hook - src/components/Ideation/IdeationView.tsx:1-50

### From polish/backlog.md (REFILL - Added 2026-01-29 00:00)

- [x] [P2] [Frontend] Consolidate ChatPanel console.error handlers into unified error handler - src/components/Chat/ChatPanel.tsx:332
- [x] [P2] [Frontend] Remove TODO comment for Tauri command integration - src/App.tsx:359

---

## Archived 2026-01-29 (First Batch)

### From refactor/backlog.md (Frontend)

- [x] Split ExtensibilityView.panels (906 LOC → 382 LOC max) - extracted to ExtensibilityView.WorkflowsPanel.tsx (192 LOC), ExtensibilityView.ArtifactsPanel.tsx (272 LOC), ExtensibilityView.ResearchPanel.tsx (382 LOC), ExtensibilityView.utils.tsx (70 LOC) - src/components/ExtensibilityView.panels.tsx:1-50

### From polish/backlog.md (REFILL - Added 2026-01-28)

- [x] [P2] [Frontend] Type safety: Replace z.unknown() with proper types - src/api/chat.ts:115
- [x] [P2] [Frontend] Refactor large API file (821 LOC → 473 LOC) - extracted schemas, transforms, types - src/api/ideation.ts:1
- [x] [P3] [Frontend] Replace promise chain .then() with async/await - src/hooks/useSupervisorAlerts.listener.ts:100
- [x] [P3] [Frontend] Error handling: Check empty catch blocks - src/components/Chat/ChatPanel.tsx:342

### From polish/backlog.md (REFILL - Added 2026-01-28 23:47)

- [x] [P2] [Frontend] Error handling: App.tsx catch blocks need proper user feedback via toast - src/App.tsx:330
- [x] [P2] [Frontend] Event listener cleanup: useResizePanel needs useEffect for document listener lifecycle - src/components/Chat/ResizeablePanel.tsx:63
- [x] [P2] [Frontend] Unnecessary useMemo: Multiple dependencies in ChatPanel could be optimized - src/components/Chat/ChatPanel.tsx:200
- [x] [P2] [Frontend] Extract ToolCallIndicator sub-functions - src/components/Chat/ToolCallIndicator.tsx:49-200

---

## Archived 2026-01-29 (Earlier)

### From polish/backlog.md (REFILL - Added 2026-01-28)

- [x] [P2] [Backend] Replace panic! with proper error handling - src-tauri/src/infrastructure/agents/claude/stream_processor.rs:432
- [x] [P2] [Backend] Replace .unwrap() calls with error handling - src-tauri/src/error.rs:95
- [x] [P2] [Backend] Replace .unwrap() calls with error handling - src-tauri/src/commands/artifact_commands.rs:452
- [x] [P2] [Backend] Replace .unwrap() calls with error handling - src-tauri/src/commands/review_commands.rs:375

### From polish/backlog.md (REFILL - Added 2026-01-28 23:47) - Strikethrough Validation

- [ ] ~~[P3] [Frontend] Fast refresh warning: Extract constants from ResizeablePanel.tsx component export~~ (stale:2 - constants already extracted to ResizeablePanel.constants.ts)

---

## Archived 2026-01-28

### From polish/backlog.md (Strikethrough Validation - 2026-01-28)

- [ ] ~~[P2] [Frontend] Type safety - Replace `any` with proper types in test mocks~~ (stale:2 - fixed with proper typing in useChat.test.ts)
- [ ] ~~[P3] [Frontend] Remove console.debug statements from production code~~ (stale:2 - no console.debug at those lines) - src/components/Chat/IntegratedChatPanel.tsx:370,402,442
- [ ] ~~[P3] [Frontend] Remove console.log stub from event handler~~ (stale:2 - no console.log present at line 263) - src/components/tasks/detail-views/CompletedTaskDetail.tsx:263

### From polish/backlog.md (PRD-deferred)

- [ ] ~~Implement TODO: Call Tauri command for answer submission - src/App.tsx (line ~200)~~ (PRD:20:1:2 - verified removed)
- [ ] ~~Implement TODO: Approve review modal - src/App.tsx (line ~400)~~ (PRD:20:1:1:1:1:1 - verified removed)
- [ ] ~~Implement TODO: Request changes modal - src/App.tsx (line ~410)~~ (PRD:20:1:1:1:1 - verified removed)

## Migrated from logs/code-quality.md (2026-01-28)

### Stale Items (Verified Fixed - Archived 2026-01-28)

- [ ] ~~Split ExtensibilityView (1076 LOC, was 1239) - extract Workflows/Artifacts/Research panels into sub-components~~ (stale:2 - now 205 LOC)
- [ ] ~~[P3] [Frontend] Remove console.debug statements from useChat (agent event tracing) - src/hooks/useChat.ts:368,404,431~~ (stale:2 - removed during refactor)
- [ ] ~~[P2] [Frontend] Error handling: console.error in useChat lacks structured error reporting~~ (stale:2 - removed during refactor)
- [ ] ~~[P2] [Frontend] Error handling: console.error in useEvents lacks structured error reporting~~ (stale:2 - removed during refactor)

### P0 - Critical (Phase Gaps)

- [x] [Backend] inject_task doesn't emit queue_changed when creating task with Ready status (target=planned) - src-tauri/src/commands/task_commands.rs:512
- [x] [Backend] answer_user_question doesn't emit queue_changed when transitioning task Blocked→Ready - src-tauri/src/commands/task_commands.rs:557
- [x] [Backend] approve_fix_task doesn't emit queue_changed when transitioning task Blocked→Ready - src-tauri/src/commands/review_commands.rs:198
- [x] [Backend] apply_proposals_to_kanban doesn't emit queue_changed when creating tasks with Ready status (todo column) - src-tauri/src/commands/ideation_commands.rs:998
- [x] [Backend] reject_fix_task doesn't emit queue_changed when creating new fix task with Ready status - src-tauri/src/commands/review_commands.rs:243
- [x] [Frontend] Orphaned: View Registry not wired - TaskDetailOverlay/TaskFullView need `useViewRegistry={true}` - src/components/tasks/TaskDetailOverlay.tsx:508, TaskFullView.tsx:343
- [x] [Backend] Fix direct status update in chat_service.rs (lines 824-830) - use TaskTransitionService instead of direct DB update - src-tauri/src/application/chat_service.rs:824

### P1 - High Impact (Frontend)

- [x] Extract messagesData useMemo hook to avoid dependency chain issues - src/components/Chat/ChatPanel.tsx:473
- [x] Extract messagesData useMemo hook to avoid dependency chain issues - src/components/Chat/IntegratedChatPanel.tsx:519
- [x] Reduce TaskCard component size (531 LOC → 407 LOC) - extracted styling utilities to TaskCard.utils.ts - src/components/tasks/TaskBoard/TaskCard.tsx:1-100
- [x] Reduce ReviewsPanel component size (605 LOC → 233 LOC) - extracted sub-components to ReviewsPanel.utils.tsx - src/components/reviews/ReviewsPanel.tsx:1-50

### P1 - High Impact (Backend)

- [x] Split artifact_flow_service.rs (1247 LOC → 304 LOC) - extracted tests to artifact_flow_service_tests.rs - src-tauri/src/domain/services/artifact_flow_service.rs:1-50
- [x] Split artifact_service.rs (1140 LOC → 266 LOC) - extracted tests to artifact_service_tests.rs - src-tauri/src/domain/services/artifact_service.rs:1-50

### P2 - Medium Impact (Frontend)

- [x] Remove unnecessary step.clone() in http_server.rs step handlers (derive Clone on StepResponse, create response once) - src-tauri/src/http_server.rs:1653,1705,1757,1809
- [x] Extract PRIORITY_CONFIG and animationStyles from IdeationView to IdeationView.constants.ts - src/components/Ideation/IdeationView.tsx
- [x] Remove duplicate PRIORITY_CONFIG from ProposalCard, import from shared constants - src/components/Ideation/ProposalCard.tsx
- [x] Extract constants from ScreenshotGallery into separate file (react-refresh/only-export-components) - src/components/qa/ScreenshotGallery/ScreenshotGallery.tsx:693
- [x] ~~Extract constants from ScreenshotGallery/index into separate file - src/components/qa/ScreenshotGallery/index.tsx:3~~ (fixed - removed utility re-export that caused react-refresh warning)
- [x] Remove useTaskBoard re-export from TaskBoard/index.tsx (react-refresh warning) - src/components/tasks/TaskBoard/index.tsx:10
- [x] Extract constants from TaskQABadge into separate file - src/components/qa/TaskQABadge.tsx:103
- [x] Remove duplicate workflowKeys from TaskBoard/hooks.ts, use canonical @/hooks/useWorkflows instead - src/components/tasks/TaskBoard/hooks.ts:26
- [x] Extract constants from TaskFormFields into separate file - src/components/tasks/TaskFormFields.tsx:18
- [x] Remove backward-compat re-exports from TaskFormFields.tsx (react-refresh lint) - updated imports in TaskCreationForm.tsx and TaskEditForm.tsx
- [x] Fix useReviews hook with multiple useMemo hooks - wrap data derivation in useMemo - src/hooks/useReviews.ts:142
- [x] Fix TaskChatPanel messagesData dependency issue in useMemo - src/components/tasks/TaskChatPanel.tsx:233
- [x] Reduce DiffViewer component size (966 LOC) - extract types/utils to DiffViewer.types.tsx (now 740 LOC) - src/components/diff/DiffViewer.tsx:1-50
- [x] Reduce SettingsView size (827 LOC → 449 LOC) - extracted shared components to SettingsView.shared.tsx - src/components/settings/SettingsView.tsx:1-50
- [x] Replace type assertions (as unknown) in test files with proper types - src/test/setup.ts
- [x] Fix type assertion in App.tsx (as unknown as TaskProposal[]) - src/App.tsx:1

### P2 - Medium Impact (Backend)

- [x] Extract emit_task_lifecycle_event helper for consistent task event emission - src-tauri/src/commands/task_commands.rs:359-369
- [x] Fix incorrect InternalStatus→State mapping in execute_entry_actions (Reviewing→PendingReview, ReExecuting→Executing) - src-tauri/src/application/task_transition_service.rs:329-332
- [x] Implement TODO: Track start time for duration - src-tauri/src/infrastructure/agents/claude/claude_code_client.rs
- [x] Reduce review_commands.rs size (790 LOC → 663 LOC) - extracted types to review_commands_types.rs - src-tauri/src/commands/review_commands.rs:1-50
- [x] Reduce task_step_commands.rs size (764 LOC → 711 LOC) - extracted types to task_step_commands_types.rs - src-tauri/src/commands/task_step_commands.rs:1-50
- [x] Extract emit_step_updated helper in task_step_commands.rs (711 LOC → 689 LOC) - src-tauri/src/commands/task_step_commands.rs:16-26
- [x] Extract Column.utils.tsx from Column.tsx (392 LOC → 350 LOC) - src/components/tasks/TaskBoard/Column.tsx
- [x] Extract ReviewStateBadge from TaskCard.tsx (621 LOC → 531 LOC) - src/components/tasks/TaskBoard/TaskCard.tsx

### P3 - Low Impact (Frontend)

- [x] Remove debug console.log statements from handleNewSession - src/App.tsx:369-378
- [x] Extract duplicate SectionTitle component to shared.tsx in detail-views - src/components/tasks/detail-views/*.tsx
- [x] Extract MODEL_OPTIONS constant from SettingsView.shared.tsx to SettingsView.constants.ts (react-refresh lint) - src/components/settings/SettingsView.shared.tsx:30
- [x] Remove console.log debug statements from TaskFullView action handlers - src/components/tasks/TaskFullView.tsx:214,219,224,229
- [x] Remove debug console.log from handleQuestionSubmit - src/App.tsx:352
- [x] Remove debug console.log from handleEditProposal and onViewDiff - src/App.tsx:404,785
- [x] Remove debug console.log in ChatPanel contextKey effect - src/components/Chat/ChatPanel.tsx:401
- [x] Remove debug console.log in ChatPanel agent run completion - src/components/Chat/ChatPanel.tsx:762
- [x] Remove debug console.log in ExtensibilityView research handler - src/components/ExtensibilityView.tsx:629
- [x] Remove debug console.log in handleViewDiff - src/components/tasks/detail-views/HumanReviewTaskDetail.tsx:366

### P3 - Low Impact (Backend)

- [x] Replace `.unwrap()` with `.expect()` in setup_test_state for better debugging - src-tauri/src/commands/execution_commands.rs:498-527
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
- [x] Use ProfileRole Display/FromStr traits instead of custom role_to_string/string_to_role helpers - src-tauri/src/infrastructure/sqlite/sqlite_agent_profile_repo.rs:34-55

### Stale Items (Verified Fixed - Archived after 2 validations)

- [ ] ~~Implement TODO: Full-text search index for production - src-tauri/src/commands/task_context_commands.rs:113~~ (stale:2 - TODO not found, ARCHIVED)
- [ ] ~~Implement TODO: Add ideation sessions to test data - src-tauri/src/commands/test_data_commands.rs:206~~ (stale:2 - TODO not found, ARCHIVED)
- [ ] ~~Implement TODO: Store answer for agent context - src-tauri/src/commands/task_commands.rs:530~~ (stale:2 - TODO not found, ARCHIVED)
- [ ] ~~Implement TODO: Task dependencies wiring - src-tauri/src/application/task_transition_service.rs:104~~ (stale:2 - TODO not found, ARCHIVED)
- [ ] ~~Implement TODO: Optimize with proper database search - src-tauri/src/http_server.rs:1294~~ (stale - explicitly marked "future iteration")
- [ ] ~~Implement TODO: Proper streaming implementation - src-tauri/src/infrastructure/agents/claude/claude_code_client.rs~~ (stale - intentional placeholder, not yet needed)
- [ ] ~~Extract task_qa_repo (repetitive CRUD patterns) - src-tauri/src/infrastructure/memory/memory_task_qa_repo.rs~~ (stale:2 - file is 336 LOC, under 500 limit - ARCHIVED)
- [ ] ~~Add contextual error messages in artifact type parsing failures - src-tauri/src/commands/artifact_commands.rs:158,216,357~~ (stale:2 - all locations already have contextual messages - ARCHIVED)
- [ ] ~~Extract duplicate parse error handling pattern in workflow/ideation commands - src-tauri/src/commands/workflow_commands.rs:25~~ (stale:2 - not a duplicate pattern, each is specific to its context - ARCHIVED)
- [ ] ~~Fix react-hooks/exhaustive-deps in ChatPanel - wrap messagesData in useMemo - src/components/Chat/ChatPanel.tsx:855~~ (stale - messagesData already wrapped in useMemo at line 473)

---

## Archived 2026-01-28 (Hygiene Validation)

### From polish/backlog.md (Strikethrough Validation)

- [ ] ~~[P3] [Frontend] Remove eslint-disable comment from useTaskExecutionState exhaustive-deps~~ (stale:2 - disable is justified for stable helper functions, validated twice)
- [ ] ~~[P3] [Frontend] Remove unused imports: useIntegratedChatScroll, useIntegratedChatHandlers, useIntegratedChatEvents~~ (stale:2 - imports are used at lines 259, 273, 293, validated twice)
- [ ] ~~[P3] [Frontend] Remove console.debug statements from production code~~ (stale:2 - no console.debug at those lines, validated twice) - src/components/Chat/IntegratedChatPanel.tsx:370,402,442
- [ ] ~~[P3] [Frontend] Remove unused variable binding in test mock~~ (stale:1 - fixed with proper typing)

---

## Archived 2026-01-28 (Hygiene Cycle)

### From polish/backlog.md (P2)

- [x] [P2] [Frontend] Extract hook logic from useChat (528 LOC → 344 LOC) - extracted event handling to useAgentEvents - src/hooks/useChat.ts:1-528

---

## Archived 2026-01-28 (Hygiene Cycle)

### From polish/backlog.md (P2)

- [x] [P2] [Frontend] Extract hook logic from useEvents (417 LOC → 102 LOC) - split by event type - src/hooks/useEvents.ts:1-417
- [x] [P2] [Frontend] Extract hook logic from useSupervisorAlerts (409 LOC → 184 LOC) - split alert management into store and listener modules - src/hooks/useSupervisorAlerts.ts:1-409
- [x] [P2] [Frontend] Unused parameter: defaultStatus in TaskCreationForm prop defaults to undefined - src/components/tasks/TaskCreationForm.tsx:59

### From polish/backlog.md (P3)

- [x] [P3] [Frontend] Remove console.warn from App.tsx global shortcut registration - src/App.tsx:283
- [x] [P3] [Frontend] Remove eslint-disable comments from useChat.test.ts (6 occurrences) - properly typed zustand mock - src/hooks/useChat.test.ts:8,29,37,42,58,103

---

## Archived 2026-01-28 (Hygiene Cycle)

### From polish/backlog.md (P3)

- [x] Remove debug console.log from agent:run_started handler - src/hooks/useChat.ts:321
- [x] Remove console.log statements in TaskChatPanel event listeners - src/components/tasks/TaskChatPanel.tsx:351,366,390,405

### From polish/backlog.md (REFILL - P2)

- [x] [P2] [Backend] Replace .expect() with error handling - src-tauri/src/http_server.rs:395
- [x] [P2] [Frontend] Type safety: Replace z.any() with specific type - src/types/task-context.ts:56

### From polish/backlog.md (Strikethrough Validation - 2026-01-28)

- [ ] ~~[P3] [Frontend] Remove console.debug statements from production code~~ (stale:2 - no console.debug present, only console.error which is appropriate) - src/hooks/useIntegratedChatHandlers.ts:97,132,172

---

**Total archived:** 72 items (7 P0 + 6 P1 + 23 P2 + 24 P3 + 12 stale)
