# Per-Commit File Changes in Review Dialog

## Problem

In the ReviewDetailModal, the History tab shows commits correctly, but clicking on a commit shows "no files changed" because:
1. No backend API exists to get files changed in a specific commit
2. No backend API exists to get file diff for a specific commit (vs its parent)
3. Frontend handlers exist but don't fetch any data

**User flow that's broken:**
1. Open ReviewDetailModal → works ✅
2. See "Changes" tab with all files → works ✅ (fixed in Phase 70)
3. See "History" tab with commits → works ✅
4. Click on a commit → shows empty file list ❌
5. Click on a file in commit → no diff ❌

## Solution

Add per-commit diff capabilities to both backend and frontend.

## Implementation

### Task 1: Add backend commands for per-commit file changes (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(diff): add per-commit file changes and diff commands`

**Files:**
- `src-tauri/src/application/diff_service.rs`
- `src-tauri/src/commands/diff_commands.rs`

**Changes to `diff_service.rs`:**

1. Add new method to get files changed in a specific commit:
```rust
/// Get files changed in a specific commit
pub fn get_commit_file_changes(
    &self,
    commit_sha: &str,
    project_path: &str,
) -> AppResult<Vec<FileChange>> {
    // Use: git diff-tree --no-commit-id --name-status -r {commit_sha}
    // Or: git show --name-status --format="" {commit_sha}
    // Returns: A/M/D status with file paths
}
```

2. Add new method to get file diff for a specific commit:
```rust
/// Get diff for a file in a specific commit (comparing to parent)
pub fn get_commit_file_diff(
    &self,
    commit_sha: &str,
    file_path: &str,
    project_path: &str,
) -> AppResult<FileDiff> {
    // Old content: git show {commit_sha}^:{file_path} (parent commit)
    // New content: git show {commit_sha}:{file_path} (this commit)
    // Handle new files (parent doesn't have file) and deleted files
}
```

**Changes to `diff_commands.rs`:**

1. Add new Tauri command:
```rust
#[tauri::command]
pub async fn get_commit_file_changes(
    app_state: State<'_, AppState>,
    task_id: String,
    commit_sha: String,
) -> AppResult<Vec<FileChange>> {
    let task_id = TaskId::from_string(task_id);
    let (_, working_path_str, _) = get_task_working_path(&app_state, &task_id).await?;

    let diff_service = DiffService::new(Arc::clone(&app_state.activity_event_repo));
    diff_service.get_commit_file_changes(&commit_sha, &working_path_str)
}
```

2. Add new Tauri command:
```rust
#[tauri::command]
pub async fn get_commit_file_diff(
    app_state: State<'_, AppState>,
    task_id: String,
    commit_sha: String,
    file_path: String,
) -> AppResult<FileDiff> {
    let task_id = TaskId::from_string(task_id);
    let (_, working_path_str, _) = get_task_working_path(&app_state, &task_id).await?;

    let diff_service = DiffService::new(Arc::clone(&app_state.activity_event_repo));
    diff_service.get_commit_file_diff(&commit_sha, &file_path, &working_path_str)
}
```

3. Register commands in `src-tauri/src/lib.rs` (invoke_handler)

### Task 2: Add frontend API methods
**Dependencies:** Task 1
**Atomic Commit:** `feat(diff): add frontend API for per-commit diffs`

**Files:**
- `src/api/diff.ts`
- `src/api/diff.schemas.ts` (if needed)

**Changes to `diff.ts`:**

```typescript
export const diffApi = {
  // ... existing methods ...

  /** Get files changed in a specific commit */
  getCommitFileChanges: async (taskId: string, commitSha: string): Promise<FileChange[]> => {
    return typedInvokeWithTransform(
      "get_commit_file_changes",
      { taskId, commitSha },
      z.array(FileChangeSchema),
      (files) => files.map(transformFileChange)
    );
  },

  /** Get diff for a file in a specific commit */
  getCommitFileDiff: async (taskId: string, commitSha: string, filePath: string): Promise<FileDiff> => {
    return typedInvokeWithTransform(
      "get_commit_file_diff",
      { taskId, commitSha, filePath },
      FileDiffSchema,
      transformFileDiff
    );
  },
} as const;
```

### Task 3: Wire frontend to use per-commit APIs
**Dependencies:** Task 2
**Atomic Commit:** `feat(diff): wire commit selection to fetch per-commit files and diffs`

**Files:**
- `src/hooks/useGitDiff.ts`
- `src/components/diff/DiffViewer.tsx`
- `src/components/reviews/ReviewDetailModal.tsx`

**Changes to `useGitDiff.ts`:**

1. Add state for commit-specific files:
```typescript
const [commitFiles, setCommitFiles] = useState<FileChange[]>([]);
const [selectedCommit, setSelectedCommit] = useState<Commit | null>(null);
```

2. Add function to fetch commit files:
```typescript
const fetchCommitFiles = useCallback(async (commitSha: string) => {
  if (!taskId) return;
  setLoading(true);
  try {
    const files = await diffApi.getCommitFileChanges(taskId, commitSha);
    setCommitFiles(files);
  } catch (err) {
    setError(err instanceof Error ? err.message : "Failed to fetch commit files");
  } finally {
    setLoading(false);
  }
}, [taskId]);
```

3. Update `fetchDiff` to handle commit-specific diffs:
```typescript
const fetchDiff = useCallback(async (filePath: string, commitSha?: string) => {
  if (!taskId) return;
  setLoading(true);
  try {
    const diff = commitSha
      ? await diffApi.getCommitFileDiff(taskId, commitSha, filePath)
      : await diffApi.getFileDiff(taskId, filePath);
    setDiffData(diff);
  } catch (err) {
    setError(err instanceof Error ? err.message : "Failed to fetch diff");
  } finally {
    setLoading(false);
  }
}, [taskId]);
```

**Changes to `DiffViewer.tsx`:**

1. Update `handleCommitSelect` to call the new fetch function:
```typescript
const handleCommitSelect = useCallback(async (commit: Commit) => {
  setSelectedCommit(commit);
  setCommitSelectedFile(null);
  setDiffData(null);
  onCommitSelect?.(commit);
  // Fetch files changed in this commit
  if (onFetchCommitFiles) {
    await onFetchCommitFiles(commit.sha);
  }
}, [onCommitSelect, onFetchCommitFiles]);
```

2. Add new prop `onFetchCommitFiles` to DiffViewerProps

3. Wire file selection in commit view to use commit-specific diff:
```typescript
const handleCommitFileSelect = useCallback(async (file: FileChange) => {
  setCommitSelectedFile(file);
  if (selectedCommit && onFetchDiff) {
    await onFetchDiff(file.path, selectedCommit.sha);
  }
}, [selectedCommit, onFetchDiff]);
```

**Changes to `ReviewDetailModal.tsx`:**

1. Pass new callbacks from useGitDiff to DiffViewer:
```typescript
<DiffViewer
  // ... existing props ...
  onFetchCommitFiles={fetchCommitFiles}
  commitFiles={commitFiles}
/>
```

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/application/diff_service.rs` | Add `get_commit_file_changes`, `get_commit_file_diff` |
| `src-tauri/src/commands/diff_commands.rs` | Add Tauri commands |
| `src-tauri/src/lib.rs` | Register new commands |
| `src/api/diff.ts` | Add `getCommitFileChanges`, `getCommitFileDiff` |
| `src/hooks/useGitDiff.ts` | Add commit file fetching, update fetchDiff |
| `src/components/diff/DiffViewer.tsx` | Wire commit select → fetch files |
| `src/components/diff/DiffViewer.types.tsx` | Add new prop types |
| `src/components/reviews/ReviewDetailModal.tsx` | Pass new props to DiffViewer |

## Verification

1. Start app with worktree-mode project that has multiple commits
2. Open a task in approved/merged state
3. Click "Review Code" to open ReviewDetailModal
4. Go to "History" tab → should see commits
5. Click on a commit → should see list of files changed in that commit
6. Click on a file → should see diff showing what changed in that specific commit
7. Click on different commit → file list should update
8. Click on file in new commit → diff should show that commit's changes

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
