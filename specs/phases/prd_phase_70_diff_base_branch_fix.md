# RalphX - Phase 70: Fix get_task_file_changes Empty After Commits

## Overview

The "Review Code" dialog shows commits but no file changes for worktree-mode projects. This is caused by `DiffService::get_task_file_changes` using `git diff --numstat HEAD` which only shows **uncommitted** changes. After task work is committed, `git diff HEAD` returns nothing because there's nothing uncommitted.

This phase fixes the diff commands to compare against the **base branch** instead of HEAD, matching the pattern used by `get_task_commits` and `get_task_diff_stats`.

**Reference Plan:**
- `specs/plans/fix_get_task_file_changes_shows_empty_after_commits.md` - Detailed implementation plan with code changes

## Goals

1. Fix `get_task_file_changes` to show files changed relative to base branch
2. Fix `get_file_diff` to show old content from base branch (not HEAD)
3. Refactor `get_task_working_path` to return project for efficient base_branch access

## Dependencies

### Phase 69 (Fix Diff Commands Worktree Support) - Required

| Dependency | Why Needed |
|------------|------------|
| Worktree-aware diff commands | This phase builds on the worktree-aware architecture from Phase 69 |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_get_task_file_changes_shows_empty_after_commits.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_get_task_file_changes_shows_empty_after_commits.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add base_branch parameter to DiffService and update callers",
    "plan_section": "Task 1: Add base_branch parameter to DiffService and update callers",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(diff): compare against base branch instead of HEAD for file changes",
    "steps": [
      "Read specs/plans/fix_get_task_file_changes_shows_empty_after_commits.md section 'Task 1'",
      "Update get_task_file_changes signature to accept base_branch parameter",
      "Update get_file_change_status to accept base_branch and use it in git diff command",
      "Change ls-files check to ls-tree for checking if file existed in base branch",
      "Update get_file_diff to accept base_branch and use it in git show command",
      "Update get_task_working_path to return project tuple for base_branch access",
      "Update get_task_file_changes command to pass base_branch from project",
      "Update get_file_diff command to pass base_branch from project",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(diff): compare against base branch instead of HEAD for file changes"
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
| **Compare to base_branch instead of HEAD** | HEAD only shows uncommitted changes; base_branch shows all changes since branching |
| **Use ls-tree instead of ls-files** | ls-files only checks working tree; ls-tree checks specific commit/branch |
| **Return project from get_task_working_path** | Avoids duplicate task/project lookups when both working_path and base_branch needed |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] DiffService compiles with new base_branch parameters
- [ ] Diff commands compile with updated function calls

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Build succeeds (`cargo build --release`)

### Manual Testing
- [ ] Start app with worktree-mode project
- [ ] Navigate to task detail view for task with commits (approved/merged state)
- [ ] Click "Review Code" button
- [ ] Verify commits list shows (already works)
- [ ] Verify file changes list shows all files modified by those commits
- [ ] Click a file and verify diff shows correctly (old content from base, new from HEAD)

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (click handler, route, event listener)
- [ ] New component is imported AND rendered (not behind disabled flag)
- [ ] API wrappers call backend commands
- [ ] State changes reflect in UI

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
