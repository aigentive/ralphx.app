# Fix Merge Workflow Bugs

## Context

Two bugs in the merge workflow discovered via task `82a21731`:

1. **Bug 3 (MergeIncomplete agent mismatch):** Error path in `attempt_programmatic_merge` transitions to `MergeIncomplete` then spawns a merger agent. But `complete_merge` and `attempt_merge_auto_complete` only accept `Merging` status. Agent does work but can't signal completion.

2. **Bug 2 (Dirty tree blocks worktree merge):** `try_rebase_and_merge` uses `git rebase` which requires a clean working tree. In worktree mode, the main repo may have unrelated unstaged changes (user edits). Rebase fails even though changes are irrelevant to the task.

## Implementation Tasks

### Task 1: Remove agent auto-spawn from MergeIncomplete error path
**Dependencies:** None
**Atomic Commit:** `fix(merge): remove agent auto-spawn from MergeIncomplete error path`

`MergeIncomplete` should be a pure human-waiting state. The user clicks "Retry" in the UI which transitions to `merging`, where the agent spawns correctly and both `complete_merge` and auto-detection work.

**Files:**
- `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` (lines 1119-1145)

**Changes:**
- Delete the agent spawn block after the `MergeIncomplete` transition (lines 1119-1145)
- Keep the transition to `MergeIncomplete` (lines 1100-1117) and the status history recording
- Keep the `tracing::error!` log (lines 1090-1098) so the error is still visible
- Add an emit of `task:status_changed` event so the UI updates to show `merge_incomplete` with the Retry button

**Verification:**
- `cargo check` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo test` passes

### Task 2: Add `try_merge` method to GitService for worktree mode (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(git-service): add try_merge for worktree mode programmatic merge`

Add a new method that uses `git merge` (no rebase) so unstaged changes in the main repo don't block the merge.

**Files:**
- `src-tauri/src/application/git_service.rs`

**Changes:**
- Add `pub fn try_merge(repo: &Path, task_branch: &str, base: &str) -> AppResult<MergeAttemptResult>` after `try_rebase_and_merge` (~line 841)
- Implementation:
  1. Fetch origin (non-fatal, same as `try_rebase_and_merge`)
  2. Checkout base branch
  3. `git merge {task_branch} --no-edit`
  4. `Success`/`FastForward` -> `MergeAttemptResult::Success { commit_sha }`
  5. `Conflict` -> abort merge, return `MergeAttemptResult::NeedsAgent { conflict_files }`
- No rebase step — `git merge` doesn't care about unstaged changes in unrelated files
- Add test(s) for the new method

**Verification:**
- `cargo check` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo test` passes (including new tests)

### Task 3: Use `try_merge` in worktree mode programmatic merge
**Dependencies:** Task 2
**Atomic Commit:** `fix(merge): use git merge instead of rebase for worktree mode`

Branch on `git_mode` in `attempt_programmatic_merge` to call `try_merge` for worktree mode and keep `try_rebase_and_merge` for local mode.

**Files:**
- `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` (line ~939)

**Changes:**
- Replace:
  ```rust
  match GitService::try_rebase_and_merge(repo_path, &source_branch, &target_branch) {
  ```
  With:
  ```rust
  let merge_result = if project.git_mode == GitMode::Worktree {
      GitService::try_merge(repo_path, &source_branch, &target_branch)
  } else {
      GitService::try_rebase_and_merge(repo_path, &source_branch, &target_branch)
  };
  match merge_result {
  ```
- No other changes needed — the match arms handle the same `MergeAttemptResult` variants

**Tradeoff:** Worktree merges produce merge commits instead of rebased linear history. This is acceptable — worktree tasks are isolated.

**Verification:**
- `cargo check` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo test` passes

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
