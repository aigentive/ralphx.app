# RalphX - Phase 85: Feature Branch for Plan Groups

## Overview

Add a "feature branch" workflow for plan groups: tasks within a plan merge into an intermediate feature branch instead of main, and a final merge task merges the feature branch into main when all work is complete. This isolates plan work from the main branch until the entire plan is verified and approved, reducing merge conflicts between concurrent plans and providing a clean integration point.

**Reference Plan:**
- `specs/plans/feature_branch_for_plan_groups.md` - Detailed architecture for plan branch entity, repository, side effects override, commands, accept integration, and frontend API/UI

## Goals

1. **Isolate plan work** — tasks within a plan merge into a feature branch instead of main, preventing incomplete work from landing on the default branch
2. **Auto-create merge task** — on plan accept, automatically create a merge task that is blocked by all plan tasks and merges the feature branch into main when all work completes
3. **Mid-plan conversion** — allow enabling feature branches after some tasks have already been merged to main
4. **Project-level default** — `use_feature_branches` project setting with per-plan override at accept time

## Dependencies

### Phase 84 (Merge Dependency Unblock Fix) - Required

| Dependency | Why Needed |
|------------|------------|
| Deferred dependency unblocking (Merged, not Approved) | Merge task must wait for actual merge completion before unblocking |
| Two-phase merge workflow (PendingMerge → Merging → Merged) | Feature branch merge uses the same programmatic + agent fallback flow |
| GitService branch/worktree operations | Feature branch creation/deletion extends existing git service |
| TaskServices DI pattern with builder methods | Plan branch repository follows the same `Option<Arc<dyn Repo>>` pattern |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/feature_branch_for_plan_groups.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/feature_branch_for_plan_groups.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add plan_branches migration, PlanBranch entity, PlanBranchStatus enum, PlanBranchId newtype, and use_feature_branches column to projects table",
    "plan_section": "Task 1: Migration + PlanBranch Entity",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(plan_branch): add plan_branches table, PlanBranch entity, and project.use_feature_branches",
    "steps": [
      "Read specs/plans/feature_branch_for_plan_groups.md section 'Task 1: Migration + PlanBranch Entity'",
      "Create migration file src-tauri/src/infrastructure/sqlite/migrations/v13_plan_branches.rs with CREATE TABLE plan_branches and ALTER TABLE projects ADD COLUMN use_feature_branches",
      "Register migration in migrations/mod.rs: bump SCHEMA_VERSION to 13, add to MIGRATIONS array",
      "Create src-tauri/src/domain/entities/plan_branch.rs with PlanBranch struct, PlanBranchStatus enum (Active/Merged/Abandoned), PlanBranchId newtype, from_row impl",
      "Register in domain/entities/mod.rs: pub mod plan_branch, re-export PlanBranch, PlanBranchId, PlanBranchStatus",
      "Add use_feature_branches: bool field to Project struct in project.rs, update from_row to read the column",
      "Add migration test file v13_plan_branches_tests.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(plan_branch): add plan_branches table, PlanBranch entity, and project.use_feature_branches"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add PlanBranchRepository trait and SQLite implementation with module registrations",
    "plan_section": "Task 2: PlanBranch Repository",
    "blocking": [4, 5],
    "blockedBy": [1],
    "atomic_commit": "feat(plan_branch): add PlanBranchRepository trait and SQLite implementation",
    "steps": [
      "Read specs/plans/feature_branch_for_plan_groups.md section 'Task 2: PlanBranch Repository'",
      "Create src-tauri/src/domain/repositories/plan_branch_repository.rs with async trait: create, get_by_plan_artifact_id, get_by_merge_task_id, get_by_project_id, update_status, set_merge_task_id, set_merged",
      "Register in domain/repositories/mod.rs: pub mod plan_branch_repository, re-export PlanBranchRepository",
      "Create src-tauri/src/infrastructure/sqlite/sqlite_plan_branch_repo.rs implementing the trait with rusqlite queries",
      "Register in infrastructure/sqlite/mod.rs: pub mod sqlite_plan_branch_repo, pub use SqlitePlanBranchRepository",
      "Write repository unit tests (CRUD operations, get_by_* queries)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(plan_branch): add PlanBranchRepository trait and SQLite implementation"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add create_feature_branch and delete_feature_branch methods to GitService",
    "plan_section": "Task 3: Git Service - Feature Branch Operations",
    "blocking": [4, 5],
    "blockedBy": [],
    "atomic_commit": "feat(git): add create_feature_branch and delete_feature_branch to GitService",
    "steps": [
      "Read specs/plans/feature_branch_for_plan_groups.md section 'Task 3: Git Service - Feature Branch Operations'",
      "Add GitService::create_feature_branch(repo_path: &Path, branch_name: &str, source_branch: &str) -> AppResult<()> — creates branch without checkout using git branch <name> <source>",
      "Add GitService::delete_feature_branch(repo_path: &Path, branch_name: &str) -> AppResult<()> — cleanup after merge using delete_branch with force=false",
      "Write unit tests for both methods",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(git): add create_feature_branch and delete_feature_branch to GitService"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Override branch creation and merge target in side_effects.rs for plan feature branches, add plan_branch_repo to TaskServices in context.rs",
    "plan_section": "Task 4: Merge Target Override in Side Effects (CORE)",
    "blocking": [6, 10],
    "blockedBy": [2, 3],
    "atomic_commit": "feat(state_machine): override branch creation and merge target for plan feature branches",
    "steps": [
      "Read specs/plans/feature_branch_for_plan_groups.md section 'Task 4: Merge Target Override in Side Effects'",
      "Add plan_branch_repo: Option<Arc<dyn PlanBranchRepository>> to TaskServices in context.rs",
      "Add with_plan_branch_repo() builder method to TaskServices, update new_mock() and Debug impl",
      "Wire plan_branch_repo in all TaskServices construction sites (search for .with_task_repo() calls — add .with_plan_branch_repo() alongside)",
      "In side_effects.rs, add helper fn resolve_task_base_branch(&task, &project, plan_branch_repo) -> String — checks task.plan_artifact_id → plan_branches → feature branch name or project.base_branch",
      "In side_effects.rs, add helper fn resolve_merge_branches(&task, &project, plan_branch_repo) -> (String, String) — returns (source, target) based on merge task vs plan task vs regular task",
      "Change 1: In on_enter(Executing), replace hardcoded base_branch with resolve_task_base_branch call",
      "Change 2: In on_enter(PendingMerge), replace hardcoded base_branch with resolve_merge_branches call",
      "Change 3: After successful merge of merge task, update plan_branch status to Merged, delete feature branch, emit plan-merge-complete event",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(state_machine): override branch creation and merge target for plan feature branches"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Add Tauri commands for plan branch CRUD and project setting, register in lib.rs",
    "plan_section": "Task 5: Tauri Commands for Plan Branches",
    "blocking": [6, 7],
    "blockedBy": [2, 3],
    "atomic_commit": "feat(commands): add plan branch Tauri commands and register in lib.rs",
    "steps": [
      "Read specs/plans/feature_branch_for_plan_groups.md section 'Task 5: Tauri Commands for Plan Branches'",
      "Create src-tauri/src/commands/plan_branch_commands.rs with commands: get_plan_branch, get_project_plan_branches, enable_feature_branch, disable_feature_branch, update_project_feature_branch_setting",
      "enable_feature_branch: creates git branch via GitService, inserts DB record, creates merge task with blockedBy on unmerged plan tasks",
      "disable_feature_branch: validates no tasks merged to feature branch yet, removes DB record and git branch",
      "Register module in commands/mod.rs",
      "Register all 5 commands in lib.rs generate_handler! macro",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(commands): add plan branch Tauri commands and register in lib.rs"
    ],
    "passes": true
  },
  {
    "id": 6,
    "category": "backend",
    "description": "Integrate feature branch setup into apply_proposals_to_kanban flow between Phase 2 (dependencies) and Phase 3 (status upgrade)",
    "plan_section": "Task 6: Accept Plan Integration (Transactional)",
    "blocking": [10],
    "blockedBy": [2, 3, 4, 5],
    "atomic_commit": "feat(ideation): create feature branch and merge task on plan accept",
    "steps": [
      "Read specs/plans/feature_branch_for_plan_groups.md section 'Task 6: Accept Plan Integration'",
      "Add use_feature_branch: Option<bool> field to ApplyProposalsInput in ideation_commands_types.rs (serde defaults to None for missing JSON)",
      "In apply_proposals_to_kanban, after dependency creation (Phase 2) and before status upgrade (Phase 3), insert Phase 2.5:",
      "  Phase 2.5a: Resolve feature branch setting — input.use_feature_branch OR project.use_feature_branches",
      "  Phase 2.5b: If enabled: create git feature branch from project.base_branch using GitService::create_feature_branch",
      "  Phase 2.5c: Insert plan_branches DB record via PlanBranchRepository::create",
      "  Phase 2.5d: Create merge task (status=Backlog, category='plan_merge', title='Merge plan: {plan_name} into {base_branch}')",
      "  Phase 2.5e: Add blockedBy dependencies: merge_task blocked by all created plan tasks",
      "  Phase 2.5f: Set plan_branches.merge_task_id via PlanBranchRepository::set_merge_task_id",
      "Handle error: if git branch creation fails, return error (tasks are still Backlog, safe to retry)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(ideation): create feature branch and merge task on plan accept"
    ],
    "passes": true
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Add plan-branch frontend API layer with Zod schemas (snake_case), transform functions, TypeScript types (camelCase), and API wrapper",
    "plan_section": "Task 7: Frontend API Layer",
    "blocking": [8, 9],
    "blockedBy": [5],
    "atomic_commit": "feat(api): add plan-branch API layer with schemas, transforms, and types",
    "steps": [
      "Read specs/plans/feature_branch_for_plan_groups.md section 'Task 7: Frontend API Layer'",
      "Create src/api/plan-branch.schemas.ts with PlanBranchSchema (snake_case fields matching Rust serialization)",
      "Create src/api/plan-branch.types.ts with PlanBranch interface (camelCase fields)",
      "Create src/api/plan-branch.transforms.ts with transformPlanBranch function (snake_case → camelCase)",
      "Create src/api/plan-branch.ts with planBranchApi object: getByPlan, getByProject, enable, disable, updateProjectSetting — using typedInvokeWithTransform",
      "Re-export planBranchApi in src/lib/tauri.ts (add to realApi aggregate)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(api): add plan-branch API layer with schemas, transforms, and types"
    ],
    "passes": true
  },
  {
    "id": 8,
    "category": "frontend",
    "description": "Add PlanGroupSettings panel and feature branch badge/status indicator to PlanGroupHeader",
    "plan_section": "Task 8: Plan Group Settings Panel (UI)",
    "blocking": [],
    "blockedBy": [7],
    "atomic_commit": "feat(task-graph): add PlanGroupSettings panel and feature branch badge to PlanGroupHeader",
    "steps": [
      "Read specs/plans/feature_branch_for_plan_groups.md section 'Task 8: Plan Group Settings Panel (UI)'",
      "Create src/components/TaskGraph/groups/PlanGroupSettings.tsx with: feature branch toggle, branch name display (read-only when active), branch status badge, 'Enable Feature Branch' button for mid-plan conversion, warning if tasks already merged to main, merge task link",
      "Modify PlanGroupHeader.tsx: add git branch icon + name when feature branch active, status indicator (green dot=active, check=merged, x=abandoned), gear icon to open PlanGroupSettings",
      "Follow design system: accent #ff6b35, SF Pro font, no purple/blue gradients",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): add PlanGroupSettings panel and feature branch badge to PlanGroupHeader"
    ],
    "passes": true
  },
  {
    "id": 9,
    "category": "frontend",
    "description": "Add plan branch mock data for web mode and feature branch toggle to project settings",
    "plan_section": "Task 9: Mock Layer + Project Settings UI",
    "blocking": [],
    "blockedBy": [7],
    "atomic_commit": "feat(mock): add plan branch mock data and project feature branch toggle",
    "steps": [
      "Read specs/plans/feature_branch_for_plan_groups.md section 'Task 9: Mock Layer + Project Settings UI'",
      "Create src/api-mock/plan-branch.ts with mock implementations for all planBranchApi methods returning snake_case data matching Zod schemas",
      "Register in src/api-mock/index.ts",
      "Add plan branch mock data to store.ts (sample PlanBranch with active status)",
      "Add feature branch toggle to project settings form (update_project_feature_branch_setting)",
      "Update apply_proposals mock to accept use_feature_branch param",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(mock): add plan branch mock data and project feature branch toggle"
    ],
    "passes": false
  },
  {
    "id": 10,
    "category": "backend",
    "description": "Add comprehensive backend tests for PlanBranch entity, repository, side effects helpers, and accept flow",
    "plan_section": "Task 10: Backend Tests",
    "blocking": [],
    "blockedBy": [4, 6],
    "atomic_commit": "test(plan_branch): add unit tests for entity, repository, side effects, and accept flow",
    "steps": [
      "Read specs/plans/feature_branch_for_plan_groups.md section 'Task 10: Backend Tests'",
      "PlanBranch entity tests: construction, status transitions, serialization",
      "Repository CRUD tests: create, get_by_plan_artifact_id, get_by_merge_task_id, get_by_project_id, update_status, set_merged",
      "resolve_task_base_branch tests: task with feature branch returns feature branch name, task without returns project.base_branch, merge task returns project.base_branch",
      "resolve_merge_branches tests: merge task returns (feature_branch, base_branch), regular plan task returns (task_branch, feature_branch), non-plan task returns (task_branch, base_branch)",
      "Migration tests: v13 creates table, adds column, idempotent re-run",
      "Accept plan integration test: apply_proposals with use_feature_branch=true creates branch + merge task + dependencies",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: test(plan_branch): add unit tests for entity, repository, side effects, and accept flow"
    ],
    "passes": false
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
| **Feature branch as git branch, not worktree** | Task worktrees are already created from the feature branch. Adding another worktree layer would overcomplicate the git topology without benefit. |
| **Merge task as regular task with `plan_merge` category** | Reuses existing task lifecycle (Ready→Executing→Approved→PendingMerge→Merged) with existing two-phase merge workflow. No new state machine states needed. |
| **Plan branch repo as Optional on TaskServices** | Follows existing DI pattern (same as task_repo, project_repo). Gracefully degrades when not available (falls back to project.base_branch). |
| **Feature branch setup between Phase 2 and Phase 3 of accept flow** | Tasks are still Backlog (not schedulable) until Phase 3 status upgrade. This is the safe window to set up the feature branch and merge task before the scheduler can pick anything up. |
| **`Option<bool>` for ApplyProposalsInput.use_feature_branch** | Additive serde change — existing frontends that don't send this field get `None` (use project default). No breaking change. |
| **Mid-plan conversion creates branch from current main HEAD** | Already-merged task work is included in main, so the feature branch starts from that point. Only future tasks redirect to the feature branch. |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] PlanBranch entity construction and serialization
- [ ] PlanBranchRepository CRUD operations
- [ ] Migration v13 creates plan_branches table and adds use_feature_branches column
- [ ] resolve_task_base_branch returns correct branch for plan tasks, merge tasks, and regular tasks
- [ ] resolve_merge_branches returns correct (source, target) pairs for all task types
- [ ] apply_proposals_to_kanban creates feature branch and merge task when enabled
- [ ] Merge task has correct blockedBy dependencies on all plan tasks

### Frontend - Run `npm run typecheck`
- [ ] PlanBranch Zod schema validates snake_case data correctly
- [ ] Transform function produces correct camelCase output
- [ ] planBranchApi methods invoke correct Tauri commands
- [ ] Mock API returns valid data matching schemas

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Create project with `use_feature_branches = true` → accept plan → feature branch created in git
- [ ] Execute a plan task → task branch created from feature branch (not main)
- [ ] Plan task merge → merges into feature branch (not main)
- [ ] All plan tasks complete → merge task unblocks → merge task merges feature branch into main
- [ ] Mid-plan conversion: enable feature branch after 1 task already merged to main → remaining tasks target feature branch
- [ ] Disable feature branch (before any merges) → branch removed, merge task removed

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] PlanGroupSettings gear icon renders and opens settings panel
- [ ] Feature branch toggle calls enable_feature_branch / disable_feature_branch commands
- [ ] Branch status badge updates reactively on plan-merge-complete event
- [ ] Merge task link opens task detail in graph split layout

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
