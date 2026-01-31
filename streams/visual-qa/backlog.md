# Visual QA Backlog

> Work queue populated from manifest.md or new PRD components.
> Bootstrap phase uses manifest.md directly; this backlog is for maintenance phase.

## Uncovered Components

<!-- Populated after bootstrap phase completes or when new components are added -->

## Mock Parity Issues

<!-- Items discovered during testing where mock data is missing or incomplete -->

- [ ] TaskDetailModal: No UI trigger exists in web mode - modal requires programmatic opening via uiStore.openModal() but lacks natural entry point (right-click menu, button, etc.). Needs either: (1) UI trigger implementation, or (2) test-only helper to expose store manipulation
- [ ] AskUserQuestionModal: **BLOCKED - Dev server restart required**. All infrastructure complete: App.tsx line 165 hooks useAskUserQuestion, EventProvider.tsx exposes bus to window, modal renders at App.tsx:733, test files ready (tests/pages/modals/ask-user-question.page.ts, tests/helpers/ask-user-question.helpers.ts, tests/visual/modals/ask-user-question/ask-user-question.spec.ts). Tests fail with "modal not visible" timeout because window.__eventBus not available without restart. **User action needed: Restart dev server with `npm run dev:web`, then run: `npx playwright test tests/visual/modals/ask-user-question/ask-user-question.spec.ts --update-snapshots`**
- [ ] PermissionDialog: **BLOCKED - Dev server restart required**. All infrastructure complete: App.tsx line 760 renders PermissionDialog, EventProvider.tsx exposes bus to window (line 123), test files ready (tests/pages/modals/permission-dialog.page.ts, tests/helpers/permission.helpers.ts, tests/visual/modals/permission-dialog/permission-dialog.spec.ts). Tests fail with "modal not visible" timeout because window.__eventBus not available without restart. **User action needed: Restart dev server with `npm run dev:web`, then run: `npx playwright test tests/visual/modals/permission-dialog/permission-dialog.spec.ts --update-snapshots`**

## Bootstrap Queue (Prioritized)

<!-- Next components to cover, in order of readiness -->

1. **ProposalEditModal** - **BLOCKED - Mock data parity issue**. Modal component exists and test infrastructure ready (tests/pages/modals/proposal-edit.page.ts, tests/helpers/ideation.helpers.ts, tests/visual/modals/proposal-edit/proposal-edit.spec.ts). Issue: Mock ideation sessions (src/api-mock/ideation.ts:ensureMockData) create session + proposal, but sessions don't render in sidebar when navigating to ideation view. Root cause unknown - may be project ID mismatch or query key issue. Needs either: (1) Fix mock data loading in web mode, OR (2) Test-only helper to set editingProposalId state in App.tsx. Tests fail at loadMockIdeationSession helper - no sessions appear in sidebar.
