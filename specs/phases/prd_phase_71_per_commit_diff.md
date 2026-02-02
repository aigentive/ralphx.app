# RalphX - Phase 71: Per-Commit File Changes in Review Dialog

## Overview

In the ReviewDetailModal, the History tab shows commits correctly, but clicking on a commit shows "no files changed" because no backend API exists to get files changed in a specific commit or get file diff for a specific commit (vs its parent). The frontend handlers exist but don't fetch any data.

This phase adds per-commit diff capabilities to both backend and frontend, enabling users to view exactly what changed in each individual commit within the review dialog.

**Reference Plan:**
- `specs/plans/per_commit_file_changes_in_review_dialog.md` - Detailed implementation plan with code snippets

## Goals

1. Add backend commands to get files changed in a specific commit
2. Add backend commands to get file diff comparing a commit to its parent
3. Wire frontend to fetch and display per-commit file changes and diffs

## Dependencies

### Phase 70 (Fix get_task_file_changes Empty After Commits) - Required

| Dependency | Why Needed |
|------------|------------|
| Base branch comparison | Phase 70 fixed the overall file changes comparison; this phase builds on that for per-commit granularity |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/per_commit_file_changes_in_review_dialog.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/per_commit_file_changes_in_review_dialog.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add backend commands for per-commit file changes and diffs",
    "plan_section": "Task 1: Add backend commands for per-commit file changes",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(diff): add per-commit file changes and diff commands",
    "steps": [
      "Read specs/plans/per_commit_file_changes_in_review_dialog.md section 'Task 1'",
      "Add get_commit_file_changes method to diff_service.rs using git diff-tree or git show",
      "Add get_commit_file_diff method to diff_service.rs comparing commit to parent",
      "Handle edge cases: new files (no parent content), deleted files (no commit content)",
      "Add get_commit_file_changes Tauri command to diff_commands.rs",
      "Add get_commit_file_diff Tauri command to diff_commands.rs",
      "Register both commands in src-tauri/src/lib.rs invoke_handler",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(diff): add per-commit file changes and diff commands"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Add frontend API methods for per-commit diffs",
    "plan_section": "Task 2: Add frontend API methods",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(diff): add frontend API for per-commit diffs",
    "steps": [
      "Read specs/plans/per_commit_file_changes_in_review_dialog.md section 'Task 2'",
      "Add getCommitFileChanges method to src/api/diff.ts",
      "Add getCommitFileDiff method to src/api/diff.ts",
      "Use typedInvokeWithTransform with existing schemas (FileChangeSchema, FileDiffSchema)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(diff): add frontend API for per-commit diffs"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Wire frontend to use per-commit APIs",
    "plan_section": "Task 3: Wire frontend to use per-commit APIs",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "feat(diff): wire commit selection to fetch per-commit files and diffs",
    "steps": [
      "Read specs/plans/per_commit_file_changes_in_review_dialog.md section 'Task 3'",
      "Add commitFiles state and fetchCommitFiles function to useGitDiff.ts",
      "Update fetchDiff to accept optional commitSha parameter",
      "Add onFetchCommitFiles prop to DiffViewer.tsx and wire handleCommitSelect",
      "Wire handleCommitFileSelect to use commit-specific diff",
      "Update ReviewDetailModal.tsx to pass new props to DiffViewer",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(diff): wire commit selection to fetch per-commit files and diffs"
    ],
    "passes": false
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
| **Use git diff-tree for commit files** | Efficient command to get files changed in a single commit with status |
| **Compare commit to parent (^)** | Standard git approach to show what a specific commit changed |
| **Reuse existing FileChange/FileDiff types** | Per-commit changes have same structure as overall changes |
| **Optional commitSha parameter** | Allows fetchDiff to work for both overall and per-commit diffs |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] get_commit_file_changes returns files for valid commit
- [ ] get_commit_file_diff returns correct old/new content
- [ ] Handles new files (no parent content) gracefully
- [ ] Handles deleted files (no commit content) gracefully

### Frontend - Run `npm run test`
- [ ] diffApi.getCommitFileChanges calls correct command
- [ ] diffApi.getCommitFileDiff calls correct command with all params

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Open ReviewDetailModal for a task with multiple commits
- [ ] Click on a commit in History tab - file list populates
- [ ] Click on a file in commit - diff shows that commit's changes
- [ ] Switch to different commit - file list updates correctly
- [ ] Click on file in new commit - shows correct diff

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (commit click in History tab)
- [ ] DiffViewer receives onFetchCommitFiles callback
- [ ] Callback triggers API call to backend
- [ ] Backend returns file list, displayed in UI
- [ ] File click triggers commit-specific diff fetch

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
