---
paths:
  - "src-tauri/src/application/git_service.rs"
  - "src-tauri/src/domain/entities/project.rs"
  - "src-tauri/src/domain/entities/plan_branch.rs"
  - "src-tauri/src/domain/repositories/plan_branch_repository.rs"
  - "src-tauri/src/domain/state_machine/transition_handler/side_effects.rs"
  - "src-tauri/src/http_server/handlers/git.rs"
  - "src-tauri/src/commands/plan_branch_commands.rs"
  - "src-tauri/src/commands/ideation_commands/**"
  - "src/api/plan-branch.ts"
  - "src/components/settings/GitSettingsSection.tsx"
  - "src/types/project.ts"
---

# Task Git Branching & Merge

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

**Required Context:** task-state-machine.md | agent-mcp-tools.md

---

## Two Git Modes

| | Local | Worktree |
|---|---|---|
| **Enum** | `GitMode::Local` | `GitMode::Worktree` |
| **Isolation** | Shared working directory | Separate directory per task |
| **Parallelism** | One task at a time (enforced) | Unlimited parallel tasks |
| **Branch switch** | `git checkout` on state entry | N/A (each worktree has own branch) |
| **Dirty tree guard** | Blocks Executing if uncommitted changes | N/A (isolated) |
| **Agent CWD** | `project.working_directory` | `task.worktree_path` |
| **Cleanup** | Delete branch on merge | Delete worktree + branch on merge |
| **Default** | YES (fallback) | — |
| **DB fields** | `task.task_branch` | `task.task_branch` + `task.worktree_path` |

**Config:** `project.git_mode` + `project.base_branch` (default: `"main"`) + `project.worktree_parent_directory` (default: `~/ralphx-worktrees`)

### Local Mode Single-Task Enforcement

**File:** `src-tauri/src/application/task_scheduler_service.rs`

Running states that block scheduling: `Executing`, `ReExecuting`, `Reviewing`, `Merging`

If any task in the same project is in a running state → no new task can enter `Executing`.

---

## Branch Hierarchy (Two Levels)

```
main (project.base_branch)
 ├─ ralphx/{slug}/plan-{artifact-id-8chars}     ← plan feature branch
 │   ├─ ralphx/{slug}/task-{task-id}            ← task branch (merges → plan branch)
 │   └─ [merge task] plan branch → main         ← final plan merge
 └─ ralphx/{slug}/task-{task-id}                ← standalone task (merges → main)
```

### Branch Naming

| Type | Pattern | Example |
|------|---------|---------|
| Task branch | `ralphx/{project-slug}/task-{task-id}` | `ralphx/my-app/task-abc123` |
| Plan branch | `ralphx/{project-slug}/plan-{short-artifact-id}` | `ralphx/my-app/plan-a1b2c3d4` |
| Worktree path | `{parent}/{project-slug}/task-{task-id}` | `~/ralphx-worktrees/my-app/task-abc123` |

`slugify()`: lowercase, non-alphanumeric → `-`, trim dashes

### Feature Branches (Plan-Level)

**Toggle:** `project.use_feature_branches` (default: `true`)

**Created at:** Plan apply (`apply_proposals_to_kanban`) or mid-plan (`enable_feature_branch`)

**On creation:**
1. Git branch `ralphx/{slug}/plan-{id}` from `project.base_branch`
2. DB record in `plan_branches` table (status: `Active`)
3. Auto-create merge task (status: `Blocked`, category: `plan_merge`)
4. Merge task `blockedBy` all plan tasks

**Entity:** `PlanBranch { id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status, merge_task_id }`

**Status:** `Active` → `Merged` | `Abandoned`

### Task / Session / PlanBranch Data Model

```
IdeationSession (has plan proposals)
  ├─ task.ideation_session_id → always set (canonical session link)
  ├─ task.plan_artifact_id    → set ONLY when real artifact exists (FK to artifacts table)
  └─ plan_branches.session_id → UNIQUE index, primary lookup key
```

| Field | Always Set? | FK Constraint? | Use For |
|-------|-------------|----------------|---------|
| `task.ideation_session_id` | YES (if from session) | None | Plan branch lookups, graph grouping |
| `task.plan_artifact_id` | Only if plan artifact exists | YES `REFERENCES artifacts(id)` | Artifact content retrieval |
| `plan_branches.session_id` | YES | None (UNIQUE index) | Primary plan branch lookup |
| `plan_branches.plan_artifact_id` | YES (may be session fallback) | None | Legacy compat |

**Rule:** Never put a session UUID into `task.plan_artifact_id` — FK violation. Use `ideation_session_id` instead.

### Base Branch Resolution

**File:** `side_effects.rs:resolve_task_base_branch()`

| Condition | Base Branch |
|-----------|-------------|
| Task has `ideation_session_id` AND plan has active feature branch | Plan feature branch |
| Otherwise | `project.base_branch` (default: `"main"`) |

### Merge Target Resolution

**File:** `side_effects.rs:resolve_merge_branches()`

| Condition | Source → Target |
|-----------|-----------------|
| Task IS the merge task (`plan_branches.merge_task_id`) | Plan feature branch → project base |
| Task belongs to plan with active feature branch (via `ideation_session_id`) | Task branch → plan feature branch |
| Standalone task (no plan) | Task branch → project base |

**Lookup path:** `task.ideation_session_id` → `plan_branch_repo.get_by_session_id()`.

---

## Merge Workflow (Two-Phase)

### Phase 1: Programmatic (Fast Path)

**Triggered on:** `pending_merge` entry via `attempt_programmatic_merge()`

| Step | Action |
|------|--------|
| 1 | Resolve source/target via `resolve_merge_branches()` |
| 2 | Worktree mode: delete worktree first (unlock branch) |
| 3 | Local: `GitService::try_rebase_and_merge()` / Worktree: `GitService::try_merge()` |
| 4a | **Success** → `complete_merge_internal()` → `Merged` |
| 4b | **Conflict** → transition to `Merging` → spawn merger agent |
| 4c | **Error** → transition to `MergeIncomplete` (human-waiting) |

**`try_rebase_and_merge()` (Local mode):**
1. Fetch origin (non-fatal)
2. If base has <=1 commit (empty repo): skip rebase, merge directly
3. Checkout task branch → `git rebase {base}`
4. Success: checkout base → `git merge {task_branch}` (fast-forward)
5. Conflict: `git rebase --abort`, checkout base → return `NeedsAgent`

**`try_merge()` (Worktree mode):**
1. Fetch origin (non-fatal)
2. Checkout base branch
3. `git merge {task_branch} --no-edit`
4. Success/FastForward: return `Success { commit_sha }`
5. Conflict: `git merge --abort` → return `NeedsAgent { conflict_files }`

**`complete_merge_internal()` cleanup:**
- Persist `task.merge_commit_sha`
- Delete worktree (if Worktree mode)
- Delete task branch
- For plan merge tasks: mark `plan_branch.status = Merged`, delete feature branch
- Emit `merge:completed` + `task:status_changed`

### Phase 2: Agent-Assisted (Conflict Resolution)

**Triggered on:** `merging` entry — spawns **merger agent** (opus model). See task-execution-agents.md.

**Merge outcome detection (auto, on agent exit):**

| Condition | Result |
|-----------|--------|
| No rebase in progress + no conflict markers | Auto → `Merged` |
| Rebase still in progress or conflict markers found | Auto → `MergeConflict` |

### Phase 3: Manual (Human Resolution)

| From | Event | → To |
|------|-------|------|
| `merge_conflict` | `ConflictResolved` | `merged` |
| `merge_incomplete` | `Retry` | `merging` (re-spawn agent) |
| `merge_incomplete` | `ConflictResolved` | `merged` |

---

## Git Operations (GitService)

**File:** `src-tauri/src/application/git_service.rs` — stateless, all methods static.

### Branch Ops

| Method | Git Command |
|--------|-------------|
| `create_branch(repo, branch, base)` | `git branch {branch} {base}` |
| `checkout_branch(repo, branch)` | `git checkout {branch}` |
| `delete_branch(repo, branch, force)` | `git branch -d/-D {branch}` |
| `create_feature_branch(repo, branch, source)` | `git branch {branch} {source}` (no checkout) |
| `delete_feature_branch(repo, branch)` | `git branch -d {branch}` |
| `get_current_branch(repo)` | `git rev-parse --abbrev-ref HEAD` |

### Worktree Ops

| Method | Git Command |
|--------|-------------|
| `create_worktree(repo, path, branch, base)` | `git worktree add -b {branch} {path} {base}` |
| `delete_worktree(repo, path)` | `git worktree remove --force {path}` |

### Commit Ops

| Method | Git Command |
|--------|-------------|
| `commit_all(path, msg)` | `git add -A && git commit -m {msg}` → returns SHA |
| `has_uncommitted_changes(path)` | `git status --porcelain` |
| `get_head_sha(path)` | `git rev-parse HEAD` |

### Merge/Rebase Ops

| Method | Git Command | Returns |
|--------|-------------|---------|
| `merge_branch(repo, source, _target)` | `git merge {source} --no-edit` | `Success` / `FastForward` / `Conflict` |
| `rebase_onto(path, base)` | `git rebase {base}` | `Success` / `Conflict` |
| `abort_merge(repo)` | `git merge --abort` | — |
| `abort_rebase(path)` | `git rebase --abort` | — |
| `get_conflict_files(repo)` | `git diff --name-only --diff-filter=U` | File list |

### Merge State Detection

| Method | Checks |
|--------|--------|
| `is_rebase_in_progress(worktree)` | `.git/rebase-merge` or `.git/rebase-apply` dirs |
| `has_conflict_markers(worktree)` | Scans tracked files for `<<<<<<<` |
| `is_commit_on_branch(repo, sha, branch)` | `git merge-base --is-ancestor` |

---

## Conflict Resolution Patterns

### Duplicate Migrations

**Pattern:** Task branch and plan branch both add migration version N (same table name, same structure).

**Root cause:** Task created off main before plan branch integrated earlier migration work. On rebase, both try to add v33.

**Resolution:**
1. Do not hand-pick the next integer migration version
2. Regenerate the task-branch migration with `python3 scripts/new_sqlite_migration.py <description>` after rebasing on latest `main`
3. Keep the already-shipped migration ids on the target branch untouched
4. Run `python3 scripts/validate_sqlite_migrations.py` before continuing the rebase or merge
5. Adapt task-branch-specific entity/repo methods to the plan branch's type definitions (don't change types mid-rebase)

**File:** `src-tauri/src/domain/repositories/migrations_impl.rs`

### Type Definition Conflicts (IDs, Entities)

**Pattern:** Task branch uses String-based ID (e.g., `ChatAttachmentId(String)` in types.rs) but plan branch uses Uuid-based newtype in entities/.

**Root cause:** Competing approaches to type safety. Plan branch integrates domain types first, task branch adds surface-layer types.

**Resolution:**
1. Keep plan branch's type definition (it's already deployed)
2. Adapt task branch's new methods to use the plan branch's type
3. Never change types during rebase — preserve both approaches:
   - Domain layer: Uuid newtypes in `entities/`
   - User-facing: String newtypes in `types.rs`
4. Conversion happens only at API boundaries (HTTP handlers)

**Files:** `src-tauri/src/domain/entities/`, `src-tauri/src/domain/types.rs`, `src-tauri/src/domain/repositories/`

### Multi-Commit Rebase Strategy

**Pattern:** Task branch has 2+ commits. First commit creates conflicts (e.g., migrations), second commit has entity/repo conflicts.

**Strategy:**
1. Resolve first commit's conflicts in isolation (read all conflicted files for that commit)
2. `git add <file> && git rebase --continue` → rebase moves to next commit
3. Repeat for each commit until completion
4. Later commits may rebase cleanly if they don't conflict

**Commands:**
```bash
git rebase <target-branch>
# Conflict 1
git add <resolved-files>
git rebase --continue
# Conflict 2 (if any)
git add <resolved-files>
git rebase --continue
```
