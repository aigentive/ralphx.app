# RalphX - Phase 69: Fix Diff Commands Worktree Support

## Overview

This phase fixes a bug where `get_task_file_changes` and `get_file_diff` commands return empty results for worktree-mode projects. The root cause is that these commands receive `project_path` from the frontend (always `project.working_directory`), but in worktree mode, the files are in `task.worktree_path`.

The fix follows the same pattern as `get_task_commits` (git_commands.rs:100-107) which correctly handles worktrees.

**Reference Plan:**
- `specs/plans/fix_get_task_file_changes_worktree.md` - Detailed implementation plan with code samples

## Goals

1. Make `get_task_file_changes` worktree-aware (lookup task/project, use correct working path)
2. Make `get_file_diff` worktree-aware (same pattern)
3. Update frontend API to remove `projectPath` parameter (backend determines path internally)

## Dependencies

### Phase 66 (Per-Task Git Branch Isolation) - Required

| Dependency | Why Needed |
|------------|------------|
| Worktree infrastructure | This fix addresses bugs in worktree-mode projects |
| Task worktree_path field | Used to determine correct working directory |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_get_task_file_changes_worktree.md`
2. Understand the working path determination logic
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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_get_task_file_changes_worktree.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Make get_task_file_changes and get_file_diff worktree-aware",
    "plan_section": "Task 1: Make backend diff commands worktree-aware",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "fix(diff): make get_task_file_changes and get_file_diff worktree-aware",
    "steps": [
      "Read specs/plans/fix_get_task_file_changes_worktree.md section 'Task 1'",
      "Update get_task_file_changes: remove project_path param, add task/project lookup",
      "Add working path determination logic (GitMode::Worktree vs GitMode::Local)",
      "Update get_file_diff: add task_id param, remove project_path, use same logic",
      "Update DiffService method signatures if needed",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(diff): make get_task_file_changes and get_file_diff worktree-aware"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Update frontend API and all callers to match worktree-aware backend",
    "plan_section": "Task 2: Update frontend API and all callers",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "fix(diff): update frontend API to match worktree-aware backend",
    "steps": [
      "Read specs/plans/fix_get_task_file_changes_worktree.md section 'Task 2'",
      "Update src/api/diff.ts: remove projectPath from getTaskFileChanges, add taskId to getFileDiff",
      "Update src/api/diff.schemas.ts if needed",
      "Update src/hooks/useGitDiff.ts: remove projectPath from calls",
      "Search for all callers of diffApi.getTaskFileChanges and diffApi.getFileDiff",
      "Update all callers to match new signatures",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(diff): update frontend API to match worktree-aware backend"
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
| **Remove project_path param** | Backend determines path from task/project - eliminates frontend error |
| **Same pattern as get_task_commits** | Proven working approach in git_commands.rs |
| **Add task_id to get_file_diff** | Required to look up task.worktree_path |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] get_task_file_changes compiles without project_path param
- [ ] get_file_diff compiles with task_id param

### Frontend - Run `npm run test`
- [ ] diffApi calls use new signatures
- [ ] No TypeScript errors

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Start app with worktree-mode project
- [ ] Navigate to task detail view for task with commits
- [ ] Verify file changes list populates correctly (not empty)
- [ ] Click a file and verify diff displays correctly

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Backend command uses task.worktree_path when GitMode::Worktree
- [ ] Frontend calls match backend signatures
- [ ] File changes display in UI for worktree-mode tasks

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
