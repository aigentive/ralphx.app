# Plan: Migrate plan_branches primary lookup from plan_artifact_id to session_id

## Context

`plan_branches` uses `plan_artifact_id` as its UNIQUE key and primary lookup. But `plan_artifact_id` is **optional** on ideation sessions — sessions without a plan artifact (common case) can't get feature branches auto-created, and tasks from these sessions can't find their feature branch during merge resolution.

**Real-world evidence** (task `82a21731`):
- Task has `plan_artifact_id = NULL`, `ideation_session_id = fe3c4282-...`
- Plan branch exists with `session_id = fe3c4282-...`, status = active
- `resolve_merge_branches()` checks `task.plan_artifact_id` → NULL → returns `target: main` instead of feature branch
- No merge task was auto-created because `apply_proposals_to_kanban` skipped feature branch creation (gated on `if let Some(plan_artifact_id)`)

**3 bugs fixed:**
1. **Merge targets main instead of feature branch** — `side_effects.rs` can't find plan branch via NULL `plan_artifact_id`
2. **Feature branch not auto-created on Accept Plan** — `apply_proposals_to_kanban` gates on `plan_artifact_id` being `Some`
3. **No merge task auto-created** — consequence of #2 (branch not created → merge task not created)

**Also fixes (UI):**
4. **Stale toggle after enable** — error handling swallows message, doesn't invalidate cache

## Strategy

Add `get_by_session_id()` as the new primary lookup. Keep `get_by_plan_artifact_id()` for backward compat. No table recreation needed — just add a UNIQUE index on `session_id`.

---

## Step 1: Migration — Add UNIQUE index on session_id (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(db): add UNIQUE index on plan_branches.session_id`

**New file:** `src-tauri/src/infrastructure/sqlite/migrations/v16_plan_branch_session_index.rs`
```sql
CREATE UNIQUE INDEX IF NOT EXISTS idx_plan_branches_session_id ON plan_branches(session_id);
```

**Modify:** `src-tauri/src/infrastructure/sqlite/migrations/mod.rs`
- Register migration, bump SCHEMA_VERSION to 16

**New file:** `src-tauri/src/infrastructure/sqlite/migrations/v16_plan_branch_session_index_tests.rs`

---

## Step 2: Repository — Add `get_by_session_id()` (BLOCKING)
**Dependencies:** Step 1
**Atomic Commit:** `feat(plan-branch): add get_by_session_id to PlanBranchRepository trait`

> **Compilation Unit Note:** Adding a method to a trait requires ALL implementors to be
> updated in the same task. Verified: only 2 impls exist (SQLite, Memory) — both covered here.
> No additional test mocks implement PlanBranchRepository (grep confirmed).

**Modify:** `src-tauri/src/domain/repositories/plan_branch_repository.rs`
- Add `async fn get_by_session_id(&self, session_id: &IdeationSessionId) -> AppResult<Option<PlanBranch>>`
- Keep `get_by_plan_artifact_id` (deprecated but functional)

**Modify:** `src-tauri/src/infrastructure/sqlite/sqlite_plan_branch_repo.rs`
- Implement `get_by_session_id`: `SELECT * FROM plan_branches WHERE session_id = ?1`
- Add tests

**Modify:** `src-tauri/src/infrastructure/memory/memory_plan_branch_repo.rs`
- Implement `get_by_session_id`: find by `b.session_id == *session_id`

---

## Step 3: Fix merge resolution — `side_effects.rs`
**Dependencies:** Step 2
**Atomic Commit:** `fix(plan-branch): resolve merge branches via session_id instead of plan_artifact_id`

> **Compilation Unit Note:** Uses `get_by_session_id()` added in Step 2 (additive call, no signature changes).
> Tests in this file use `make_task` helper with `plan_artifact_id` — must add `ideation_session_id`
> field to test helpers and update assertions to test session_id-based lookup.

**Modify:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`

### `resolve_task_base_branch()` (~line 211)
```
BEFORE: task.plan_artifact_id → get_by_plan_artifact_id()
AFTER:  task.ideation_session_id → get_by_session_id()
```

### `resolve_merge_branches()` (~line 244)
```
BEFORE (Path 2, line 270):
  if let Some(ref plan_artifact_id) = task.plan_artifact_id {
      plan_branch_repo.get_by_plan_artifact_id(plan_artifact_id)

AFTER:
  if let Some(ref session_id) = task.ideation_session_id {
      plan_branch_repo.get_by_session_id(session_id)
```

### Test updates (~line 1152+)
- Update `make_task()` to accept `ideation_session_id` parameter
- Update `make_plan_branch()` to use consistent session IDs
- Update all tests that exercise plan branch lookup to set `task.ideation_session_id`

This directly fixes Bug #1 — task `82a21731` has `ideation_session_id` set, so the lookup will find the active feature branch.

---

## Step 4: Fix auto-create on Accept Plan — `ideation_commands_apply.rs`
**Dependencies:** Step 2
**Atomic Commit:** `fix(plan-branch): remove plan_artifact_id gate from apply_proposals_to_kanban`

> **Compilation Unit Note:** Uses `get_by_session_id()` added in Step 2 (additive call).
> Modifies conditional logic around `plan_artifact_id` — no signature changes, no cross-file impact.

**Modify:** `src-tauri/src/commands/ideation_commands/ideation_commands_apply.rs`

### Remove plan_artifact_id gate on feature branch creation (~line 207-289)

> **CRITICAL FK SAFETY:** `tasks.plan_artifact_id` has a FK to the `artifacts` table.
> `plan_branches.plan_artifact_id` does NOT (just `TEXT NOT NULL UNIQUE`).
> Phase 96 fixed this FK violation — do NOT re-introduce it by setting session_id
> into `tasks.plan_artifact_id`.

**Two separate concerns:**

#### 1. Task plan_artifact_id propagation (line 171) — KEEP AS-IS
```
// Phase 96 fix: only set when real artifact exists. DO NOT CHANGE.
if let Some(ref artifact_id) = plan_artifact_id {
    // set tasks.plan_artifact_id = artifact_id (real artifact, FK-safe)
}
```

#### 2. Feature branch creation (line 207) — REMOVE GATE
```
BEFORE (line 207-208):
  if use_feature_branch {
      if let Some(ref artifact_id) = plan_artifact_id {
          // create branch — SKIPPED when plan_artifact_id is None

AFTER:
  if use_feature_branch {
      // Compute effective_plan_id for plan_branches table only (no FK constraint)
      let effective_plan_id: ArtifactId = session.plan_artifact_id.clone()
          .unwrap_or_else(|| ArtifactId::from_string(session_id.as_str().to_string()));
      // Always check/create — session_id is always available
```

- **Keep** Phase 96's `tasks.plan_artifact_id` behavior (only set when real artifact exists)
- **Remove** the `if let Some` gate around feature branch creation block
- **Use** `effective_plan_id` only for `plan_branches.plan_artifact_id` (no FK on that table)
- Feature branch existence check: `get_by_session_id(&session_id)` instead of `get_by_plan_artifact_id`
- Branch creation always proceeds when `use_feature_branch` is true (session_id always available)
- Merge task: set `ideation_session_id` always, `plan_artifact_id` only if real artifact exists

This fixes Bug #2 and #3.

---

## Step 5: Fix plan branch commands — `plan_branch_commands.rs`
**Dependencies:** Step 2
**Atomic Commit:** `fix(plan-branch): session-first lookup in get/enable/disable commands`

> **Compilation Unit Note:** Uses `get_by_session_id()` added in Step 2 (additive call).
> Keeps existing params for frontend compat — no frontend changes needed for this step.

**Modify:** `src-tauri/src/commands/plan_branch_commands.rs`

### `get_plan_branch` (~line 63)
- Keep `plan_artifact_id: String` param (frontend compat — graph sends session_id as fallback already)
- Internally: try `get_by_session_id()` first, fallback to `get_by_plan_artifact_id()`

### `enable_feature_branch` (~line 105)
- Existence check: `get_by_session_id(&session_id)` instead of `get_by_plan_artifact_id`
- Re-fetch after creation: `get_by_session_id(&session_id)`

### `disable_feature_branch` (~line 263)
- Keep `plan_artifact_id: String` param (frontend compat)
- Try `get_by_session_id()` first, fallback to `get_by_plan_artifact_id()`

---

## Step 6: Fix frontend error handling — `PlanGroupSettings.tsx`
**Dependencies:** None (independent frontend change)
**Atomic Commit:** `fix(graph): fix stale toggle after feature branch enable`

> **Compilation Unit Note:** Pure frontend change, no backend dependencies.
> Can be executed in parallel with Steps 3-5.

**Modify:** `src/components/TaskGraph/groups/PlanGroupSettings.tsx`

### `handleToggle` catch/finally block
- Move `onBranchChange?.()` to `finally` (always invalidate cache, not just on success)
- Handle Tauri string errors: `typeof err === "string" ? err : ...`
- Silence "already exists" error (just stale UI, refetch will fix it)

---

## ~~Step 7: Update mock repos used in tests~~ (MERGED INTO STEP 2)

> **Compilation Unit Validation:** Grep confirmed only 2 implementations of
> `PlanBranchRepository` exist: `SqlitePlanBranchRepository` and `MemoryPlanBranchRepository`.
> No additional test mocks implement the trait (files like `apply_service/tests.rs` and
> `task_context_service.rs` reference `plan_branch_repo` but use the Memory impl via `AppState`).
> All implementors are already covered in Step 2.

---

## Files Summary

| File | Change |
|------|--------|
| `migrations/v16_plan_branch_session_index.rs` | **New** — UNIQUE index on session_id |
| `migrations/v16_plan_branch_session_index_tests.rs` | **New** — migration tests |
| `migrations/mod.rs` | Register v16, bump version |
| `repositories/plan_branch_repository.rs` | Add `get_by_session_id` to trait |
| `sqlite_plan_branch_repo.rs` | Implement + tests |
| `memory_plan_branch_repo.rs` | Implement |
| `side_effects.rs` | Use `ideation_session_id` → `get_by_session_id` |
| `plan_branch_commands.rs` | Session-first lookup in get/enable/disable |
| `ideation_commands_apply.rs` | Remove plan_artifact_id gate, use session_id |
| `PlanGroupSettings.tsx` | Fix error handling + cache invalidation |
| Test mocks (~3 files) | Add `get_by_session_id` stub |

## Compilation Order

Each step compiles independently:
1. Migration (no code deps) — **BLOCKING** Steps 2+
2. Repo trait + ALL impls (additive, single compilation unit) — **BLOCKING** Steps 3-5
3. side_effects.rs (uses new method from Step 2)
4. ideation_commands_apply.rs (uses new method from Step 2)
5. plan_branch_commands.rs (uses new method from Step 2)
6. Frontend fix (independent — can run in parallel with Steps 1-5)
7. ~~Test mocks~~ (merged into Step 2 — no separate mocks exist)

**Dependency graph:**
```
Step 1 → Step 2 → Step 3
                 → Step 4
                 → Step 5
Step 6 (independent, parallel)
```

Steps 3, 4, 5 depend on Step 2 but are independent of each other.

## Verification

```bash
# Backend compiles and passes tests
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml

# Frontend lints
npm run lint && npm run typecheck

# DB verification (after running app)
sqlite3 src-tauri/ralphx.db "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_plan_branches_session_id';"
# Should return: idx_plan_branches_session_id
```

### Manual test scenario:
1. Create ideation session (no plan artifact)
2. Add proposals, click "Accept Plan"
3. Verify: feature branch auto-created, merge task exists, tasks grouped under plan
4. Execute a task → verify merge target is feature branch (not main)
5. Toggle feature branch OFF/ON in settings → verify UI updates immediately

## Compilation Unit Validation

| Step | Adds new trait method? | Renames/removes? | All callers in same step? | Compiles alone? |
|------|----------------------|-----------------|--------------------------|----------------|
| 1 | No | No | N/A (SQL only) | ✅ |
| 2 | **Yes** (`get_by_session_id`) | No | ✅ All 2 impls included | ✅ |
| 3 | No | No | N/A (additive call) | ✅ (after Step 2) |
| 4 | No | No | N/A (additive call) | ✅ (after Step 2) |
| 5 | No | No | N/A (additive call) | ✅ (after Step 2) |
| 6 | No | No | N/A (frontend only) | ✅ |

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
