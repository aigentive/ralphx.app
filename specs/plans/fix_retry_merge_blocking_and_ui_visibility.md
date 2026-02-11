# Fix Retry Merge Blocking and UI Visibility

## Objective

Eliminate app-wide perceived hangs during retry merge and post-merge validation, while preserving strict merge semantics:

- A task must only transition to `merged` after all configured post-merge validations pass.
- UI must show immediate progress after Retry is clicked.
- App reload during merge/validation must not break backend processing or leave UI stale.
- No new intermediary task status should be introduced.

## Problem Statement

Current behavior shows three coupled problems:

1. `retry_merge` can remain tied to long-running backend work, so frontend callback lifecycle is coupled to validation duration.
2. Post-merge validations are executed with blocking shell calls (`std::process::Command::output`) in merge transition path.
3. UI receives insufficient high-level progress/state updates, making it look stuck until final transition.

Observed symptom during reload:

- `[TAURI] Couldn't find callback id ...` appears while Rust is still processing async operation.
- UI appears frozen but eventually recovers once backend emits final transition.

## Confirmed Evidence

### State persisted early
Task state history shows immediate transition from `merge_incomplete -> pending_merge` when retry starts (example observed around `2026-02-11T06:37:26Z`).

### Long blocking validation window
Logs show sequential commands with long wall-clock durations, e.g.:

- `npm run typecheck`
- `npm run lint`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `npx tsc --noEmit`

In one reported run (`2026-02-11`), validation spanned multiple minutes between merge fast-path success and final `complete_merge_internal`.

### Blocking execution path
`run_validation_commands` currently executes shell commands synchronously (`Command::new("sh").arg("-c").arg(...).output()`) in state-machine side effects, increasing risk of runtime thread starvation under long commands.

## Non-Goals

- Do not loosen merge correctness (no early `merged` before required validations pass).
- Do not redesign full task lifecycle/status taxonomy.
- Do not remove existing detailed validation streaming (`merge:validation_step`) used for diagnostics.

## Design Decisions

1. Keep strict completion semantics:
- Task remains `pending_merge` until all blocking validations complete.
- Final transition only at pass (`merged`) or fail (`merge_incomplete` / `merging` if conflict/agent path).

2. No new task state:
- Reuse `pending_merge` as the single state representing in-progress programmatic merge + validation.

3. Decouple UI invocation from backend duration:
- `retry_merge` must return quickly after scheduling work.
- Long work must continue in background.

4. Reload policy:
- Continue processing in backend during reload.
- Reconnected UI must resubscribe and recover current status/progress.

5. Progress model for UX:
- Expose high-level phases for primary UI, while preserving low-level command stream for diagnostics.

## Implementation Plan

### 1) Make retry merge non-blocking at command boundary

#### Files
- `src-tauri/src/commands/git_commands.rs`

#### Changes
- Refactor `retry_merge` to:
  1. Validate allowed source statuses.
  2. Persist transition to `pending_merge`.
  3. Emit status-change events immediately.
  4. Spawn/queue merge execution asynchronously.
  5. Return `Ok(())` immediately.

- Add per-task in-flight guard metadata (e.g. `merge_retry_in_progress: true`) to prevent duplicate concurrent retries from repeated clicks.

#### Acceptance
- `invoke("retry_merge")` resolves quickly regardless of validation duration.
- UI can continue interacting with app while validations run.

### 2) Remove blocking shell execution from merge transition hot path

#### Files
- `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`

#### Changes
- Replace synchronous command execution in `run_validation_commands` with one of:
  - `tokio::process::Command` async execution, or
  - `tokio::task::spawn_blocking` wrapper for strictly blocking subprocess path.

- Ensure heavy command loop does not occupy critical runtime worker threads.

- Preserve existing command ordering and failure behavior.

#### Acceptance
- Long-running commands no longer cause app-wide responsiveness degradation.
- Runtime remains responsive to unrelated events/commands during validation.

### 3) Introduce high-level merge progress event stream

#### Backend
- Emit new event: `task:merge_progress`.

#### Payload
- `task_id: string`
- `phase: "worktree_setup" | "programmatic_merge" | "typecheck" | "lint" | "clippy" | "test" | "finalize"`
- `status: "started" | "passed" | "failed"`
- `message: string`
- `timestamp: string (RFC3339)`

#### Mapping rules
- Each configured validation command maps to a canonical phase when possible:
  - `npm run typecheck`, `npx tsc --noEmit` -> `typecheck`
  - `npm run lint` -> `lint`
  - `cargo clippy ...` -> `clippy`
  - `cargo test` -> `test`
- Unknown commands map to `finalize` with message preserving original command context (or optionally `custom`).

#### Acceptance
- UI receives deterministic, user-friendly progress phases throughout the full merge window.

### 4) Keep and harden status event compatibility

#### Files
- `src-tauri/src/application/task_transition_service.rs`

#### Changes
- Ensure every status transition in this flow emits both:
  - `task:event` (`status_changed` payload)
  - `task:status_changed` (legacy snake_case payload)

#### Acceptance
- Existing listeners continue working without migration breakage.

### 5) Frontend retry UX: immediate state and live progress

#### Files
- `src/components/tasks/detail-views/MergeIncompleteTaskDetail.tsx`
- `src/components/tasks/TaskDetailOverlay.tsx`
- `src/hooks/useEvents.task.ts` (or dedicated merge-progress hook)
- `src/types/events.ts`

#### Changes
- On Retry click:
  1. Exit history mode (`taskHistoryState = null`) to force live view.
  2. Optimistically update selected task to `pending_merge`.
  3. Invoke backend retry command.

- Add listener/schema for `task:merge_progress` and render phase timeline in merge-related detail views.

- Preserve fallback behavior from status events/query invalidation.

#### Acceptance
- User instantly sees task leave `merge_incomplete`.
- Progress panel updates while validations are running.

### 6) Reload recovery and continuation

#### Existing infrastructure to leverage
- Startup/resumption and reconciliation for `pending_merge` / `merging` tasks.

#### Changes
- On app startup/reload:
  - Rehydrate current task statuses from DB.
  - Re-attach merge progress listeners.
  - If task is still active merge state and no recent progress event exists, display "resuming merge status..." until next event/state update.

- Ensure in-flight guard flag and any transient progress cache are reconciled on restart.

#### Acceptance
- Reload mid-validation no longer appears as permanent hang.
- Task eventually resolves to final correct state with visible progress after reconnect.

### 7) Concurrency interactions with deferred merge queue

#### Context from logs
- Deferred merge behavior (`merge_deferred`) correctly serialized merges targeting same feature branch.

#### Required behavior
- Non-blocking changes must not break:
  - Older-task priority logic.
  - Deferred merge re-trigger after prior merge completion.

#### Acceptance
- Deferred tasks continue from `pending_merge` after predecessor completes.
- No duplicate merge attempts per deferred task.

## Test Plan

### Rust tests

1. `retry_merge` command latency test
- Assert command returns quickly while merge worker still running.

2. Background execution correctness
- Assert state transitions occur in correct order with long simulated validation.

3. Blocking isolation test
- While merge validation runs, execute unrelated command and assert responsiveness.

4. Reload continuation test
- Simulate callback drop / restart while validation running.
- Assert eventual correct terminal state.

5. Event emission tests
- Validate `task:merge_progress` phase ordering and statuses.
- Validate `task:event` + `task:status_changed` both emitted.

6. Deferred merge compatibility tests
- Keep existing deferred merge tests and add regression checks for non-blocking path.

### Frontend tests

1. Retry immediate visual transition
- From `merge_incomplete`, click Retry and assert view switches to active merge state immediately.

2. Progress rendering
- Inject `task:merge_progress` events and assert phase UI updates in sequence.

3. Reload-style remount behavior
- Unmount/remount listeners while progress is mid-stream; assert recovery and eventual terminal display.

4. History mode exit behavior
- If in historical mode, Retry exits history and shows live status.

## Rollout Plan

1. Land backend non-blocking + event emission behind internal feature flag if needed.
2. Land frontend progress visualization and optimistic transition.
3. Enable by default after validation in dev and one internal dogfood cycle.

## Observability

Add/verify structured logs:

- Retry accepted + background job ID/task ID
- Phase start/end with duration
- Callback lifecycle decoupled confirmation
- Restart/recovery resumption markers

Metrics (optional):

- `merge_retry_command_latency_ms`
- `merge_validation_total_duration_ms`
- `merge_phase_duration_ms{phase=...}`
- `merge_reload_recovery_count`

## Risks and Mitigations

1. Duplicate merge execution
- Mitigation: per-task in-flight guard + idempotent scheduler checks.

2. Event ordering race after reload
- Mitigation: status remains source of truth; progress is advisory.

3. Runtime resource pressure from many concurrent long validations
- Mitigation: bounded worker/semaphore for merge validation tasks.

4. Semantic drift in validation command mapping
- Mitigation: canonical mapper with fallback labels.

## Acceptance Criteria Checklist

- [ ] Retry returns immediately and does not block UI interactivity.
- [ ] Task remains `pending_merge` during validation without adding new states.
- [ ] User sees live high-level progress across full validation duration.
- [ ] Reload during validation no longer causes perceived stuck state; UI reconnects and recovers.
- [ ] Merge only transitions to `merged` after all blocking validations pass.
- [ ] Deferred merge queue behavior remains correct.
- [ ] Existing event consumers remain backward compatible.

## Suggested Commit Sequence for Implementer

1. Backend: non-blocking retry + validation execution refactor.
2. Backend: `task:merge_progress` event emission + tests.
3. Frontend: schema/listener/progress UI + tests.
4. Startup/recovery/reconciliation polishing + tests.

