# Fix: Worktree-mode plan merge fails when target is base branch

## Context

In worktree mode, the auto-created "Merge plan into main" task fails with:
```
fatal: 'main' is already used by worktree '/Users/lazabogdan/Code/ralphx'
```

**Root cause:** `attempt_programmatic_merge()` always uses `try_merge_in_worktree()` in worktree mode. This creates a temporary worktree checking out the target branch. But for plan merge tasks (plan feature branch → main), the target is `main`, which is already checked out in the primary repo. Git forbids the same branch in multiple worktrees.

**Why it only affects plan merges:** Regular task merges (task → plan branch) work fine because plan branches aren't checked out anywhere. Only `main` (the base branch) is always occupied by the primary repo.

## Changes

### 1. `src-tauri/src/application/git_service.rs` — Add `try_merge_in_repo()` (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(git): add try_merge_in_repo for base-branch merge path`

Add a new method that merges directly in the primary repo **without aborting on conflict** (unlike `try_merge()` which aborts). This lets the merger agent resolve conflicts in-place if needed.

```rust
pub fn try_merge_in_repo(repo, source_branch, target_branch) -> AppResult<MergeAttemptResult>
```

- Fetch origin (non-fatal)
- Checkout target branch (already checked out → no-op)
- `git merge {source} --no-edit`
- Success → return SHA
- Conflict → **don't abort** (leave conflict state for agent) → return `NeedsAgent`
- Error → return Err

This is essentially `try_merge()` lines 924-975 but with the abort removed from the conflict path (line 964).

### 2. `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` — Detect base branch case
**Dependencies:** Task 1
**Atomic Commit:** `fix(merge): detect base-branch checkout in worktree-mode merge`

In `attempt_programmatic_merge()` at the worktree-mode branch (~line 965), add detection:

```rust
if project.git_mode == GitMode::Worktree {
    let current_branch = GitService::get_current_branch(repo_path).unwrap_or_default();
    let target_is_checked_out = current_branch == target_branch;

    if target_is_checked_out {
        // Target branch (main) is checked out in primary repo.
        // Merge directly there instead of creating a worktree.
        let merge_result = GitService::try_merge_in_repo(
            repo_path, &source_branch, &target_branch,
        );
        // Handle success/conflict/error (same patterns as existing code)
        // On conflict: set task.worktree_path = project.working_directory
    } else {
        // Existing path: try_merge_in_worktree()
    }
}
```

**Success path:** Call `complete_merge_internal()` + `post_merge_cleanup()` — identical to existing worktree success path (no merge worktree to delete).

**Conflict path:** Set `task.worktree_path = project.working_directory` so the merger agent CWD resolves to the primary repo. Transition to `Merging`, spawn agent.

**Error path:** Transition to `MergeIncomplete` with error metadata — identical to existing error path.

### 3. Status transitions + Retry routing + Tests (COMPILATION UNIT)
**Dependencies:** None (independent of Tasks 1-2)
**Atomic Commit:** `fix(merge): route MergeIncomplete Retry through PendingMerge`

> **Compilation unit note:** Tasks 3, 4, and 5 from the original plan are merged into a single task.
> Reason: Changing the Retry target to `PendingMerge` (transitions.rs) requires `PendingMerge` to be
> in `MergeIncomplete`'s valid transitions (status.rs), and tests must match. Splitting these would
> create a runtime-invalid intermediate state where Retry targets a disallowed transition.

**3a. `src-tauri/src/domain/entities/status.rs` — Add PendingMerge as valid from MergeIncomplete**

Change line 100:
```rust
// Before:
MergeIncomplete => &[Merging, Merged],
// After:
MergeIncomplete => &[PendingMerge, Merging, Merged],
```

**3b. `src-tauri/src/domain/state_machine/machine/transitions.rs` — Route Retry to PendingMerge**

Change `merge_incomplete()` at line 260-266:
```rust
// Before:
TaskEvent::Retry => Response::Transition(State::Merging),
// After:
TaskEvent::Retry => Response::Transition(State::PendingMerge),
```

This makes Retry re-attempt the programmatic merge (which now handles the base branch case), rather than going straight to the merger agent (which would face the same issue or have no valid CWD).

**3c. Tests**

- Update `merge_incomplete_transitions` test (status.rs:786) to expect `&[PendingMerge, Merging, Merged]`
- Update `merge_incomplete_to_merging` test (status.rs:793) — now retries via PendingMerge, rename to `merge_incomplete_to_pending_merge`

## Files Modified

| File | Task | Change |
|------|------|--------|
| `src-tauri/src/application/git_service.rs` | 1 | Add `try_merge_in_repo()` method |
| `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` | 2 | Detect base-branch case in `attempt_programmatic_merge()` |
| `src-tauri/src/domain/entities/status.rs` | 3 | Add `PendingMerge` to `MergeIncomplete` transitions + update tests |
| `src-tauri/src/domain/state_machine/machine/transitions.rs` | 3 | Change Retry target to `PendingMerge` |

## Task Dependency Graph

```
Task 1: Add try_merge_in_repo()  ──┐
                                    ├─→ Task 2: Detect base-branch case (calls try_merge_in_repo)
Task 3: Status + Retry + Tests  ──(independent, can run in parallel with 1 & 2)
```

Tasks 1→2 are sequential. Task 3 is independent and can be done in any order.

## Verification

1. `cargo clippy --all-targets --all-features -- -D warnings` — no warnings
2. `cargo test` — all tests pass (including updated transition tests)
3. Manual test: Create a plan with tasks in worktree mode, execute all tasks, let the auto-merge task trigger. It should now merge the feature branch into main successfully via the direct-in-repo path.
4. For the currently stuck task: after deploying, click "Retry Merge" — it will now go through `PendingMerge` → `attempt_programmatic_merge()` → detect base branch → merge directly → success.

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
