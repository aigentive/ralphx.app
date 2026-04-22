# GitHub PR Mode Architecture

> **Source:** Derived from code audit of
> `src-tauri/src/commands/project_commands.rs`,
> `src-tauri/src/commands/ideation_commands/ideation_commands_apply.rs`,
> `src-tauri/src/domain/state_machine/transition_handler/merge_helpers.rs`,
> `src-tauri/src/domain/state_machine/transition_handler/side_effects/merge_attempt/pr_mode.rs`,
> `src-tauri/src/domain/state_machine/transition_handler/on_enter_states/merge.rs`,
> `src-tauri/src/application/services/pr_merge_poller.rs`,
> `src-tauri/src/application/reconciliation/handlers/merge.rs`,
> and the frontend task-detail/task-card surfaces.
> Do not update from older docs alone. Re-audit the code if behavior changes.

## Overview

GitHub PR Mode is a **plan-delivery variant** layered on top of the normal plan/task branch model.

Normal plan flow:

```
task branches -> plan branch -> base branch
```

GitHub PR Mode keeps the same branch hierarchy, but replaces the **final plan branch -> base branch** step with a GitHub PR-backed settlement flow:

```
task branches -> plan branch -> GitHub PR -> base branch
```

Scope boundaries:

- Applies to **plan merge tasks**, not every task merge.
- Individual task merges still use RalphX's normal merge pipeline.
- The feature is controlled by a project-level setting and copied into each plan branch as plan-local eligibility.

## Core Data Model

| Field | Layer | Meaning |
|------|-------|---------|
| `projects.github_pr_enabled` | Project setting | Default policy for future plans in the project |
| `plan_branches.pr_eligible` | Plan branch | Whether this plan uses the PR-backed final merge path |
| `plan_branches.pr_number` | Plan branch | GitHub PR identity once created |
| `plan_branches.pr_url` | Plan branch | Deep link for UI/open-in-browser |
| `plan_branches.pr_status` | Plan branch | Current PR settlement state shown in the UI |
| `plan_branches.pr_polling_active` | Plan branch | Whether RalphX believes the background poller is active |
| `plan_branches.last_polled_at` | Plan branch | Poller heartbeat / UI freshness marker |
| `plan_branches.merge_commit_sha` | Plan branch | Merge SHA captured from GitHub after PR merge |

## Lifecycle

### 1. Project toggle

`update_github_pr_enabled` persists the project-level setting and then reconciles in-progress plans.

Important semantics:

- **PR -> push-to-main** is active cleanup: stop poller, close PR, clear PR metadata, and retry via the normal merge path if needed.
- **push-to-main -> PR** now retrofits active plans by updating `plan_branches.pr_eligible`; if a merge task is already in `PendingMerge`, RalphX immediately re-enters that merge through the PR path.

### 2. Plan creation

When proposals are applied, `apply_proposals_core` creates or updates the plan branch record and copies `project.github_pr_enabled` into `plan_branches.pr_eligible`.

This is the key contract:

- The project setting is the default for **new** plans.
- The project-toggle reconciler can also update active plans already in flight.
- `pr_eligible` is the plan-local switch the merge pipeline reads later.

### 3. Early draft PR creation

While regular plan tasks are resolving their base branch, RalphX opportunistically calls `create_draft_pr_if_needed(...)` for `pr_eligible` plan branches.

Behavior:

- Push the plan branch if needed
- Create a draft PR
- Persist PR metadata on the plan branch
- Use a CAS guard to prevent duplicate PR creation across concurrent task executions

This stage is best-effort and non-blocking. Failure here does not fail the task; the final plan-merge path can still create the PR later.

### 4. PendingMerge fork for the final plan merge task

`attempt_programmatic_merge()` has a PR-mode fork:

- If the merge task's plan branch has `pr_eligible=true` and GitHub integration is available, RalphX does **not** run the normal local plan-merge path.
- Instead it runs `run_pr_mode_pending_merge(...)`.

That path:

1. Clears stale local merge metadata
2. Runs the normal concurrent-merge guard
3. Sets `pr_polling_active`
4. Pushes the plan branch
5. Uses the existing PR if present, or creates one if still missing
6. Marks the PR ready for review
7. Transitions the task from `PendingMerge` to `Merging`

### 5. Merging state changes meaning

For direct merges, `Merging` usually means a merger agent is working.

For PR mode, `on_enter(Merging)` skips spawning the merger agent and starts the PR poller instead.

This is the most important semantic difference in the UI contract:

- same task state
- different runtime owner
- different user expectation

### 6. PR poller settlement loop

`PrPollerRegistry` owns the background GitHub pollers.

Responsibilities:

- one live poller per task
- cancellation / stopping guard
- rate-limit-aware polling
- persisted `pr_status` and `last_polled_at` updates
- transition on terminal PR outcomes

Settlement behavior:

- `Open` -> keep polling, update DB for UI
- `Merged` -> fetch remote, persist merge SHA, transition task to `Merged`
- `Closed` -> transition task to `MergeIncomplete`
- repeated errors / stale PR -> transition task to `MergeIncomplete`

### 7. Post-merge cleanup

PR mode has a different cleanup fork after the task reaches `Merged`.

Instead of only deleting local feature branches, RalphX also:

- deletes the remote plan branch
- deletes remote task branches for sibling tasks in the same plan

This keeps GitHub-side branch cleanup aligned with the PR-backed settlement path.

### 8. Recovery and reconciliation

The reconciler treats PR-mode merges as a special case:

- live poller + `pr_polling_active=true` means “this merge is waiting on GitHub; do not run normal merge recovery”
- startup recovery can restart PR polling when needed
- mode-switch metadata allows a PR-backed merge to be converted back to the direct merge path safely

## UI Contract

Current user-facing surfaces depend on plan-branch PR metadata:

| Surface | Current behavior |
|--------|------------------|
| Repository settings | Exposes the project-level toggle and prerequisite copy |
| Task card | Shows `Review PR` when the plan-merge task has an active PR |
| Merging detail | Shows `Waiting for PR Merge`, PR status, polling state, and `Open in GitHub` |
| Merged detail | Shows `Merged via PR #...` when the final merge came from GitHub |

## Constraints and Invariants

- `pr_eligible=false` means the task must use the normal direct-merge path, even if PR metadata exists.
- `pr_eligible=true` without a live GitHub service also falls back to the direct-merge path.
- Active plans can be converted when the project toggle is turned on later because the reconciler updates `pr_eligible` and re-runs pending plan merges.
- Disabling PR mode mid-plan is destructive to the PR flow by design: RalphX closes the PR and resumes direct merge handling.
- The current UI reuses the `Merging` state for both “agent resolving merge work” and “waiting for GitHub PR settlement.” Keep user-facing copy aligned with that dual meaning.
