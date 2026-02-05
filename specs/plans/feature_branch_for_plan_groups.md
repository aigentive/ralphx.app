# Plan: Feature Branch for Plan Groups

## Summary

Add a "feature branch" workflow for plan groups: tasks within a plan merge into an intermediate feature branch instead of main, and a final merge task merges the feature branch into main when all work is complete.

## Key Design Decisions

### Data Model
- New `plan_branches` table linking `plan_artifact_id` → feature branch metadata
- `merge_task_id` column identifies the auto-created "merge to main" task
- Project-level `use_feature_branches` setting (default **true**)
- Per-plan override at accept time

### Merge Target Override
Two lookup paths in `side_effects.rs`:
1. **Regular plan tasks**: task has `plan_artifact_id` → look up `plan_branches` → merge into feature branch (instead of `project.base_branch`)
2. **Merge task**: task is `plan_branches.merge_task_id` → merge feature branch into `project.base_branch`

### Branch Creation Override
When task enters Executing and has a feature branch:
- Create task branch FROM the feature branch (not from main)
- This ensures task work is based on the combined plan progress

### Merge Task Lifecycle
The merge task is a regular task with category `"plan_merge"`:
- Auto-created on plan accept (or mid-plan conversion)
- `blockedBy` all other plan tasks
- Goes through normal flow: Ready → Executing (agent verifies) → Approved → PendingMerge
- PendingMerge handler detects it's a merge task → merges feature branch into main
- Same two-phase merge: programmatic fast path → agent fallback on conflict

### Mid-Plan Conversion
When user enables feature branch after some tasks already merged to main:
1. Create feature branch from current `project.base_branch` HEAD (includes already-merged work)
2. Create merge task, blocked by remaining unmerged plan tasks
3. Future tasks branch from and merge into the feature branch
4. Already-merged tasks are unaffected

### Branch Naming
`ralphx/{project-slug}/plan-{short-artifact-id}` (e.g., `ralphx/my-app/plan-a1b2c3`)

---

## Files to Modify/Create

### Backend - New Files
| File | Purpose |
|------|---------|
| `src-tauri/src/domain/entities/plan_branch.rs` | PlanBranch entity |
| `src-tauri/src/domain/repositories/plan_branch_repository.rs` | Repository trait |
| `src-tauri/src/infrastructure/sqlite/plan_branch_repository.rs` | SQLite impl |
| `src-tauri/src/infrastructure/sqlite/migrations/vN_plan_branches.rs` | Migration |
| `src-tauri/src/commands/plan_branch_commands.rs` | Tauri commands |

### Backend - Modified Files
| File | Change |
|------|--------|
| `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` | Merge target + branch creation override |
| `src-tauri/src/domain/entities/project.rs` | `use_feature_branches` field |
| `src-tauri/src/commands/ideation_commands/ideation_commands_apply.rs` | Create feature branch on accept |
| `src-tauri/src/infrastructure/sqlite/migrations/mod.rs` | Register migration |
| `src-tauri/src/domain/repositories/mod.rs` | Register repo trait |
| `src-tauri/src/infrastructure/sqlite/mod.rs` | Register repo impl |
| `src-tauri/src/main.rs` or `lib.rs` | Register commands |

### Frontend - New Files
| File | Purpose |
|------|---------|
| `src/api/plan-branch.ts` | API module |
| `src/api/plan-branch.schemas.ts` | Zod schemas (snake_case) |
| `src/api/plan-branch.transforms.ts` | Transform fns |
| `src/api/plan-branch.types.ts` | TS types (camelCase) |
| `src/components/TaskGraph/groups/PlanGroupSettings.tsx` | Settings panel |

### Frontend - Modified Files
| File | Change |
|------|--------|
| `src/components/TaskGraph/groups/PlanGroupHeader.tsx` | Show feature branch badge/status |
| `src/api-mock/task-graph.ts` | Mock plan branch data |
| `src/types/task.ts` | (if needed) |
| Project settings component | Add feature branch toggle |

---

## Task Breakdown

### Task 1: Migration + PlanBranch Entity (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(plan_branch): add plan_branches table, PlanBranch entity, and project.use_feature_branches`
**Files:** `vN_plan_branches.rs`, `plan_branch.rs`, `project.rs`, migration `mod.rs`, `entities/mod.rs`

**Compilation Unit Note:** Migration + entity + project field change + module registrations must be in same commit. The `from_row` in `project.rs` will fail without the migration column, and entity won't compile without the `entities/mod.rs` registration. Also register in `domain/repositories/mod.rs` and `infrastructure/sqlite/mod.rs` (module declarations only — the actual files come in Task 2).

```sql
CREATE TABLE IF NOT EXISTS plan_branches (
    id TEXT PRIMARY KEY,
    plan_artifact_id TEXT NOT NULL UNIQUE,
    session_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    source_branch TEXT NOT NULL,  -- what it was created from
    status TEXT NOT NULL DEFAULT 'active',  -- active | merged | abandoned
    merge_task_id TEXT,
    created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
    merged_at TEXT
);
```

Add to projects table:
```sql
ALTER TABLE projects ADD COLUMN use_feature_branches INTEGER NOT NULL DEFAULT 1;
```

PlanBranch entity:
```rust
pub struct PlanBranch {
    pub id: PlanBranchId,
    pub plan_artifact_id: ArtifactId,
    pub session_id: IdeationSessionId,
    pub project_id: ProjectId,
    pub branch_name: String,
    pub source_branch: String,
    pub status: PlanBranchStatus,  // Active, Merged, Abandoned
    pub merge_task_id: Option<TaskId>,
    pub created_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
}
```

### Task 2: PlanBranch Repository (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(plan_branch): add PlanBranchRepository trait and SQLite implementation`
**Files:** `plan_branch_repository.rs` (trait), `sqlite_plan_branch_repo.rs` (impl), `domain/repositories/mod.rs`, `infrastructure/sqlite/mod.rs`

**Compilation Unit Note:** Trait + SQLite impl + module registrations (pub mod + pub use) must be in same commit. Task 1 adds the module declarations; this task adds the actual files. If Task 1 already declared the modules, the files must exist or compilation fails — coordinate: either Task 1 creates stub files, or defer module declarations to this task. **Recommended:** Task 1 does NOT add module declarations for the repository — Task 2 adds both the files AND the `pub mod`/`pub use` in `repositories/mod.rs` and `sqlite/mod.rs`.

Trait methods:
- `create(branch: PlanBranch) -> Result<PlanBranch>`
- `get_by_plan_artifact_id(id: &ArtifactId) -> Result<Option<PlanBranch>>`
- `get_by_merge_task_id(task_id: &TaskId) -> Result<Option<PlanBranch>>`
- `get_by_project_id(project_id: &ProjectId) -> Result<Vec<PlanBranch>>`
- `update_status(id: &PlanBranchId, status: PlanBranchStatus) -> Result<()>`
- `set_merge_task_id(id: &PlanBranchId, task_id: &TaskId) -> Result<()>`
- `set_merged(id: &PlanBranchId) -> Result<()>`

### Task 3: Git Service - Feature Branch Operations
**Dependencies:** None (additive methods, no existing signature changes)
**Atomic Commit:** `feat(git): add create_feature_branch and delete_feature_branch to GitService`
**Files:** `git_service.rs`

New methods:
- `create_feature_branch(repo_path, branch_name, source_branch)` — creates branch without checkout
- `delete_feature_branch(repo_path, branch_name)` — cleanup after merge

### Task 4: Merge Target Override in Side Effects (CORE, BLOCKING)
**Dependencies:** Task 2, Task 3
**Atomic Commit:** `feat(state_machine): override branch creation and merge target for plan feature branches`
**Files:** `side_effects.rs`, `context.rs` (TaskServices)

**Compilation Unit Note:** This task MUST also modify `context.rs` to add `plan_branch_repo: Option<Arc<dyn PlanBranchRepository>>` to `TaskServices` with a `with_plan_branch_repo()` builder method and update `new_mock()`. The `side_effects.rs` helpers access the repo via `self.machine.context().services.plan_branch_repo`. Both files must change together — `side_effects.rs` won't compile without the `TaskServices` field, and the field won't compile without the `PlanBranchRepository` trait (Task 2). Also update `TaskTransitionService` construction site (wherever `TaskServices` is built with `.with_plan_branch_repo()`) — likely in `app_state.rs` or the command handler that creates the service.

**Change 1 - Branch creation (Executing state, ~line 263):**
```rust
// Before: always uses project base branch
let base_branch = project.base_branch.as_deref().unwrap_or("main");

// After: check for feature branch
let base_branch = resolve_task_base_branch(&task, &project, &plan_branch_repo).await;
```

Helper: `resolve_task_base_branch` checks if task has `plan_artifact_id` → looks up `plan_branches` → returns feature branch name or falls back to project base branch.

**Change 2 - Merge target (PendingMerge state, ~line 817):**
```rust
// Before: always merges into project base branch
let base_branch = project.base_branch.as_deref().unwrap_or("main");

// After: check if merge task or plan task
let (source_branch, target_branch) = resolve_merge_branches(&task, &project, &plan_branch_repo).await;
```

Helper: `resolve_merge_branches` returns:
- **Merge task**: `(feature_branch, project.base_branch)` — merge feature into main
- **Regular plan task with feature branch**: `(task_branch, feature_branch)` — merge task into feature
- **Regular task (no feature branch)**: `(task_branch, project.base_branch)` — normal behavior

**Change 3 - Post-merge cleanup for merge task:**
After successful merge of feature branch:
- Update `plan_branch.status = Merged`
- Delete the feature branch
- Emit plan-merge-complete event

### Task 5: Tauri Commands for Plan Branches (BLOCKING)
**Dependencies:** Task 2, Task 3
**Atomic Commit:** `feat(commands): add plan branch Tauri commands and register in lib.rs`
**Files:** `plan_branch_commands.rs`, `commands/mod.rs`, `lib.rs`

**Compilation Unit Note:** Command file + `commands/mod.rs` registration + `lib.rs` `generate_handler!` registration must all be in same commit. The `enable_feature_branch` command needs `PlanBranchRepository` (Task 2) and `GitService::create_feature_branch` (Task 3). Must also register in `lib.rs` invoke_handler macro or clippy will warn about dead code with `-D warnings`.

Commands:
- `get_plan_branch(plan_artifact_id)` → returns PlanBranch or null
- `get_project_plan_branches(project_id)` → returns Vec<PlanBranch>
- `enable_feature_branch(input: EnableFeatureBranchInput)` → creates feature branch mid-plan
  - Input: `{ plan_artifact_id, session_id, project_id }`
  - Creates git branch, DB record, merge task with blockedBy on unmerged plan tasks
- `disable_feature_branch(plan_artifact_id)` → only if no tasks merged to it yet
- `update_project_feature_branch_setting(project_id, enabled: bool)`

### Task 6: Accept Plan Integration (Transactional)
**Dependencies:** Task 2, Task 3, Task 4, Task 5
**Atomic Commit:** `feat(ideation): create feature branch and merge task on plan accept`
**Files:** `ideation_commands_apply.rs`, `ideation_commands_types.rs`

**Compilation Unit Note:** `ApplyProposalsInput` struct change (add `use_feature_branch: Option<bool>`) in `ideation_commands_types.rs` + handler logic in `ideation_commands_apply.rs` must be in same commit. Adding a new `Option<bool>` field to a `Deserialize` struct is additive (defaults to `None` for missing JSON fields), so the frontend doesn't break. Depends on Task 4 because the feature branch must be looked up during execution (side_effects), and Task 5 because mid-plan conversion uses the same commands.

**Race condition concern:** The scheduler picks up `Ready` tasks and starts execution (branching from whatever base). We MUST ensure the feature branch, merge task, and all dependencies exist BEFORE any task reaches `Ready` status.

**Current flow in `apply_proposals_to_kanban`:**
```
Phase 1: Create all tasks (status = Backlog)          ← safe, not schedulable
Phase 2: Add proposal dependencies                     ← safe, still Backlog
Phase 3: Upgrade statuses to Ready/Blocked             ← DANGER: scheduler triggers
Phase 4: Emit events + trigger scheduler (600ms delay) ← tasks get picked up
```

**Modified flow — inject feature branch setup between Phase 2 and Phase 3:**
```
Phase 1: Create all tasks (status = Backlog)           ← existing, unchanged
Phase 2: Add proposal dependencies                      ← existing, unchanged
Phase 2.5: Feature branch setup (NEW):
   a. Resolve: project.use_feature_branches OR input.use_feature_branch override
   b. If enabled:
      - Create git feature branch from project.base_branch
      - Insert plan_branches DB record
      - Create merge task (status = Backlog, category = "plan_merge")
      - Add blockedBy dependencies: merge_task → all created plan tasks
      - Set plan_branches.merge_task_id
Phase 3: Upgrade statuses to Ready/Blocked              ← existing, now includes merge task
Phase 4: Emit events + trigger scheduler                ← everything wired up
```

**Why this is safe:** Tasks only become `Ready` in Phase 3. By Phase 2.5, the feature branch exists, the merge task has dependencies, and `resolve_task_base_branch` (Task 4) will find the `plan_branches` record when the scheduler eventually runs.

**Error handling:** If the git branch creation (Phase 2.5b) fails:
- Tasks are still `Backlog` — not schedulable, no harm done
- No plan_branches record inserted — tasks will use normal `project.base_branch`
- Return error to user, they can retry

**Ideal: DB transaction wrapper** — Wrap Phases 1-3 in a single SQLite transaction so partial failures don't leave orphaned records. The git operation (branch creation) is idempotent and happens within the transaction scope but outside the DB transaction itself. If it fails, the transaction rolls back cleanly.

**Accept input extension:**
```rust
pub struct ApplyProposalsInput {
    // ... existing fields ...
    pub use_feature_branch: Option<bool>,  // per-plan override (None = use project default)
}
```

### Task 7: Frontend API Layer (BLOCKING)
**Dependencies:** Task 5 (backend commands must exist for real API calls)
**Atomic Commit:** `feat(api): add plan-branch API layer with schemas, transforms, and types`
**Files:** `src/api/plan-branch.ts`, `src/api/plan-branch.schemas.ts`, `src/api/plan-branch.transforms.ts`, `src/api/plan-branch.types.ts`, `src/lib/tauri.ts` (re-export planBranchApi)

Schema (snake_case, matching Rust):
```typescript
const PlanBranchSchema = z.object({
    id: z.string(),
    plan_artifact_id: z.string(),
    session_id: z.string(),
    project_id: z.string(),
    branch_name: z.string(),
    source_branch: z.string(),
    status: z.enum(["active", "merged", "abandoned"]),
    merge_task_id: z.string().nullable(),
    created_at: z.string(),
    merged_at: z.string().nullable(),
});
```

Types (camelCase):
```typescript
interface PlanBranch {
    id: string;
    planArtifactId: string;
    sessionId: string;
    projectId: string;
    branchName: string;
    sourceBranch: string;
    status: "active" | "merged" | "abandoned";
    mergeTaskId: string | null;
    createdAt: string;
    mergedAt: string | null;
}
```

API:
```typescript
export const planBranchApi = {
    getByPlan: (planArtifactId) => typedInvokeWithTransform(...),
    getByProject: (projectId) => typedInvokeWithTransform(...),
    enable: (input) => typedInvokeWithTransform(...),
    disable: (planArtifactId) => typedInvoke(...),
} as const;
```

### Task 8: Plan Group Settings Panel (UI)
**Dependencies:** Task 7
**Atomic Commit:** `feat(task-graph): add PlanGroupSettings panel and feature branch badge to PlanGroupHeader`
**Files:** `src/components/TaskGraph/groups/PlanGroupSettings.tsx`, `src/components/TaskGraph/groups/PlanGroupHeader.tsx`

**PlanGroupSettings panel** (opens from plan group header):
- Feature branch toggle (enable/disable)
- Branch name display (read-only when active)
- Branch status badge (active/merged)
- "Enable Feature Branch" button (mid-plan conversion)
- Warning if tasks already merged to main
- Merge task link (opens task detail in graph split layout)

**PlanGroupHeader changes:**
- Show git branch icon + name when feature branch is active
- Status indicator: active (green dot), merged (check), abandoned (x)
- Gear icon → opens PlanGroupSettings

### Task 9: Mock Layer + Project Settings UI
**Dependencies:** Task 7
**Atomic Commit:** `feat(mock): add plan branch mock data and project feature branch toggle`
**Files:** `src/api-mock/task-graph.ts` (or new `src/api-mock/plan-branch.ts`), `src/api-mock/index.ts`, project settings component

- Add plan branch mock data
- Add feature branch toggle to project settings form
- Update `apply_proposals` mock to handle `use_feature_branch` param

### Task 10: Backend Tests
**Dependencies:** Task 4, Task 6
**Atomic Commit:** `test(plan_branch): add unit tests for entity, repository, side effects, and accept flow`

- PlanBranch entity unit tests
- Repository CRUD tests
- `resolve_task_base_branch` tests (feature branch vs default)
- `resolve_merge_branches` tests (merge task vs plan task vs regular task)
- Migration tests
- Accept plan with feature branch test

---

## Edge Cases

| Scenario | Handling |
|----------|----------|
| Task already merged to main, then feature branch enabled | Feature branch created from current main HEAD (includes merged work). Only unmerged tasks redirect. |
| Feature branch disabled after tasks merged to it | Block if tasks already merged to feature branch. Only allow when no merges yet. |
| Main advances while feature branch active | Final merge task handles via rebase/conflict resolution (existing two-phase merge). |
| Merge task conflicts | Same agent-assisted merge flow as any task. Agent resolves on feature→main merge. |
| Worktree mode | Feature branch is just a git branch, no worktree. Task worktrees still created from feature branch. |
| Task outside plan depends on plan task | Works fine - the dependency is task-to-task, independent of branching. |
| Plan task depends on non-plan task | Works fine - non-plan task merges to main, plan task branches from feature branch (which was created from main). Feature branch needs rebase if non-plan task merges after feature branch creation. |

## Dependency Graph

```
Task 1 (Entity + Migration) ──┐
                               ├──→ Task 2 (Repository) ──┬──→ Task 4 (Side Effects) ──┬──→ Task 6 (Accept Integration) ──→ Task 10 (Tests)
Task 3 (Git Service) ─────────┘                           │                             │
                                                          └──→ Task 5 (Commands) ───────┘
                                                                      │
                                                                      └──→ Task 7 (Frontend API) ──┬──→ Task 8 (UI Components)
                                                                                                    └──→ Task 9 (Mock + Settings)
```

**Parallelizable groups:**
- **Group A** (no deps): Task 1, Task 3 — can run in parallel
- **Group B** (after Group A): Task 2
- **Group C** (after Task 2+3): Task 4, Task 5 — can run in parallel
- **Group D** (after Task 4+5): Task 6
- **Group E** (after Task 5): Task 7
- **Group F** (after Task 7): Task 8, Task 9 — can run in parallel
- **Group G** (after Task 4+6): Task 10

## Compilation Unit Warnings

| Risk | Tasks | Issue | Resolution |
|------|-------|-------|------------|
| Module declaration timing | 1 → 2 | Task 1 must NOT declare `pub mod plan_branch_repository` in `repositories/mod.rs` — the file doesn't exist yet | Task 2 adds both files AND module declarations |
| TaskServices field | 4 | `side_effects.rs` needs `plan_branch_repo` on `TaskServices` | Task 4 must also modify `context.rs` |
| TaskServices construction | 4 | Builder `.with_plan_branch_repo()` must be called where `TaskServices` is constructed | Find all `TaskServices::new()` call sites and add `.with_plan_branch_repo()` |
| ApplyProposalsInput | 6 | New field must be `Option<bool>` (serde defaults to None for missing fields) | Safe — frontend won't break until it sends the new field |
| Command registration | 5 | Dead code warning if commands exist but aren't registered in `generate_handler!` | Task 5 must include `lib.rs` registration |

## Verification

1. **Unit tests**: `cargo test` for all new backend code
2. **Type check**: `npm run typecheck` for frontend
3. **Lint**: `cargo clippy` + `npm run lint`
4. **Manual test flow**:
   - Create project with `use_feature_branches = true`
   - Create ideation session → accept plan → verify feature branch created
   - Execute a plan task → verify it branches from feature branch
   - Merge task → verify it merges into feature branch
   - Complete all tasks → merge task unblocks → verify feature branch merges into main
5. **Mid-plan conversion test**:
   - Create plan without feature branch
   - Merge 1 task to main
   - Enable feature branch
   - Verify remaining tasks target feature branch
   - Verify merge task only blocked by remaining tasks

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
- Run `cargo clippy --all-targets --all-features -- -D warnings` for backend tasks
- Run `npm run lint && npm run typecheck` for frontend tasks
- Use `cargo check` as a quick compilation verification before full clippy
