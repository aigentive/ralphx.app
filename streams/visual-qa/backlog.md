# Visual QA Backlog

> Work queue populated from manifest.md or new PRD components.
> Bootstrap phase uses manifest.md directly; this backlog is for maintenance phase.

## Uncovered Components

<!-- Populated after bootstrap phase completes or when new components are added -->

## Mock Parity Issues

<!-- Items discovered during testing where mock data is missing or incomplete -->

- [ ] TaskDetailModal: No UI trigger exists in web mode - modal requires programmatic opening via uiStore.openModal() but lacks natural entry point (right-click menu, button, etc.). Needs either: (1) UI trigger implementation, or (2) test-only helper to expose store manipulation
- [ ] AskUserQuestionModal: useAskUserQuestion hook not subscribing to events (listener count = 0 after app mount). Recent changes to App.tsx added the hook call, but requires dev server restart/rebuild. Files created: tests/pages/modals/ask-user-question.page.ts, tests/helpers/ask-user-question.helpers.ts, tests/visual/modals/ask-user-question/ask-user-question.spec.ts. EventBus exposure to window added to EventProvider.tsx.
