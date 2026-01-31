# Visual Coverage Manifest

## Bootstrap Status
Phase: COMPLETE

## Views (6 total)
| View | Mock Ready | Spec File | Baseline | Status |
|------|------------|-----------|----------|--------|
| kanban | ✅ | kanban.spec.ts | ✅ | covered |
| ideation | ✅ | ideation.spec.ts | ✅ | covered |
| activity | ✅ | activity.spec.ts | ✅ | covered |
| settings | ✅ | settings.spec.ts | ✅ | covered |
| extensibility | ✅ | extensibility.spec.ts | ✅ | covered |
| task_detail | ✅ | task-detail.spec.ts | ✅ | covered |

## Modals (6 total)
| Modal | Mock Ready | Spec File | Baseline | Status |
|-------|------------|-----------|----------|--------|
| TaskDetailModal | 🚧 | — | — | blocked |
| ReviewsPanel | ✅ | reviews-panel.spec.ts | ✅ | covered |
| AskUserQuestionModal | ✅ | ask-user-question.spec.ts | ✅ | covered |
| ProjectCreationWizard | ✅ | project-creation-wizard.spec.ts | ✅ | covered |
| ProposalEditModal | 🚧 | proposal-edit.spec.ts | — | blocked |
| PermissionDialog | 🚧 | — | — | blocked |

## States & Edge Cases
| State | View/Modal | Spec | Status |
|-------|------------|------|--------|
| empty-kanban | kanban | empty-kanban.spec.ts | covered |
| all-status-columns | kanban | all-status-columns.spec.ts | covered |
| loading-state | kanban | loading-state.spec.ts | covered |
| error-state | various | error-state.spec.ts | covered |
