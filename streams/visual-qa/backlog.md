# Visual QA Backlog

> Work queue populated from manifest.md or new PRD components.
> Bootstrap phase uses manifest.md directly; this backlog is for maintenance phase.

## Uncovered Components

<!-- Populated after bootstrap phase completes or when new components are added -->

## Mock Parity Issues

<!-- Items discovered during testing where mock data is missing or incomplete -->

- [ ] TaskDetailModal: No UI trigger exists in web mode - modal requires programmatic opening via uiStore.openModal() but lacks natural entry point (right-click menu, button, etc.). Needs either: (1) UI trigger implementation, or (2) test-only helper to expose store manipulation
- [ ] AskUserQuestionModal: **BLOCKED - Dev server restart required**. All infrastructure complete: App.tsx line 165 hooks useAskUserQuestion, EventProvider.tsx exposes bus to window, modal renders at App.tsx:733, test files ready (tests/pages/modals/ask-user-question.page.ts, tests/helpers/ask-user-question.helpers.ts, tests/visual/modals/ask-user-question/ask-user-question.spec.ts). Tests fail with "modal not visible" timeout because window.__eventBus not available without restart. **User action needed: Restart dev server with `npm run dev:web`, then run: `npx playwright test tests/visual/modals/ask-user-question/ask-user-question.spec.ts --update-snapshots`**
