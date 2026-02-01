# Wire TaskRerunDialog to CompletedTaskDetail

## Context

**Orphan Component:** `src/components/tasks/TaskRerunDialog/TaskRerunDialog.tsx`

The `TaskRerunDialog` is fully implemented but never used. It provides a modal for handling re-run workflow when a completed task is reopened, offering 3 options:
- Keep changes (recommended) — AI sees current state
- Revert commit — Undo the original work
- Create new task — Keep original completed, spawn new

**Current Behavior:** `CompletedTaskDetail.tsx:145-152` directly moves task via `api.tasks.move(task.id, "ready")` without showing the dialog.

**Required Props:**
- `task: Task`
- `commitInfo: { sha, message, hasDependentCommits }`
- `onConfirm: (result: TaskRerunResult) => void`
- `onClose: () => void`
- `isOpen: boolean`

## Implementation Tasks

### Task 1: Wire TaskRerunDialog to CompletedTaskDetail
**Dependencies:** None
**Atomic Commit:** `feat(tasks): wire TaskRerunDialog to CompletedTaskDetail`

**Files to modify:**
- `src/components/tasks/detail-views/CompletedTaskDetail.tsx`

**Changes:**
1. Import `TaskRerunDialog` and its types from `@/components/tasks/TaskRerunDialog`
2. Import `useGitDiff` hook from `@/hooks/useGitDiff`
3. Add `useState` for `isRerunDialogOpen`
4. Add `useState` for `isProcessing` and `error`
5. Use `useGitDiff(task.id)` to get commit data
6. Change `handleReopenTask` to open dialog instead of direct move
7. Add `handleRerunConfirm` callback that:
   - Sets `isProcessing` to true
   - Based on `result.option`:
     - `keep_changes`: Move task to ready
     - `revert_commit`: (Future) Call revert API, then move
     - `create_new`: (Future) Call create duplicate API
   - Invalidates queries
   - Closes dialog
8. Render `TaskRerunDialog` at bottom of component

**Mock commitInfo for now:**
```typescript
const latestCommit = commits[0];
const commitInfo = {
  sha: latestCommit?.shortSha ?? "unknown",
  message: latestCommit?.message ?? "No commit info available",
  hasDependentCommits: commits.length > 1,
};
```

## Acceptance Criteria

- [ ] "Reopen Task" button opens TaskRerunDialog instead of immediately moving task
- [ ] Dialog shows task title and commit info
- [ ] "Keep changes" option moves task to ready status
- [ ] "Revert commit" option shows warning but moves task (full revert is future work)
- [ ] "Create new" option moves task (full duplicate is future work)
- [ ] Dialog can be cancelled
- [ ] Loading state shown during processing
- [ ] Errors displayed in dialog

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
