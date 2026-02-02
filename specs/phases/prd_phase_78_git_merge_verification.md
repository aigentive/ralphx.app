# RalphX - Phase 78: Git Merge Verification Fix

## Overview

This phase fixes a critical bug where the auto-merge completion logic incorrectly marks tasks as "merged" by capturing the commit SHA from the worktree (task branch) instead of verifying the merge actually happened on the main branch. This causes tasks to appear merged when no actual merge to main occurred.

The bug was discovered in ralphx-demo-3 where tasks showed as "merged" but files never appeared on the main branch.

**Reference Plan:**
- `specs/plans/git_merge_failure_fix.md` - Detailed analysis, root cause, and implementation plan

## Goals

1. Add verification that merges actually happen on the main branch before marking complete
2. Fix the auto-complete logic to use main repo HEAD, not worktree HEAD
3. Handle the "first task on empty repo" edge case where rebase fails
4. Add validation to the HTTP complete_merge endpoint

## Dependencies

### Phase 76 (Hybrid Merge Completion Detection) - Required

| Dependency | Why Needed |
|------------|------------|
| `attempt_merge_auto_complete()` | This is the function being fixed |
| Merge workflow | Understanding current merge state machine |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/git_merge_failure_fix.md`
2. Understand the bug root cause and verification approach
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
2. **Read the ENTIRE implementation plan** at `specs/plans/git_merge_failure_fix.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add is_commit_on_branch helper to GitService",
    "plan_section": "Step 2.1: Add helper to verify commit is on branch",
    "blocking": [2, 3],
    "blockedBy": [],
    "atomic_commit": "feat(git): add is_commit_on_branch helper",
    "steps": [
      "Read specs/plans/git_merge_failure_fix.md section 'Step 2.1'",
      "Add is_commit_on_branch() function to src-tauri/src/application/git_service.rs",
      "Function uses git merge-base --is-ancestor to check if commit is on branch",
      "Exit code 0 means commit is ancestor, 1 means not",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(git): add is_commit_on_branch helper"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Fix attempt_merge_auto_complete to verify merge on main",
    "plan_section": "Step 2.2: Fix attempt_merge_auto_complete() - verify merge on main",
    "blocking": [5],
    "blockedBy": [1],
    "atomic_commit": "fix(merge): verify merge happened on main before auto-completing",
    "steps": [
      "Read specs/plans/git_merge_failure_fix.md section 'Step 2.2'",
      "In chat_service_send_background.rs, find attempt_merge_auto_complete() around line 780",
      "Change: Get HEAD SHA from main repo path, not worktree",
      "Add: Verify task branch commit is merged into main using is_commit_on_branch()",
      "If not merged: transition to MergeConflict instead of Merged",
      "Use main branch HEAD as the merge commit SHA",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(merge): verify merge happened on main before auto-completing"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add verification to complete_merge HTTP handler",
    "plan_section": "Step 2.3: Add verification to complete_merge HTTP handler",
    "blocking": [5],
    "blockedBy": [1],
    "atomic_commit": "fix(http): verify commit is on main branch in complete_merge handler",
    "steps": [
      "Read specs/plans/git_merge_failure_fix.md section 'Step 2.3'",
      "In http_server/handlers/git.rs, find complete_merge handler",
      "After SHA format validation, add verification using is_commit_on_branch()",
      "Return BAD_REQUEST if commit is not on base branch",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(http): verify commit is on main branch in complete_merge handler"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Handle first task on empty repo case",
    "plan_section": "Step 2.4: Handle first task on empty repo case",
    "blocking": [5],
    "blockedBy": [],
    "atomic_commit": "fix(merge): handle first task on empty repo without rebase",
    "steps": [
      "Read specs/plans/git_merge_failure_fix.md section 'Step 2.4'",
      "In side_effects.rs, find try_rebase_and_merge()",
      "Add get_commit_count() helper to GitService if not exists",
      "Check if base branch has <= 1 commits (empty or initial commit only)",
      "If so: skip rebase, directly checkout base and merge task branch",
      "Otherwise: proceed with normal rebase then merge flow",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(merge): handle first task on empty repo without rebase"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Add merge verification tests",
    "plan_section": "Part 3: Add Tests",
    "blocking": [],
    "blockedBy": [2, 3, 4],
    "atomic_commit": "test(merge): add merge verification tests",
    "steps": [
      "Read specs/plans/git_merge_failure_fix.md section 'Part 3: Add Tests'",
      "Create tests for is_commit_on_branch() helper",
      "Test: commit on branch returns true",
      "Test: commit not on branch returns false",
      "Test: merge auto-complete fails when commit not on main",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: test(merge): add merge verification tests"
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
| **Verify on main repo, not worktree** | Worktree contains task branch, not main branch - must check actual main repo |
| **Use git merge-base --is-ancestor** | Standard git command for ancestry check, exit codes are reliable |
| **Handle empty repo as special case** | First task has no common history with main - rebase cannot work |
| **Fail to MergeConflict state** | Allows retry via human intervention rather than silent false success |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] is_commit_on_branch() returns true for merged commits
- [ ] is_commit_on_branch() returns false for unmerged commits
- [ ] attempt_merge_auto_complete() transitions to MergeConflict when not merged
- [ ] complete_merge handler rejects commits not on main

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Build succeeds (`cargo build --release`)

### Manual Testing
- [ ] Create new project with empty initial commit
- [ ] Create first task that adds files
- [ ] Execute task through full workflow
- [ ] Verify files appear on main branch after merge
- [ ] Verify task branch is deleted after successful merge
- [ ] Verify merge_commit_sha is actually on main branch

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] is_commit_on_branch() is called from attempt_merge_auto_complete()
- [ ] is_commit_on_branch() is called from complete_merge HTTP handler
- [ ] MergeConflict transition is properly triggered on verification failure

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
