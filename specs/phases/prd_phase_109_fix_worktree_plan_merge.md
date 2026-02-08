# RalphX - Phase 109: Fix Worktree-Mode Plan Merge Base Branch

## Overview

In worktree mode, the auto-created "Merge plan into main" task fails with `fatal: 'main' is already used by worktree`. The root cause is that `attempt_programmatic_merge()` always uses `try_merge_in_worktree()` in worktree mode, which creates a temporary worktree checking out the target branch. For plan merge tasks (plan feature branch → main), the target is `main`, which is already checked out in the primary repo. Git forbids the same branch in multiple worktrees.

Regular task merges (task → plan branch) work fine because plan branches aren't checked out anywhere. Only `main` (the base branch) is always occupied by the primary repo.

**Reference Plan:**
- `specs/plans/fix_worktree_plan_merge_base_branch.md` - Detailed implementation plan with code snippets and dependency graph

## Goals

1. Add a `try_merge_in_repo()` method to GitService that merges directly in the primary repo without aborting on conflict
2. Detect when the merge target branch is already checked out and use the direct-in-repo path instead of creating a worktree
3. Route MergeIncomplete Retry through PendingMerge to re-attempt the programmatic merge (which now handles the base branch case)

## Dependencies

### Phase 108 (MergeIncomplete View) - Required

| Dependency | Why Needed |
|------------|------------|
| MergeIncomplete status + metadata | Error context display that this phase's retry flow depends on |
| Task metadata column | Used to persist error context on merge failure |

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

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_worktree_plan_merge_base_branch.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_worktree_plan_merge_base_branch.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add try_merge_in_repo() method to GitService — merges directly in primary repo without aborting on conflict",
    "plan_section": "1. git_service.rs — Add try_merge_in_repo()",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(git): add try_merge_in_repo for base-branch merge path",
    "steps": [
      "Read specs/plans/fix_worktree_plan_merge_base_branch.md section '1. git_service.rs'",
      "Add try_merge_in_repo() to GitService — essentially try_merge() (lines 924-975) but with the abort_merge removed from the conflict path",
      "Method: fetch origin (non-fatal), checkout target (no-op if already checked out), git merge --no-edit, return Success/NeedsAgent/Err",
      "On conflict: leave conflict state in place (don't abort) so merger agent can resolve in-place",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(git): add try_merge_in_repo for base-branch merge path"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Detect base-branch case in attempt_programmatic_merge() — when target branch is already checked out, merge directly in repo instead of creating worktree",
    "plan_section": "2. side_effects.rs — Detect base branch case",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "fix(merge): detect base-branch checkout in worktree-mode merge",
    "steps": [
      "Read specs/plans/fix_worktree_plan_merge_base_branch.md section '2. side_effects.rs'",
      "In attempt_programmatic_merge() at the worktree-mode branch (~line 965), add detection: get_current_branch, compare to target_branch",
      "If target is checked out: call GitService::try_merge_in_repo() instead of try_merge_in_worktree()",
      "Success path: complete_merge_internal() + post_merge_cleanup() (no merge worktree to delete)",
      "Conflict path: set task.worktree_path = project.working_directory (merger agent CWD → primary repo), transition to Merging, spawn agent",
      "Error path: transition to MergeIncomplete with error metadata (identical to existing error path)",
      "Else (target NOT checked out): existing try_merge_in_worktree() path unchanged",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(merge): detect base-branch checkout in worktree-mode merge"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Route MergeIncomplete Retry through PendingMerge — update valid transitions, transition dispatcher, and tests as a single compilation unit",
    "plan_section": "3. Status transitions + Retry routing + Tests (COMPILATION UNIT)",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(merge): route MergeIncomplete Retry through PendingMerge",
    "steps": [
      "Read specs/plans/fix_worktree_plan_merge_base_branch.md section '3. Status transitions + Retry routing + Tests'",
      "In status.rs: change MergeIncomplete valid transitions from &[Merging, Merged] to &[PendingMerge, Merging, Merged]",
      "In transitions.rs: change merge_incomplete() Retry target from State::Merging to State::PendingMerge",
      "Update merge_incomplete_transitions test (status.rs:786) to expect &[PendingMerge, Merging, Merged]",
      "Update merge_incomplete_to_merging test (status.rs:793) to verify PendingMerge transition",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: fix(merge): route MergeIncomplete Retry through PendingMerge"
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
| **Add `try_merge_in_repo()` vs modifying `try_merge()`** | Existing `try_merge()` aborts on conflict (cleanup). The new method leaves conflict state for agent resolution. Separate method avoids flag parameter and keeps both paths clear. |
| **Detect via `get_current_branch()` comparison** | Direct check — if current branch == target, the target is occupied. No need to enumerate all worktrees or parse git state. |
| **Set `worktree_path = working_directory` on conflict** | Reuses existing CWD resolution in agent spawner — merger agent resolves to primary repo where conflicts live. Minimal change to spawner logic. |
| **Route Retry → PendingMerge (not Merging)** | PendingMerge triggers `attempt_programmatic_merge()` which now handles the base branch case. Going straight to Merging would spawn the agent without attempting the fast path first. |
| **Tasks 3/4/5 merged as compilation unit** | Changing Retry target to PendingMerge requires PendingMerge in valid transitions and tests to match. Splitting creates runtime-invalid intermediate state. |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] `merge_incomplete_transitions` test passes with new `&[PendingMerge, Merging, Merged]` expected value
- [ ] `merge_incomplete_to_pending_merge` test passes (renamed from `merge_incomplete_to_merging`)
- [ ] All existing merge-related tests still pass

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### Manual Testing
- [ ] Create a plan with tasks in worktree mode, execute all tasks, let the auto-merge task trigger — it should merge the feature branch into main successfully via the direct-in-repo path
- [ ] For the currently stuck task: click "Retry Merge" — it should go through PendingMerge → attempt_programmatic_merge() → detect base branch → merge directly → success
- [ ] Regular task merges (task → plan branch) still work via the existing worktree path

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `try_merge_in_repo()` is called from `attempt_programmatic_merge()` when target branch is checked out
- [ ] Retry button in MergeIncomplete view transitions to PendingMerge (not Merging)
- [ ] PendingMerge triggers `attempt_programmatic_merge()` which detects the base branch case

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No functions exported but never called

See `.claude/rules/gap-verification.md` for full verification workflow.
