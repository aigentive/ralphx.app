# RalphX - Phase 98: Fix Merge Workflow Bugs

## Overview

Two bugs in the merge workflow cause tasks to get stuck after programmatic merge failures. (1) The `MergeIncomplete` error path auto-spawns a merger agent, but both `complete_merge` and `attempt_merge_auto_complete` only accept `Merging` status — the agent works but can't signal success. (2) In worktree mode, `try_rebase_and_merge` uses `git rebase` which fails when the main repo has unrelated unstaged changes.

**Reference Plan:**
- `specs/plans/fix_merge_workflow_bugs.md` - Root cause analysis and implementation details for both bugs

## Goals

1. Make `MergeIncomplete` a pure human-waiting state (no agent auto-spawn)
2. Use `git merge` instead of `git rebase` for worktree mode programmatic merges
3. Ensure worktree-mode merges are not blocked by unrelated dirty files in the main repo

## Dependencies

### Phase 97 (Plan Branch Session-Based Lookup) - Required

| Dependency | Why Needed |
|------------|------------|
| `resolve_merge_branches` uses `session_id` | Phase 96-97 fixed the lookup; this phase modifies the same `attempt_programmatic_merge` function |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_merge_workflow_bugs.md`
2. Understand the merge workflow architecture (see `.claude/rules/task-execution-git-workflows.md`)
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
- Tasks 1 and 2 have no dependencies — can start immediately
- Task 3 depends on Task 2
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_merge_workflow_bugs.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Remove agent auto-spawn from MergeIncomplete error path — make it a pure human-waiting state",
    "plan_section": "Task 1: Remove agent auto-spawn from MergeIncomplete error path",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(merge): remove agent auto-spawn from MergeIncomplete error path",
    "steps": [
      "Read specs/plans/fix_merge_workflow_bugs.md section 'Task 1'",
      "In side_effects.rs, delete the agent spawn block (lines 1119-1145) after the MergeIncomplete transition",
      "Keep the transition to MergeIncomplete (lines 1100-1117) and status history recording",
      "Keep the tracing::error! log (lines 1090-1098)",
      "Add emit of task:status_changed event so the UI updates to show merge_incomplete with Retry button",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(merge): remove agent auto-spawn from MergeIncomplete error path"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add try_merge method to GitService that uses git merge (no rebase) for worktree mode",
    "plan_section": "Task 2: Add try_merge method to GitService for worktree mode",
    "blocking": [3],
    "blockedBy": [],
    "atomic_commit": "feat(git-service): add try_merge for worktree mode programmatic merge",
    "steps": [
      "Read specs/plans/fix_merge_workflow_bugs.md section 'Task 2'",
      "Add pub fn try_merge(repo, task_branch, base) -> AppResult<MergeAttemptResult> after try_rebase_and_merge (~line 841)",
      "Implementation: fetch origin (non-fatal), checkout base, git merge task_branch --no-edit, handle Success/Conflict",
      "Add test(s) for the new method (similar to existing try_rebase_and_merge tests)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(git-service): add try_merge for worktree mode programmatic merge"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Use try_merge in worktree mode, keep try_rebase_and_merge for local mode",
    "plan_section": "Task 3: Use try_merge in worktree mode programmatic merge",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "fix(merge): use git merge instead of rebase for worktree mode",
    "steps": [
      "Read specs/plans/fix_merge_workflow_bugs.md section 'Task 3'",
      "In side_effects.rs (~line 939), replace direct try_rebase_and_merge call with git_mode branch",
      "Worktree mode: call GitService::try_merge, Local mode: call GitService::try_rebase_and_merge",
      "Match arms remain unchanged — both return MergeAttemptResult",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(merge): use git merge instead of rebase for worktree mode"
    ],
    "passes": false
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Remove agent spawn from MergeIncomplete** | `MergeIncomplete` was designed as a human-waiting state ("Retry" button). Spawning an agent into it creates a status mismatch where neither `complete_merge` nor auto-detection can transition the task |
| **Use git merge (not rebase) for worktree mode** | `git rebase` requires a clean working tree. In worktree mode, the main repo may have unrelated user edits. `git merge` doesn't have this restriction. Tradeoff: merge commits instead of linear history — acceptable for isolated worktree tasks |
| **Keep rebase for local mode** | In local mode there's only one working tree — linear history matters more and dirty tree detection is already a guard condition |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] Existing `try_rebase_and_merge` tests still pass
- [ ] New `try_merge` tests pass
- [ ] State machine transition tests pass

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### Manual Testing
- [ ] In worktree mode with unstaged changes in main repo: task merge succeeds (programmatic path)
- [ ] In worktree mode with conflict: task transitions to `merging` and agent spawns
- [ ] Programmatic merge error transitions to `merge_incomplete` without spawning agent
- [ ] "Retry" button from `merge_incomplete` transitions to `merging` and spawns agent correctly
- [ ] In local mode: merge still uses rebase (behavior unchanged)

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `try_merge` method is called from `attempt_programmatic_merge` when `git_mode == Worktree`
- [ ] `MergeIncomplete` error path no longer spawns an agent
- [ ] `task:status_changed` event is emitted on MergeIncomplete transition
- [ ] UI Retry button still works (merge_incomplete -> merging transition)

See `.claude/rules/gap-verification.md` for full verification workflow.
