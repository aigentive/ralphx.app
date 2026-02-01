# Visual QA Backlog

> Work queue populated from manifest.md or new PRD components.
> Bootstrap phase uses manifest.md directly; this backlog is for maintenance phase.

## Uncovered Components

<!-- Populated after bootstrap phase completes or when new components are added -->

### Priority 1 - Major Views/Modals
- [x] **WelcomeScreen** - Full-screen welcome/onboarding view with animated constellation
- [ ] ~~**MergeWorkflowDialog**~~ (no-trigger)
- [ ] ~~**ApplyModal**~~ (orphan - not used in production code, only in tests)
- [ ] **ReviewDetailModal** - Full-width review modal with diff viewer and history
- [ ] **ReviewNotesModal** - Modal for adding review notes with fix descriptions
- [ ] **TaskRerunDialog** - Task re-run workflow modal (keep/revert/create new)
- [ ] **TaskFullView** - Full-screen task view component

### Priority 2 - Secondary Dialogs
- [ ] **BlockReasonDialog** - Modal for capturing task block reason
- [ ] **TaskPickerDialog** - Select draft tasks for ideation
- [ ] **ScreenshotGallery** - Professional gallery with lightbox and comparison mode

## Mock Parity Issues

<!-- Items discovered during testing where mock data is missing or incomplete -->

- [x] TaskDetailModal: Resolved by creating test helper (tests/helpers/task-detail.helpers.ts) that uses window.__uiStore (already exposed for web mode). No production UI trigger needed for visual testing.
- [x] AskUserQuestionModal: Resolved by exposing uiStore to window for direct state manipulation in tests (avoiding event subscription race condition)
- [ ] PermissionDialog: **BLOCKED - Component subscription issue**. Server restarted, window.__eventBus available, but PermissionDialog useEffect never subscribes to permission:request events (listener count = 0 after app load). Root cause: Component mounts but useEffect subscription not working in web mode. Needs investigation: (1) useEventBus() error in web mode? (2) Component lifecycle issue? (3) EventProvider context not available? Test infrastructure ready: tests/pages/modals/permission-dialog.page.ts, tests/helpers/permission.helpers.ts, tests/visual/modals/permission-dialog/permission-dialog.spec.ts. **Reclassified as P1 technical debt** (non-blocking per stream rules).

## Bootstrap Queue (Prioritized)

<!-- Next components to cover, in order of readiness -->

- [x] **ProposalEditModal** - Completed. Spec, baseline, and helper updated. All 7 tests pass.
