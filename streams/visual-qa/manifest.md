# Visual Coverage Manifest

## Bootstrap Status
Phase: IN_PROGRESS

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
| AskUserQuestionModal | ❌ | — | — | uncovered |
| ProjectCreationWizard | ✅ | project-creation-wizard.spec.ts | ✅ | covered |
| ProposalEditModal | ❌ | — | — | uncovered |
| PermissionDialog | ❌ | — | — | uncovered |

## States & Edge Cases
| State | View/Modal | Spec | Status |
|-------|------------|------|--------|
| empty-kanban | kanban | — | uncovered |
| all-status-columns | kanban | — | uncovered |
| loading-state | various | — | uncovered |
| error-state | various | — | uncovered |
