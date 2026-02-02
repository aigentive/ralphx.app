# Fix: get_task_file_changes shows empty after commits

## Problem

"Review Code" dialog shows commits but no file changes for worktree-mode projects.

**Root cause:** `DiffService::get_task_file_changes` uses `git diff --numstat HEAD` which only shows **uncommitted** changes. After the task work is committed, `git diff HEAD` returns nothing because there's nothing uncommitted.

**Evidence:**
- `get_task_commits` uses `git log base..HEAD` → correctly shows commits ✅
- `get_task_diff_stats` uses `git diff --shortstat base` → correctly compares to base ✅
- `get_task_file_changes` uses `git diff --numstat HEAD` → only uncommitted changes ❌

**Location:** `src-tauri/src/application/diff_service.rs:135-139`
```rust
let output = Command::new("git")
    .args(["diff", "--numstat", "HEAD", "--", file_path])  // ← WRONG: compares to HEAD
    .current_dir(project_path)
    .output()
```

## Solution

Change `DiffService` to compare against **base branch** instead of HEAD, matching the pattern used by `get_task_commits` and `get_task_diff_stats`.

## Implementation

### Task 1: Add base_branch parameter to DiffService and update callers
**Dependencies:** None
**Atomic Commit:** `fix(diff): compare against base branch instead of HEAD for file changes`

> **Compilation Unit Note:** This task must include both the service signature changes AND the command updates because changing the function signatures without updating callers would break compilation.

**Files:**
- `src-tauri/src/application/diff_service.rs`
- `src-tauri/src/commands/diff_commands.rs`

**Changes to `diff_service.rs`:**

1. Update `get_task_file_changes` signature to accept `base_branch`:
```rust
pub async fn get_task_file_changes(
    &self,
    task_id: &TaskId,
    project_path: &str,
    base_branch: &str,  // NEW
) -> AppResult<Vec<FileChange>>
```

2. Update call to `get_file_change_status` (line ~121):
```rust
if let Some(change) = self.get_file_change_status(&file_path, project_path, base_branch) {
```

3. Update `get_file_change_status` signature:
```rust
fn get_file_change_status(
    &self,
    file_path: &str,
    project_path: &str,
    base_branch: &str,  // NEW
) -> Option<FileChange>
```

4. Update git commands in `get_file_change_status` to use `base_branch`:
```rust
// Line ~136: diff for tracked changes
.args(["diff", "--numstat", base_branch, "--", file_path])

// Line ~151: check if file existed in base (use ls-tree instead of ls-files)
.args(["ls-tree", "-r", "--name-only", base_branch, "--", file_path])

// Line ~179: untracked files check remains the same (new files)
```

5. Update `get_file_diff` signature and git show command (line ~211):
```rust
pub fn get_file_diff(
    &self,
    file_path: &str,
    project_path: &str,
    base_branch: &str,  // NEW
) -> AppResult<FileDiff>

// Change git show command:
.args(["show", &format!("{}:{}", base_branch, file_path)])
```

**Changes to `diff_commands.rs`:**

1. Update `get_task_working_path` to also return the project:
```rust
async fn get_task_working_path(
    app_state: &AppState,
    task_id: &TaskId,
) -> AppResult<(PathBuf, String, crate::domain::entities::Project)>
```

2. Update `get_task_file_changes` command:
```rust
#[tauri::command]
pub async fn get_task_file_changes(
    app_state: State<'_, AppState>,
    task_id: String,
) -> AppResult<Vec<FileChange>> {
    let task_id = TaskId::from_string(task_id);

    // Get the correct working path and project for this task
    let (_, working_path_str, project) = get_task_working_path(&app_state, &task_id).await?;
    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    let diff_service = DiffService::new(Arc::clone(&app_state.activity_event_repo));
    diff_service
        .get_task_file_changes(&task_id, &working_path_str, base_branch)
        .await
}
```

3. Update `get_file_diff` command:
```rust
#[tauri::command]
pub async fn get_file_diff(
    app_state: State<'_, AppState>,
    task_id: String,
    file_path: String,
) -> AppResult<FileDiff> {
    let task_id = TaskId::from_string(task_id);

    // Get the correct working path and project for this task
    let (_, working_path_str, project) = get_task_working_path(&app_state, &task_id).await?;
    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    let diff_service = DiffService::new(Arc::clone(&app_state.activity_event_repo));
    diff_service.get_file_diff(&file_path, &working_path_str, base_branch)
}

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/application/diff_service.rs` | Add `base_branch` param, change git commands |
| `src-tauri/src/commands/diff_commands.rs` | Pass `base_branch` from project to DiffService |

## Verification

1. Start app with worktree-mode project
2. Navigate to task detail view for task with commits (approved/merged state)
3. Click "Review Code" button
4. Verify commits list shows (already works)
5. Verify file changes list shows all files modified by those commits
6. Click a file and verify diff shows correctly (old content from base, new from HEAD)

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
