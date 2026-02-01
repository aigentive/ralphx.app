# Visual Coverage Manifest

## Bootstrap Status
Phase: IN PROGRESS (discovered 10 new components)

## Views (6 total)
| View | Mock Ready | Spec File | Baseline | Status |
|------|------------|-----------|----------|--------|
| kanban | ✅ | kanban.spec.ts | ✅ | covered |
| ideation | ✅ | ideation.spec.ts | ✅ | covered |
| activity | ✅ | activity.spec.ts | ✅ | covered |
| settings | ✅ | settings.spec.ts | ✅ | covered |
| extensibility | ✅ | extensibility.spec.ts | ✅ | covered |
| task_detail | ✅ | task-detail.spec.ts | ✅ | covered |

## Modals (16 total)
| Modal | Mock Ready | Spec File | Baseline | Status |
|-------|------------|-----------|----------|--------|
| TaskDetailModal | ✅ | task-detail-modal.spec.ts | ✅ | covered |
| ReviewsPanel | ✅ | reviews-panel.spec.ts | ✅ | covered |
| AskUserQuestionModal | ✅ | ask-user-question.spec.ts | ✅ | covered |
| ProjectCreationWizard | ✅ | project-creation-wizard.spec.ts | ✅ | covered |
| ProposalEditModal | ✅ | proposal-edit.spec.ts | ✅ | covered |
| PermissionDialog | 🚧 | — | — | blocked |
| WelcomeScreen | ✅ | welcome-screen.spec.ts | ✅ | covered |
| MergeWorkflowDialog | — | — | — | uncovered |
| ApplyModal | — | — | — | uncovered |
| ReviewDetailModal | — | — | — | uncovered |
| ReviewNotesModal | — | — | — | uncovered |
| TaskRerunDialog | — | — | — | uncovered |
| TaskFullView | — | — | — | uncovered |
| BlockReasonDialog | — | — | — | uncovered |
| TaskPickerDialog | — | — | — | uncovered |
| ScreenshotGallery | — | — | — | uncovered |

## States & Edge Cases
| State | View/Modal | Spec | Status |
|-------|------------|------|--------|
| empty-kanban | kanban | empty-kanban.spec.ts | covered |
| all-status-columns | kanban | all-status-columns.spec.ts | covered |
| loading-state | kanban | loading-state.spec.ts | covered |
| error-state | various | error-state.spec.ts | covered |
