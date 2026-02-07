# RalphX - Phase 99: MergeIncomplete Detail View

## Overview

The `merge_incomplete` status is a human-waiting state for non-conflict git errors (branch deleted, git lock, network failure). Currently it's mapped to `MergingTaskDetail` — the active agent view showing "AI agent is resolving conflicts" with a spinner. Users see no Retry or Mark Resolved buttons, making the state a dead end in the UI.

This phase adds a dedicated `MergeIncompleteTaskDetail` view with error context, recovery steps, and action buttons (Retry Merge + Mark Resolved), plus widens the backend `resolve_merge_conflict` guard to accept `merge_incomplete` tasks.

**Reference Plan:**
- `specs/plans/add_merge_incomplete_task_detail.md` - Full implementation plan with component structure, backend changes, and compilation unit analysis

## Goals

1. Add dedicated `MergeIncompleteTaskDetail` view with error banner (red variant) distinguishing from conflict (amber)
2. Provide Retry Merge and Mark Resolved action buttons for user recovery
3. Widen `resolve_merge_conflict` backend guard to accept both `MergeConflict` and `MergeIncomplete`
4. Update view registry documentation

## Dependencies

### Phase 98 (Fix Merge Workflow Bugs) - Required

| Dependency | Why Needed |
|------------|------------|
| `MergeIncomplete` status | Phase 83 added the status; Phase 98 fixed agent mismatches |
| `MergeConflictTaskDetail` | Pattern source for the new component |
| Shared detail view components | `StatusBanner`, `DetailCard`, `TwoColumnLayout` from `detail-views/shared/` |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/add_merge_incomplete_task_detail.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/add_merge_incomplete_task_detail.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Create MergeIncompleteTaskDetail component, wire into view registry, widen backend guard, update docs",
    "plan_section": "Changes 1-4 (single compilation unit)",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(merge): add MergeIncompleteTaskDetail view with retry/resolve actions",
    "steps": [
      "Read specs/plans/add_merge_incomplete_task_detail.md (all sections)",
      "Read src/components/tasks/detail-views/MergeConflictTaskDetail.tsx as pattern source",
      "Create src/components/tasks/detail-views/MergeIncompleteTaskDetail.tsx (~200 LOC) following MergeConflictTaskDetail pattern: StatusBanner variant='error', DetailCard for error context (branch deleted, git lock, network causes), DetailCard for numbered recovery steps, two action buttons (Retry Merge primary + Mark Resolved green), error variant (red) to distinguish from MergeConflict warning (amber), no conflict files display, props: { task: Task; isHistorical?: boolean }",
      "Add export to src/components/tasks/detail-views/index.ts",
      "Update src/components/tasks/TaskDetailPanel.tsx: import MergeIncompleteTaskDetail, change merge_incomplete mapping from MergingTaskDetail to MergeIncompleteTaskDetail (line 109)",
      "Backend: In src-tauri/src/commands/git_commands.rs:204, widen resolve_merge_conflict guard from MergeConflict-only to accept both MergeConflict and MergeIncomplete using valid_resolve_states array",
      "Update .claude/rules/task-detail-views.md: add merge_incomplete | MergeIncompleteTaskDetail | Non-conflict merge failure, retry/resolve to view mapping table",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(merge): add MergeIncompleteTaskDetail view with retry/resolve actions"
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
| **Single compilation unit** | Component + wiring + backend guard + docs must be one commit — Change 2 imports Change 1, Change 1's Mark Resolved needs Change 3 at runtime |
| **Error variant (red) vs warning (amber)** | Distinguishes non-conflict errors from merge conflicts visually |
| **Generic error messaging** | Programmatic merge error is only logged via `tracing::error!`, not stored in task metadata — future improvement |
| **Reuse `resolve_merge_conflict` command** | State machine already allows `merge_incomplete → merged`; just need to widen the command handler guard |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] `resolve_merge_conflict` accepts both `MergeConflict` and `MergeIncomplete` states
- [ ] State machine transition `merge_incomplete → merged` still works

### Frontend - Run `npm run typecheck`
- [ ] `MergeIncompleteTaskDetail` component type-checks
- [ ] View registry maps `merge_incomplete` to new component
- [ ] Export exists in `detail-views/index.ts`

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes

### Manual Testing
- [ ] In web mode, a task with `merge_incomplete` status shows error banner (red, not amber)
- [ ] Retry Merge button visible and not disabled
- [ ] Mark Resolved button visible and not disabled
- [ ] No agent spinner view shown (old MergingTaskDetail mapping removed)

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `MergeIncompleteTaskDetail` is imported AND rendered via `TASK_DETAIL_VIEWS` registry
- [ ] `handleRetryMerge` calls `invoke("retry_merge")` — already accepts `MergeIncomplete`
- [ ] `handleMarkResolved` calls `invoke("resolve_merge_conflict")` — guard widened to accept `MergeIncomplete`
- [ ] Buttons disabled when `isHistorical` is true

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
