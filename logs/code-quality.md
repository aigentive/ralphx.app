# Code Quality Backlog

> Auto-maintained by quality improvement loop. Explore agent populates, iterations consume.

## How This Works
1. Each iteration reads this file first
2. **P0 items ALWAYS get picked first** (gaps from phase verification)
3. Then pick ONE item matching current task scope (small = P3, medium = P2, large = P1)
4. **VERIFY:**
   - Issue still exists? (read file:line)
   - NOT in active PRD? (cross-reference with current phase task list)
   - **LOC items: Check against `.claude/rules/code-quality-standards.md`**
5. Valid & not in PRD? → Execute the fix, mark `[x]`, commit with `refactor:` prefix
6. Stale (issue genuinely FIXED)? → Strikethrough text `~~text~~ (stale)`, pick next
   - **"Stale" = fixed, NOT "I think it's fine"**
   - **Files over LOC limits are NOT stale — they need extraction**
7. In PRD (planned work)? → Strikethrough text `~~text~~ (PRD)`, pick next
8. **Scope exhausted? → ESCALATE** (P3→P2→P1, pick larger scope item)
9. **ALL exhausted? → Launch Explore agent** to replenish, then pick ONE

## NO SKIPPING
**"Nothing to do" is NOT valid.** Escalate scope or replenish. A `refactor:` commit is MANDATORY.

## TODO Tracking
**Adding a TODO during task work?** Log it here immediately:
`- [ ] [P2/P3] Implement TODO: [description] - file:line`

## Priority Levels
- **P0 - Critical**: Gaps found during phase verification (pick FIRST, any task size)
- **P1 - High**: Architecture, major refactors (large tasks)
- **P2 - Medium**: Error handling, extraction (medium tasks)
- **P3 - Low**: Lint, naming, cleanup (small tasks)

## Markers
- `[ ]` Pending
- `[x]` Done
- `[ ] ~~text~~ (stale)` — Strikethrough for already fixed
- `[ ] ~~text~~ (PRD)` — Strikethrough for PRD-planned tasks
- `[ ] ~~text~~ (excluded)` — Strikethrough for excluded paths

## Exclusions (do NOT scan or pick)
- `src/components/ui/*` — shadcn/ui components (upgraded externally)

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
- [ ] ~~Implement TODO: Call Tauri command for answer submission - src/App.tsx (line ~200)~~ (PRD)
- [ ] ~~Implement TODO: Approve review modal - src/App.tsx (line ~400)~~ (PRD)
- [ ] ~~Implement TODO: Request changes modal - src/App.tsx (line ~410)~~ (PRD)
- [ ] ~~Implement TODO: Open diff viewer - src/App.tsx (line ~420)~~ (PRD)
- [ ] ~~Implement TODO: Edit task modal - src/components/tasks/TaskFullView.tsx (line ~100)~~ (PRD)
- [ ] ~~Implement TODO: Archive task - src/components/tasks/TaskFullView.tsx (line ~120)~~ (PRD)
- [ ] ~~Implement TODO: Pause execution - src/components/tasks/TaskFullView.tsx (line ~130)~~ (PRD)
- [ ] ~~Implement TODO: Stop execution - src/components/tasks/TaskFullView.tsx (line ~140)~~ (PRD)
- [ ] ~~Implement TODO: File change handling in useEvents - src/hooks/useEvents.ts (line ~50)~~ (PRD)

---

## Backend (src-tauri/)

### P0 - Critical (Phase Gaps)
<!-- Gaps found during phase verification go here - pick FIRST -->

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
- [ ] Split artifact_flow_service.rs (1247 LOC) - separate queries from domain logic - src-tauri/src/domain/services/artifact_flow_service.rs:1-50
- [ ] Split artifact_service.rs (1140 LOC) - separate concerns - src-tauri/src/domain/services/artifact_service.rs:1-50

### P2 - Medium Impact
- [ ] Implement TODO: Optimize with proper database search - src-tauri/src/http_server.rs:1951
- [ ] Implement TODO: Full-text search index for production - src-tauri/src/commands/task_context_commands.rs
- [ ] Implement TODO: Add ideation sessions to test data - src-tauri/src/commands/test_data_commands.rs
- [ ] Implement TODO: Store answer for agent context - src-tauri/src/commands/task_commands.rs:1867
- [ ] Implement TODO: Task dependencies wiring - src-tauri/src/application/task_transition_service.rs
- [ ] Replace TODO state mappings with proper state variants - src-tauri/src/application/task_transition_service.rs
- [ ] Implement TODO: Track start time for duration - src-tauri/src/infrastructure/agents/claude/claude_code_client.rs
- [ ] Implement TODO: Proper streaming implementation - src-tauri/src/infrastructure/agents/claude/claude_code_client.rs
- [x] Reduce review_commands.rs size (790 LOC → 663 LOC) - extracted types to review_commands_types.rs - src-tauri/src/commands/review_commands.rs:1-50
- [ ] Reduce task_step_commands.rs size (764 LOC) - extract step handlers - src-tauri/src/commands/task_step_commands.rs:1-50
- [ ] Extract task_qa_repo (repetitive CRUD patterns) - src-tauri/src/infrastructure/memory/memory_task_qa_repo.rs

### P3 - Low Impact
- [x] Implement TODO: Fetch maxRevisionCycles from review settings - src-tauri/src/http_server.rs:1115
- [~] Implement TODO: Handle tracking for specific agent - src-tauri/src/infrastructure/agents/spawner.rs (STALE: TODO not found)
- [x] Implement TODO: ChatContextType::Review in state transitions - src-tauri/src/domain/state_machine/transition_handler.rs

---

## Last Explored
**Date:** 2026-01-28 12:30:00
**Areas:** src/, src-tauri/
**Agent:** a092c9d
**Total Issues:** 72 (P1: 23 | P2: 32 | P3: 17)
