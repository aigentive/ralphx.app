# Plan: Add MergeIncompleteTaskDetail View

## Context

`merge_incomplete` is a human-waiting state for non-conflict git errors, but it's mapped to `MergingTaskDetail` (active agent view showing "AI agent is resolving conflicts"). Users see no Retry button. This should be added as **Task 4** to the existing Phase 98 PRD.

## Changes

### 1. Create `src/components/tasks/detail-views/MergeIncompleteTaskDetail.tsx` (~200 LOC) (BLOCKING)
**Dependencies:** None
**Atomic Commit:** Single commit with all 4 changes (see Commit section)

Follow `MergeConflictTaskDetail.tsx` pattern exactly. Structure:

| Section | Component | Details |
|---------|-----------|---------|
| Header | `StatusBanner` variant="error" | Icon: AlertTriangle, title: "Merge Incomplete", subtitle: error context |
| Error context | `DetailCard` variant="error" | Generic error explanation + possible causes (branch deleted, git lock, network) |
| Recovery steps | `DetailCard` default | Numbered steps for manual recovery (different from conflict resolution) |
| Actions | Two buttons | "Retry Merge" (primary) + "Mark Resolved" (secondary, green) |

- Props: `{ task: Task; isHistorical?: boolean }` (standard `TaskDetailProps`)
- Buttons disabled when `isHistorical` or processing
- `handleRetryMerge` invokes `retry_merge` (already accepts MergeIncomplete)
- `handleMarkResolved` invokes `resolve_merge_conflict` (needs backend fix below)
- Use `error` variant (red) to distinguish from MergeConflict's `warning` (amber)
- No conflict files display (not a conflict state)

### 2. Wire into view registry
**Dependencies:** Change 1 (imports the new component)

**`src/components/tasks/detail-views/index.ts`** - add export

**`src/components/tasks/TaskDetailPanel.tsx`**:
- Import `MergeIncompleteTaskDetail`
- Change line 109: `merge_incomplete: MergingTaskDetail` -> `merge_incomplete: MergeIncompleteTaskDetail`

### 3. Backend: Widen `resolve_merge_conflict` status guard
**Dependencies:** None (independent, but required for Change 1's Mark Resolved button to work)

**`src-tauri/src/commands/git_commands.rs:204`**

Currently only accepts `MergeConflict`. Change to accept both `MergeConflict` and `MergeIncomplete`:

```rust
let valid_resolve_states = [
    InternalStatus::MergeConflict,
    InternalStatus::MergeIncomplete,
];
if !valid_resolve_states.contains(&task.internal_status) { ... }
```

The state machine already allows `merge_incomplete -> merged`. This just unblocks the command handler.

### 4. Update `.claude/rules/task-detail-views.md`
**Dependencies:** Change 1 (documents the new component)

Add `merge_incomplete | MergeIncompleteTaskDetail | Non-conflict merge failure, retry/resolve` to the view mapping table.

## Note on error details

The programmatic merge error is only logged via `tracing::error!`, never stored in task metadata. The view will show generic "merge failed due to a git error" messaging. Storing error details in metadata is a future improvement.

## Verification

1. `cargo clippy --all-targets --all-features -- -D warnings && cargo test` (backend guard change)
2. `npm run lint && npm run typecheck` (new component + registry wiring)
3. Visual: In web mode, a task with `merge_incomplete` status should show error banner + Retry/Resolve buttons (not the agent spinner view)

## Commit

Single atomic commit: `feat(merge): add MergeIncompleteTaskDetail view with retry/resolve actions`

All 4 changes form one compilation unit (component + wiring + backend guard + docs).

## Compilation Unit Analysis

All 4 changes MUST be in a single commit:
- Change 2 imports Change 1's component → separate commits would break TS compilation
- Change 1's `handleMarkResolved` calls `resolve_merge_conflict` which only works for `merge_incomplete` after Change 3 → runtime correctness requires both
- Change 4 is documentation only but should reflect the code state

No chicken-egg problem: this is purely additive (new component + widened guard).

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
