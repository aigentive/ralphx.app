# Fix: get_task_file_changes not worktree-aware

## Problem

`get_task_file_changes` returns empty results for worktree-mode projects because it uses the project's `working_directory` instead of the task's `worktree_path`.

**Root cause:** The command receives `project_path` as a parameter from frontend, which is always `project.working_directory`. In worktree mode, files are in `task.worktree_path`.

**Evidence:**
- `get_task_commits` (git_commands.rs:100-107) correctly handles worktrees
- `get_task_file_changes` (diff_commands.rs:13-21) ignores worktrees

## Solution

Make `get_task_file_changes` and `get_file_diff` worktree-aware by:
1. Removing `project_path` parameter
2. Looking up task and project from database
3. Determining working path based on `git_mode` (same pattern as `get_task_commits`)

## Implementation

### Task 1: Make backend diff commands worktree-aware (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(diff): make get_task_file_changes and get_file_diff worktree-aware`

**Files:**
- `src-tauri/src/commands/diff_commands.rs`
- `src-tauri/src/application/diff_service.rs` (if method signatures need updating)

**Why merged:** All backend changes form a single compilation unit. Changing `get_task_file_changes` signature without also changing `get_file_diff` leaves an inconsistent API.

**Changes to `get_task_file_changes`:**
```rust
// Before:
pub async fn get_task_file_changes(
    app_state: State<'_, AppState>,
    task_id: String,
    project_path: String,
) -> AppResult<Vec<FileChange>>

// After:
pub async fn get_task_file_changes(
    app_state: State<'_, AppState>,
    task_id: String,
) -> AppResult<Vec<FileChange>>
```

Add task/project lookup and working path determination:
```rust
let task = app_state.task_repo.get_by_id(&task_id).await?
    .ok_or_else(|| AppError::TaskNotFound(task_id.to_string()))?;

let project = app_state.project_repo.get_by_id(&task.project_id).await?
    .ok_or_else(|| AppError::ProjectNotFound(task.project_id.to_string()))?;

let working_path = match project.git_mode {
    GitMode::Worktree => task.worktree_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&project.working_directory)),
    GitMode::Local => PathBuf::from(&project.working_directory),
};
```

**Changes to `get_file_diff`:**
Same pattern - add `task_id` parameter, remove `project_path`, use same working path logic.

### Task 2: Update frontend API and all callers
**Dependencies:** Task 1
**Atomic Commit:** `fix(diff): update frontend API to match worktree-aware backend`

**Files:**
- `src/api/diff.ts` - Remove `projectPath` parameter from both functions
- `src/api/diff.schemas.ts` - Update if needed
- `src/hooks/useGitDiff.ts` - Remove `projectPath` from calls
- All callers of `diffApi.getTaskFileChanges` and `diffApi.getFileDiff`

**Why merged:** Changing API function signatures in TypeScript requires updating all callers in the same commit - otherwise TypeScript type checking fails.

**Before:**
```typescript
getTaskFileChanges: (taskId: string, projectPath: string) => ...
getFileDiff: (filePath: string, projectPath: string) => ...
```

**After:**
```typescript
getTaskFileChanges: (taskId: string) => ...
getFileDiff: (taskId: string, filePath: string) => ...
```

Search for all callers and remove `projectPath` argument, adding `taskId` where needed.

## Verification

1. Start app with worktree-mode project
2. Navigate to task detail view for task with commits
3. Verify file changes list populates correctly
4. Verify clicking a file shows the diff correctly

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
