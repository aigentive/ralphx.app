# Features Backlog (P0 - Critical Gaps)

> P0 items are phase gaps — bugs where code exists but isn't wired up.
> These BLOCK all PRD work. Fix P0 first, no exceptions.

<!-- All P0 items from Phase 20-22 have been resolved. New P0s from verify stream go here. -->

## From Phase 24 Verification (2026-01-28)

- [x] [Infrastructure] Regex pattern error in fswatch cleanup: pkill pattern uses invalid regex - ralph-tmux.sh:185
- [x] [Infrastructure] Unquoted variable expansion in fswatch arguments - scripts/stream-watch-features.sh:35
- [x] [Infrastructure] Race condition: initial cycle and fswatch startup overlap - scripts/stream-watch-features.sh:24
- [x] [Infrastructure] Orphaned subshells: fswatch pipes not properly managed on stop - ralph-tmux.sh:167
- [x] [Infrastructure] Stream wrappers missing signal trap handlers for clean shutdown - scripts/stream-watch-features.sh:1

## From Phase 24 Re-verification (2026-01-28)

- [x] [Infrastructure] Missing watch file: hygiene stream does not watch streams/features/backlog.md - scripts/stream-watch-hygiene.sh:10

## From Phase 25 Verification (2026-01-29)

- [x] [Backend] Missing migration: v26 for seed_task_id column never added - src-tauri/src/infrastructure/sqlite/migrations/mod.rs

## From Phase 26 Verification (2026-01-29)

- [x] [Backend] Missing production implementation: TaskScheduler trait has no concrete implementation in application layer - src-tauri/src/application/
- [x] [Backend] Service not injected: TaskScheduler missing from AppState builder - src-tauri/src/application/app_state.rs

## From Phase 28 Verification (2026-01-29)

- [x] [Frontend] Dead hook: useIdeationEvents defined but never called in any component - src/hooks/useIdeationEvents.ts:33

## From Phase 54 Verification (2026-02-01)

- [x] [Frontend] Orphaned mutation: handleBlockWithReason bypasses blockMutation, calls api directly - src/components/tasks/TaskBoard/TaskCard.tsx:192

## From Phase 54 Verification (2026-02-01) - Second Pass

- [x] [Frontend] Orphaned mutation: unblockMutation never used - TaskCard uses moveMutation for "Unblock" action instead - src/components/tasks/TaskBoard/TaskCard.tsx:189

## From Phase 55 Verification (2026-01-31)

- [x] [Frontend] Bypassed mock API: useWorkflows imports directly from @/lib/api/workflows instead of centralized api object - src/hooks/useWorkflows.ts:15
- [x] [Frontend] Bypassed mock API: useMethodologies imports directly from @/lib/api/methodologies instead of centralized api object - src/hooks/useMethodologies.ts:12
- [x] [Frontend] Bypassed mock API: useArtifacts imports directly from @/lib/api/artifacts instead of centralized api object - src/hooks/useArtifacts.ts:15
- [x] [Frontend] Bypassed mock API: useResearch imports directly from @/lib/api/research instead of centralized api object - src/hooks/useResearch.ts:12
- [x] [Frontend] Direct invoke call: useAskUserQuestion uses invoke() directly, not mockable - src/hooks/useAskUserQuestion.ts:86
- [x] [Frontend] Direct invoke call: PermissionDialog uses invoke() directly, not mockable - src/components/PermissionDialog.tsx:47

## From Phase 55 Verification (2026-01-31) - Event Listener Bypasses

- [x] [Frontend] Bypassed EventProvider: TaskChatPanel uses direct listen() import - src/components/tasks/TaskChatPanel.tsx:9
- [x] [Frontend] Bypassed EventProvider: TaskBoard uses direct listen() import - src/components/tasks/TaskBoard/TaskBoard.tsx:22
- [x] [Frontend] Bypassed EventProvider: PermissionDialog uses direct listen() import - src/components/PermissionDialog.tsx:2
- [x] [Frontend] Bypassed EventProvider: IdeationView uses direct listen() import - src/components/Ideation/IdeationView.tsx:21
- [x] [Frontend] Bypassed EventProvider: useSupervisorAlerts.listener uses direct listen() import - src/hooks/useSupervisorAlerts.listener.ts:9
- [x] [Frontend] Bypassed EventProvider: useAskUserQuestion uses direct listen() import - src/hooks/useAskUserQuestion.ts:10
- [x] [Frontend] Bypassed EventProvider: useChatPanelHandlers uses direct listen() import - src/hooks/useChatPanelHandlers.ts:9

## From Phase 57 Verification (2026-02-01)

- [x] [Frontend] Missing mock API: all.list() method missing from mockActivityEventsApi - src/api-mock/activity-events.ts:16

## From Phase 59 Verification (2026-02-01)

- [x] [Frontend] Missing wiring: historicalStatus prop not passed to TaskChatPanel - chat filtering feature cannot be used from TaskDetailOverlay - src/components/tasks/TaskDetailOverlay.tsx:641
  - Fixed by adding StateTimelineNav, history mode state, and historicalStatus wiring to TaskFullView

## From Phase 66 Verification (2026-02-02)

- [x] [Frontend] Missing Implementation: useGitDiff hook doesn't call get_task_commits command - src/hooks/useGitDiff.ts:55
  - Fixed by adding getTaskCommits to diffApi and wiring in useGitDiff to fetch commits on mount

## From Phase 67 Verification (2026-02-03)

- [x] [Frontend] Orphaned Filter Logic: GraphControls allows filter selection but applyFilters() is never called - filters have no effect on graph - src/components/TaskGraph/TaskGraphView.tsx:348
  - Fixed by adding applyGraphFilters() and filteredGraphData useMemo in TaskGraphView

- [x] [Frontend] Orphaned Implementation: GraphLegend component created but never imported or rendered in TaskGraphView - src/components/TaskGraph/controls/GraphLegend.tsx:1
  - Fixed by adding import and render in TaskGraphView, positioned bottom-left inside ReactFlow canvas

## From Phase 79 Verification (2026-02-04)

- [x] [Visual/Mock] Missing mock for getGitDefaultBranch - prevents web mode testing in ProjectCreationWizard and GitSettingsSection - src/api-mock/projects.ts
  - Fixed by adding mockGetGitDefaultBranch function and command handler in tauri-api-core.ts
- [x] [Visual/Mock] Mock update method missing worktreeParentDirectory handler - changes don't persist in web mode - src/api-mock/projects.ts:43-60
  - Fixed by adding worktreeParentDirectory to mock update() method

---

**Migrated from:** logs/code-quality.md (2026-01-28)
**P0 items:** 10 completed (moved to archive)
**Last maintenance:** 2026-01-30 (archived 1 item)
