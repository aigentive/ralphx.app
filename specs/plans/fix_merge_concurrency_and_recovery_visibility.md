# Fix Plan: Deterministic Merge Deferral + Visible Recovery Timeline

## Summary
Implement an end-to-end fix so concurrent merges targeting the same branch are safely serialized, automatically retried when unblocked, and clearly visible in task UI.

This plan addresses:
1. Wrong winner selection (`created_at` race) causing false `merge_incomplete`.
2. Missing auto-retry for tasks that should have been deferred.
3. Poor observability of recovery attempts in the task detail screen.

Chosen direction:
- Scope: **End-to-end**
- Winner rule: **first task to enter `pending_merge` wins**
- UX: **rich timeline** of defer/retry/recovery attempts

## Root-Cause Fix (Backend)

### 1) Replace `created_at` arbitration with `pending_merge_entered_at`
**Files**
- `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`
- `src-tauri/src/domain/repositories/task_repository.rs`
- `src-tauri/src/infrastructure/sqlite/sqlite_task_repo/*` (query + impl)
- `src-tauri/src/infrastructure/memory/memory_task_repo/*` (test parity)

**Changes**
- Add repository method:
  - `get_status_entered_at(task_id: &TaskId, status: InternalStatus) -> Option<DateTime<Utc>>`
- In concurrent merge guard, when evaluating same-target tasks in `{pending_merge, merging}`:
  - Winner = smallest `pending_merge_entered_at`
  - Tie-breaker = lexical `task.id` (stable deterministic fallback)
- Defer losers by setting `merge_deferred` metadata and staying in `pending_merge`.

**Why**
This matches runtime order and removes false failures caused by tiny `created_at` differences.

### 2) Treat "branch already used by worktree" as deferrable, not terminal
**Files**
- `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`
- `src-tauri/src/application/git_service.rs` (optional helper for error classification)

**Changes**
- In merge attempt error handling (`try_merge_in_worktree` / in-repo paths), classify branch-lock errors:
  - Match git error signatures containing `already used by worktree` / checked-out branch lock patterns.
- For classified lock errors:
  - Do **not** transition to `merge_incomplete`.
  - Set/refresh `merge_deferred` metadata + recovery event.
  - Keep task in `pending_merge`.
- Keep current `merge_incomplete` behavior for non-deferrable failures.

**Why**
This makes branch contention recoverable automatically.

### 3) Ensure deferred merges are retried whenever blocker exits merge workflow
**Files**
- `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`
- `src-tauri/src/application/task_scheduler_service.rs`

**Changes**
- Trigger `try_retry_deferred_merges(project_id)` when a task leaves merge-blocking states (`pending_merge`/`merging`) to a non-blocking state (`merged`, `merge_incomplete`, etc.), not only on successful merge cleanup.
- Keep retry one-at-a-time semantics for safe serialization.

**Why**
Deferred tasks should resume promptly regardless of how blocker ended.

## Recovery Metadata & Observability

### 4) Add structured merge recovery event log in task metadata
**Storage**
- Continue using `tasks.metadata` JSON; add `merge_recovery` object.

**Schema (new metadata keys)**
```json
{
  "merge_recovery": {
    "version": 1,
    "events": [
      {
        "at": "2026-02-11T05:28:38.420043+00:00",
        "kind": "deferred|auto_retry_triggered|attempt_started|attempt_failed|attempt_succeeded|manual_retry",
        "source": "system|auto|user",
        "reason_code": "target_branch_busy|git_error|validation_failed|unknown",
        "message": "human readable summary",
        "target_branch": "ralphx/ralphx/plan-cb81bf42",
        "source_branch": "ralphx/ralphx/task-...",
        "blocking_task_id": "7a206241-...",
        "attempt": 2
      }
    ],
    "last_state": "deferred|retrying|failed|succeeded"
  }
}
```

**Rules**
- Append-only events (cap at last 50, trim oldest).
- Preserve existing keys (`error`, `source_branch`, `target_branch`, validation fields).
- For legacy tasks with only old error metadata, UI shows a derived single “failed” event.

## UI Improvements (Rich Timeline)

### 5) Expand `MergeIncompleteTaskDetail` to show recovery attempts
**Files**
- `src/components/tasks/detail-views/MergeIncompleteTaskDetail.tsx`
- `src/types/task.ts` (optional helper typings)
- test files under `src/components/tasks/detail-views/*.test.tsx`

**Changes**
- Parse `metadata.merge_recovery.events`.
- Add “Recovery Attempts” section:
  - Chronological entries with timestamp, kind, reason, blocker task id, source (auto/manual), attempt no.
- Add explicit badges:
  - `Auto-recovery attempted`
  - `Deferred due to active merge`
  - `Last attempt failed`
- Keep existing “What Happened” + action buttons.
- If no structured events, show fallback text: “No recorded recovery attempts for this task.”

**Why**
Makes auto-recovery behavior visible and debuggable directly in the task view.

## Public Interfaces / API / Types Changes

### Rust
- `TaskRepository`:
  - Add `get_status_entered_at(...)` read method.
- Metadata contract:
  - New optional `merge_recovery` object in `tasks.metadata` JSON.

### TypeScript
- Add optional client-side type for merge recovery metadata:
  - `MergeRecoveryEvent`, `MergeRecoveryState`.
- No HTTP/Tauri command signature changes required for this iteration.

## Testing Plan

### Backend Unit/Integration
1. **Arbitration correctness**
- Two tasks same target, newer `created_at` but earlier `pending_merge_entered_at` wins.
- Tie case resolved by task ID.
2. **Deferral on lock error**
- Simulate `already used by worktree` error -> task remains `pending_merge` with `merge_deferred=true`.
3. **Non-deferrable errors**
- Unknown git error -> transitions to `merge_incomplete`.
4. **Retry trigger on blocker exit**
- Blocker transitions to `merged` -> one deferred task retried.
- Blocker transitions to `merge_incomplete` -> deferred task retried.
5. **Metadata events**
- Event append order, cap/trim behavior, compatibility with preexisting metadata.

### Frontend
1. Renders full recovery timeline from metadata.
2. Shows fallback message when no events.
3. Correct labels for auto/manual attempts and blocker info.
4. Snapshot/DOM tests for `merge_incomplete` with rich timeline.

### Regression
- Existing merge conflict flow (`pending_merge -> merging`) unchanged.
- Existing retry/resolve buttons continue to work.

## Acceptance Criteria

1. Concurrent merges to same target no longer fail due to branch/worktree lock races.
2. Loser tasks stay in `pending_merge` with deferred metadata instead of `merge_incomplete` for lock contention.
3. Deferred tasks auto-retry when blocker exits merge workflow.
4. `merge_incomplete` screen clearly shows whether auto-recovery happened and what attempts were made.
5. Existing `merge_incomplete` tasks still render correctly without migration.

## Rollout & Diagnostics

- Add structured logs on arbitration decision:
  - winner task id, loser task id, target branch, reason.
- Add structured logs on deferral and retry trigger.
- Optional metric counters (if available): `merge.deferred`, `merge.retry_auto`, `merge.retry_failed`.

## Assumptions and Defaults

1. No DB migration: recovery timeline is stored in `tasks.metadata`.
2. Event cap default = 50 per task.
3. Deterministic winner default = earliest `pending_merge` entry time, tie by task ID.
4. Deferrable error matching is substring-based on known git messages.
5. UI timeline is implemented only on `MergeIncompleteTaskDetail` in this iteration (not global activity feed).
