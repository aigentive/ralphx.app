# Fix: Feature Branch Not Created on Accept Plan + Task Merging to Main

## Context

After clicking "Accept Plan" from ideation, the plan group's feature branch toggle was OFF despite the project having `use_feature_branches = true`. Clicking the toggle manually failed. After ~1 minute the UI updated to show the branch. Additionally, a task in the plan merged to MAIN instead of the feature branch.

## Root Cause Analysis

**Primary bug: Inconsistent `plan_artifact_id` fallback logic**

- Session `40fde38b` has `plan_artifact_id = NULL` (no plan artifact was created during ideation)
- The **graph query** (`query.rs:551-554`) uses `session_id` as fallback when `plan_artifact_id` is None
- The **apply flow** (`ideation_commands_apply.rs:167`) does NOT have this fallback — it uses `session.plan_artifact_id` directly
- Result: When `session.plan_artifact_id` is None:
  1. Tasks don't get `plan_artifact_id` set (line 170: `if let Some(ref artifact_id)` → skipped)
  2. Feature branch is NOT created (line 207: `if let Some(ref artifact_id)` → skipped)
  3. Graph still shows a plan group (using session_id fallback), but with no feature branch
  4. Task merges to MAIN because `resolve_merge_branches` checks `task.plan_artifact_id` which is NULL

**Secondary bugs:**
- `useApplyProposals.onSuccess` doesn't invalidate `["plan-branch"]` queries → stale UI
- `enable_feature_branch` can't find plan tasks (they lack `plan_artifact_id`) → merge task has no dependencies

**DB evidence:**
- Task `b451df8f`: `plan_artifact_id = NULL`, merged to main with commit `3f89dbc`
- Plan branch `2eff9b4c`: `plan_artifact_id = '40fde38b'` (session_id), `merge_task_id = NULL`

## Fix Plan

### 1. Backend: Apply consistent plan_artifact_id resolution in apply flow (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(ideation): use session_id fallback for plan_artifact_id in apply flow`

**File:** `src-tauri/src/commands/ideation_commands/ideation_commands_apply.rs`

At line 167, change:
```rust
let plan_artifact_id: Option<ArtifactId> = session.plan_artifact_id.clone();
```
To:
```rust
// Use session.plan_artifact_id, falling back to session.id (matches graph query logic)
let plan_artifact_id: Option<ArtifactId> = Some(
    session.plan_artifact_id.clone().unwrap_or_else(|| {
        ArtifactId::from_string(session.id.as_str().to_string())
    })
);
```

This ensures:
- Tasks always get `plan_artifact_id` set
- Feature branch is always created when `use_feature_branches = true`
- `resolve_merge_branches` can find the plan branch for the task

### 2. Backend: Apply same fallback in enable_feature_branch task lookup
**Dependencies:** None
**Atomic Commit:** `fix(plan-branch): add session-based task lookup fallback in enable_feature_branch`

**File:** `src-tauri/src/commands/plan_branch_commands.rs`

The `enable_feature_branch` command filters tasks by `plan_artifact_id` (line 168-169). For tasks created before this fix (where `plan_artifact_id` is NULL), also match tasks that belong to this plan via proposal tracing:

At line 160-173, after finding tasks by `plan_artifact_id`, add fallback: also find tasks linked via `task_proposals` where `session_id` matches the plan's session_id. This ensures mid-plan enable works even for older tasks.

Alternatively (simpler): After finding unmerged plan tasks, also backfill `plan_artifact_id` on any matched tasks that have it NULL.

### 3. Frontend: Invalidate plan-branch queries after apply
**Dependencies:** None
**Atomic Commit:** `fix(ideation): invalidate plan-branch queries after apply`

**File:** `src/hooks/useApplyProposals.ts`

In `onSuccess` callback (after line 62), add:
```typescript
// Invalidate plan-branch queries since feature branch may have been created
queryClient.invalidateQueries({
  queryKey: ["plan-branch"],
});
```

### 4. Frontend: Reduce plan-branch staleTime
**Dependencies:** None
**Atomic Commit:** `fix(graph): reduce plan-branch staleTime for faster UI updates`

**File:** `src/components/TaskGraph/groups/PlanGroupHeader.tsx`

At line 219, reduce `staleTime` from `30_000` to `5_000` (5 seconds). The plan branch data is small and fast to fetch, and 30 seconds is too long for a critical state indicator.

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/commands/ideation_commands/ideation_commands_apply.rs` | Add session_id fallback for plan_artifact_id |
| `src-tauri/src/commands/plan_branch_commands.rs` | Improve task lookup in enable_feature_branch |
| `src/hooks/useApplyProposals.ts` | Invalidate plan-branch queries |
| `src/components/TaskGraph/groups/PlanGroupHeader.tsx` | Reduce staleTime |

## Verification

1. **Reproduce:** Create a new ideation session (no plan artifact), add proposals, click "Accept Plan"
2. **Check DB:** `SELECT plan_artifact_id FROM tasks WHERE ...` → should have plan_artifact_id set
3. **Check DB:** `SELECT * FROM plan_branches WHERE ...` → should have active branch with merge_task_id
4. **Check UI:** Navigate to graph immediately after accept → toggle should be ON
5. **Check merge:** Execute a task → should merge to feature branch, not main
6. **Run tests:** `cargo test` and `npm run typecheck && npm run lint`

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

## Compilation Unit Notes

All 4 tasks are independent compilation units — no cross-task type/signature changes:
- Tasks 1-2 (backend) modify separate Rust files with no shared type changes
- Tasks 3-4 (frontend) modify separate TS files with no shared type changes
- Backend and frontend tasks have no cross-layer dependencies
- All tasks can be executed in any order
