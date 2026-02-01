# RalphX - Phase 58: Wire TaskRerunDialog to CompletedTaskDetail

## Overview

This phase wires the orphaned `TaskRerunDialog` component to the `CompletedTaskDetail` view. Currently, clicking "Reopen Task" on a completed task immediately moves it back to Ready status without user confirmation. The `TaskRerunDialog` modal provides a proper UX for handling re-run workflow with three options: keep changes, revert commit, or create new task.

**Reference Plan:**
- `specs/plans/wire_task_rerun_dialog.md` - Detailed implementation steps for wiring the dialog

## Goals

1. Show TaskRerunDialog when user clicks "Reopen Task" on a completed task
2. Display commit information (sha, message) in the dialog
3. Handle all three rerun options (keep changes, revert, create new) with appropriate behavior

## Dependencies

### Phase 57 (Activity Page Global History & UX) - Required

| Dependency | Why Needed |
|------------|------------|
| Task state machine | Task status transitions used by rerun workflow |
| Completed task view | CompletedTaskDetail component where dialog is triggered |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/wire_task_rerun_dialog.md`
2. Understand the existing TaskRerunDialog component interface
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/wire_task_rerun_dialog.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Wire TaskRerunDialog to CompletedTaskDetail",
    "plan_section": "Task 1: Wire TaskRerunDialog to CompletedTaskDetail",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(tasks): wire TaskRerunDialog to CompletedTaskDetail",
    "steps": [
      "Read specs/plans/wire_task_rerun_dialog.md section 'Task 1'",
      "Import TaskRerunDialog and types from @/components/tasks/TaskRerunDialog",
      "Import useGitDiff hook from @/hooks/useGitDiff",
      "Add useState for isRerunDialogOpen, isProcessing, error",
      "Use useGitDiff(task.id) to get commit data for commitInfo prop",
      "Change handleReopenTask to open dialog instead of direct move",
      "Add handleRerunConfirm callback to handle all three options",
      "Render TaskRerunDialog at bottom of component",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tasks): wire TaskRerunDialog to CompletedTaskDetail"
    ],
    "passes": true
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start (inverse of blocking)
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Use mock commit data from useGitDiff** | useGitDiff already provides commit info (mocked), avoiding need for new backend endpoint |
| **All options move to ready for now** | Full revert/duplicate functionality is future work; current focus is wiring the dialog |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] TaskRerunDialog renders when Reopen Task clicked
- [ ] Dialog displays task title and commit info
- [ ] All three options are selectable

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Navigate to a completed task in TaskFullView
- [ ] Click "Reopen Task" - dialog appears
- [ ] Select "Keep changes" and confirm - task moves to Ready
- [ ] Cancel button closes dialog without action
- [ ] Loading state appears during processing

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified: "Reopen Task" button click in CompletedTaskDetail
- [ ] TaskRerunDialog is imported AND rendered (not behind disabled flag)
- [ ] Dialog opens on button click (not direct api.tasks.move)
- [ ] Confirm action moves task and closes dialog

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] Dialog actually renders (not just imported)
- [ ] handleReopenTask no longer directly moves task

See `.claude/rules/gap-verification.md` for full verification workflow.
