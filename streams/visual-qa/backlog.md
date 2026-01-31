# Visual QA Backlog

> Work queue populated from manifest.md or new PRD components.
> Bootstrap phase uses manifest.md directly; this backlog is for maintenance phase.

## Uncovered Components

<!-- Populated after bootstrap phase completes or when new components are added -->

## Mock Parity Issues

<!-- Items discovered during testing where mock data is missing or incomplete -->

- [x] TaskDetailModal: Resolved by creating test helper (tests/helpers/task-detail.helpers.ts) that uses window.__uiStore (already exposed for web mode). No production UI trigger needed for visual testing.
- [x] AskUserQuestionModal: Resolved by exposing uiStore to window for direct state manipulation in tests (avoiding event subscription race condition)
- [ ] PermissionDialog: **BLOCKED - Component subscription issue**. Server restarted, window.__eventBus available, but PermissionDialog useEffect never subscribes to permission:request events (listener count = 0 after app load). Root cause: Component mounts but useEffect subscription not working in web mode. Needs investigation: (1) useEventBus() error in web mode? (2) Component lifecycle issue? (3) EventProvider context not available? Test infrastructure ready: tests/pages/modals/permission-dialog.page.ts, tests/helpers/permission.helpers.ts, tests/visual/modals/permission-dialog/permission-dialog.spec.ts. **Reclassified as P1 technical debt** (non-blocking per stream rules).

## Bootstrap Queue (Prioritized)

<!-- Next components to cover, in order of readiness -->

1. **ProposalEditModal** - **BLOCKED - Mock data parity issue**. Modal component exists and test infrastructure ready (tests/pages/modals/proposal-edit.page.ts, tests/helpers/ideation.helpers.ts, tests/visual/modals/proposal-edit/proposal-edit.spec.ts). Issue: Mock ideation sessions (src/api-mock/ideation.ts:ensureMockData) create session + proposal, but sessions don't render in sidebar when navigating to ideation view. Root cause unknown - may be project ID mismatch or query key issue. Needs either: (1) Fix mock data loading in web mode, OR (2) Test-only helper to set editingProposalId state in App.tsx. Tests fail at loadMockIdeationSession helper - no sessions appear in sidebar.
