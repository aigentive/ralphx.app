# Task Execution & Git Workflows

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

**Required Context:** @.claude/rules/agent-mcp-tools.md | @.claude/rules/api-layer.md

---

## Two Git Modes

| | Local | Worktree |
|---|---|---|
| **Enum** | `GitMode::Local` | `GitMode::Worktree` |
| **Isolation** | Shared working directory | Separate directory per task |
| **Parallelism** | ❌ One task at a time (enforced) | ✅ Unlimited parallel tasks |
| **Branch switch** | `git checkout` on state entry | N/A (each worktree has own branch) |
| **Dirty tree guard** | ❌ Blocks Executing if uncommitted changes | N/A (isolated) |
| **Agent CWD** | `project.working_directory` | `task.worktree_path` |
| **Cleanup** | Delete branch on merge | Delete worktree + branch on merge |
| **Default** | ✅ (fallback) | — |
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

### Task ↔ Session ↔ PlanBranch Data Model

```
IdeationSession (has plan proposals)
  ├─ task.ideation_session_id → always set (canonical session link)
  ├─ task.plan_artifact_id    → set ONLY when real artifact exists (FK to artifacts table)
  └─ plan_branches.session_id → UNIQUE index, primary lookup key
```

| Field | Always Set? | FK Constraint? | Use For |
|-------|-------------|----------------|---------|
| `task.ideation_session_id` | ✅ (if from session) | None | Plan branch lookups, graph grouping |
| `task.plan_artifact_id` | Only if plan artifact exists | ✅ `REFERENCES artifacts(id)` | Artifact content retrieval |
| `plan_branches.session_id` | ✅ | None (UNIQUE index) | Primary plan branch lookup |
| `plan_branches.plan_artifact_id` | ✅ (may be session fallback) | None | Legacy compat |

**Rule:** Never put a session UUID into `task.plan_artifact_id` — FK violation. Use `ideation_session_id` instead.

### Base Branch Resolution (Task Execution)

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

**Lookup path:** `task.ideation_session_id` → `plan_branch_repo.get_by_session_id()`. The `ideation_session_id` is the canonical link between tasks and their originating session (always valid, unlike `plan_artifact_id` which may be NULL for sessions without a plan artifact).

---

## 24 Internal Statuses

| # | Status | Category | Terminal? |
|---|--------|----------|-----------|
| 1 | `backlog` | Idle | — |
| 2 | `ready` | Idle | — |
| 3 | `blocked` | Idle | — |
| 4 | `executing` | Active (agent) | — |
| 5 | `qa_refining` | Active (agent) | — |
| 6 | `qa_testing` | Active (agent) | — |
| 7 | `qa_passed` | Transient | — |
| 8 | `qa_failed` | Waiting (human) | — |
| 9 | `pending_review` | Transient | — |
| 10 | `reviewing` | Active (agent) | — |
| 11 | `review_passed` | Waiting (human) | — |
| 12 | `escalated` | Waiting (human) | — |
| 13 | `revision_needed` | Transient | — |
| 14 | `re_executing` | Active (agent) | — |
| 15 | `approved` | Transient | — |
| 16 | `pending_merge` | Transient | — |
| 17 | `merging` | Active (agent) | — |
| 18 | `merge_incomplete` | Waiting (human) | — |
| 19 | `merge_conflict` | Waiting (human) | — |
| 20 | `merged` | Done | ✅ |
| 21 | `failed` | Done | ✅ |
| 22 | `cancelled` | Done | ✅ |
| 23 | `paused` | Suspended | — |
| 24 | `stopped` | Done | ✅ |

**Transient:** Auto-transitions immediately to next state (no UI dwell time).

---

## State Transition Table

| From | → Valid Targets |
|------|----------------|
| `backlog` | `ready`, `cancelled` |
| `ready` | `executing`, `blocked`, `cancelled` |
| `blocked` | `ready`, `cancelled` |
| `executing` | `qa_refining`, `pending_review`, `failed`, `blocked`, `stopped`, `paused` |
| `qa_refining` | `qa_testing`, `stopped`, `paused` |
| `qa_testing` | `qa_passed`, `qa_failed`, `stopped`, `paused` |
| `qa_passed` | `pending_review` |
| `qa_failed` | `revision_needed` |
| `pending_review` | `reviewing` |
| `reviewing` | `review_passed`, `revision_needed`, `escalated`, `stopped`, `paused` |
| `review_passed` | `approved`, `revision_needed` |
| `escalated` | `approved`, `revision_needed` |
| `revision_needed` | `re_executing`, `cancelled` |
| `re_executing` | `pending_review`, `failed`, `blocked`, `stopped`, `paused` |
| `approved` | `pending_merge`, `ready` |
| `pending_merge` | `merged`, `merging` |
| `merging` | `merged`, `merge_conflict`, `merge_incomplete`, `stopped`, `paused` |
| `merge_incomplete` | `merging`, `merged` |
| `merge_conflict` | `merged` |
| `merged` | `ready` |
| `failed` | `ready` |
| `cancelled` | `ready` |
| `stopped` | `ready` |
| `paused` | `executing`, `re_executing`, `qa_refining`, `qa_testing`, `reviewing`, `merging` |

---

## Auto-Transitions (Immediate, Chained)

| Reached State | Auto-Transitions To | Why |
|---------------|---------------------|-----|
| `qa_passed` | `pending_review` | QA done → start review |
| `pending_review` | `reviewing` | Spawn reviewer agent immediately |
| `revision_needed` | `re_executing` | Spawn worker agent for revision immediately |
| `approved` | `pending_merge` | Start merge workflow immediately |

These are persisted as intermediate states but the system chains through them in one operation.

---

## Side Effects on State Entry

| State | Side Effects |
|-------|-------------|
| **ready** | Spawn `qa-prep` agent (background, if QA enabled). After 600ms delay → `try_schedule_ready_tasks()` |
| **executing** | Create task branch (Local: check dirty tree → create+checkout | Worktree: create worktree dir). Persist `task_branch`/`worktree_path`. Spawn **worker** agent |
| **qa_refining** | Wait for qa-prep if needed. Spawn `qa-refiner` agent |
| **qa_testing** | Spawn `qa-tester` agent |
| **qa_passed** | Emit `qa_passed` event |
| **qa_failed** | Emit `qa_failed` event. Notify user |
| **pending_review** | Start AI review via `ReviewStarter`. Emit `review:update` event |
| **reviewing** | Checkout task branch (Local mode). Spawn **reviewer** agent |
| **review_passed** | Emit `review:ai_approved`. Notify user "Please review and approve" |
| **escalated** | Emit `review:escalated`. Notify user "Please review and decide" |
| **re_executing** | Checkout task branch (Local mode). Spawn **worker** agent with revision context |
| **approved** | Emit `task_completed` event |
| **pending_merge** | Run `attempt_programmatic_merge()` (see Merge Workflow below) |
| **merging** | Spawn **merger** agent |
| **merged** | `dependency_manager.unblock_dependents()` → auto-unblock waiting tasks |

## Side Effects on State Exit

| From State | Exit Effects |
|------------|-------------|
| Agent-active states (`executing`, `re_executing`, `qa_refining`, `qa_testing`, `reviewing`, `merging`) | Decrement `running_count`. Emit `execution:status_changed`. `try_schedule_ready_tasks()` (slot freed) |
| `executing`, `re_executing` | `auto_commit_on_execution_done()` — commits any uncommitted changes as `feat: {task_title}` |
| `reviewing` | Emit `review:state_exited` |

---

## Happy Path Flows

### Without QA
```
Backlog → Ready → Executing → PendingReview → (auto) Reviewing
→ ReviewPassed → (HumanApprove) Approved → (auto) PendingMerge
→ Merged (fast) or Merging → Merged
```

### With QA
```
Backlog → Ready → Executing → QaRefining → QaTesting
→ QaPassed → (auto) PendingReview → (auto) Reviewing
→ ReviewPassed → (HumanApprove) Approved → (auto) PendingMerge → Merged
```

### Revision Loop
```
Reviewing → RevisionNeeded → (auto) ReExecuting → PendingReview
→ (auto) Reviewing → ... (repeat until pass or fail/cancel)
```

---

## Merge Workflow (Two-Phase)

### Phase 1: Programmatic (Fast Path)

**Triggered on:** `pending_merge` entry via `attempt_programmatic_merge()`

| Step | Action |
|------|--------|
| 1 | Resolve source/target via `resolve_merge_branches()` |
| 2 | Worktree mode: delete worktree first (unlock branch) |
| 3 | Local mode: `GitService::try_rebase_and_merge(repo, task_branch, target)` / Worktree mode: `GitService::try_merge(repo, task_branch, target)` |
| 4a | **Success** → `complete_merge_internal()` → `Merged` |
| 4b | **Conflict** → transition to `Merging` → spawn merger agent |
| 4c | **Error** → transition to `MergeIncomplete` (human-waiting, no agent spawn) |

**`try_rebase_and_merge()` internals (Local mode):**
1. Fetch origin (non-fatal)
2. If base has ≤1 commit (empty repo): skip rebase, merge directly
3. Checkout task branch → `git rebase {base}`
4. Success: checkout base → `git merge {task_branch}` (fast-forward)
5. Conflict: `git rebase --abort`, checkout base → return `NeedsAgent`

**`try_merge()` internals (Worktree mode):**
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

**Triggered on:** `merging` entry — spawns **merger agent** (opus model)

**Merger workflow:**
1. `get_merge_target(task_id)` → returns `{ source_branch, target_branch }`
2. `get_task_context(task_id)` → conflict files, task details
3. Read each conflicted file, resolve markers, edit files
4. Verify: grep for remaining `<<<<<<< HEAD`, run `cargo check`/`npm run typecheck`
5. `git add .` + `git rebase --continue` (or fresh commit)

**Merge outcome detection (auto, on agent exit):**

| Condition | Result |
|-----------|--------|
| No rebase in progress + no conflict markers | Auto → `Merged` |
| Rebase still in progress or conflict markers found | Auto → `MergeConflict` |

**Merger MCP tools:**

| Tool | Purpose | Required? |
|------|---------|-----------|
| `get_merge_target` | Get source/target branches | ✅ Always call first |
| `complete_merge` | Explicit success signal (with commit SHA) | Optional (auto-detected) |
| `report_conflict` | Cannot resolve → `MergeConflict` | ✅ If stuck |
| `report_incomplete` | Non-conflict failure → `MergeIncomplete` | ✅ If git error |
| `get_task_context` | Task details + conflict files | As needed |

### Phase 3: Manual (Human Resolution)

**States:** `merge_conflict` or `merge_incomplete`

| From | Event | → To |
|------|-------|------|
| `merge_conflict` | `ConflictResolved` | `merged` |
| `merge_incomplete` | `Retry` | `merging` (re-spawn agent) |
| `merge_incomplete` | `ConflictResolved` | `merged` |

---

## Execution Agents

### Worker (`ralphx-worker`)

| Aspect | Detail |
|--------|--------|
| **Model** | sonnet |
| **Trigger** | `executing` or `re_executing` entry |
| **CWD** | Worktree path or project dir |
| **Permission** | `acceptEdits` (Write/Edit/Bash pre-approved) |
| **Env var** | `RALPHX_TASK_STATE` = `executing` or `re_executing` |

**Execution flow:**
1. If `re_executing` → fetch `get_review_notes()` + `get_task_issues(status: "open")` first
2. `get_task_context(task_id)` → task details, proposal, plan, dependencies
3. If blocked → STOP
4. Read plan artifact if exists
5. `start_step()` → work → `complete_step()` (per step)
6. For re-execution: `mark_issue_in_progress()` / `mark_issue_addressed()` per issue

**Key MCP tools:** `start_step`, `complete_step`, `skip_step`, `fail_step`, `add_step`, `get_task_context`, `get_review_notes`, `get_task_issues`, `mark_issue_in_progress`, `mark_issue_addressed`

### Reviewer (`ralphx-reviewer`)

| Aspect | Detail |
|--------|--------|
| **Model** | sonnet |
| **Trigger** | `reviewing` entry |
| **Session** | Always fresh (never resumed) |

**MUST call `complete_review` before exiting.** Task stuck in `reviewing` otherwise.

**`complete_review` params:**
- `outcome`: `"approved"` | `"needs_changes"` | `"escalate"`
- `notes`, `fix_description` (if needs_changes)
- `issues[]` (REQUIRED for needs_changes): `{ title, severity, step_id, description, file_path, line_number }`
- `escalation_reason` (if escalate)

**Review outcomes → transitions:**

| Outcome | Transition |
|---------|------------|
| `approved` | `reviewing` → `review_passed` |
| `needs_changes` | `reviewing` → `revision_needed` → (auto) `re_executing` |
| `escalate` | `reviewing` → `escalated` |

### Merger (`ralphx-merger`)

| Aspect | Detail |
|--------|--------|
| **Model** | opus (most capable, for complex conflicts) |
| **Trigger** | `merging` entry (after programmatic merge fails) |
| **Pre-approved** | Read, Edit, Bash |

See Phase 2 in Merge Workflow above.

### ChatService Context → Agent Resolution

| Context Type | Default Agent | Status Override | Session |
|-------------|---------------|-----------------|---------|
| `TaskExecution` | `ralphx-worker` | — | Never resumed (fresh spawn) |
| `Review` | `ralphx-reviewer` | `review_passed` → `ralphx-review-chat` | Never resumed (fresh) |
| `Merge` | `ralphx-merger` | — | May resume |
| `Ideation` | `orchestrator-ideation` | `accepted` → `orchestrator-ideation-readonly` | Resumes |
| `Task` | `chat-task` | — | Resumes |
| `Project` | `chat-project` | — | Resumes |

### Support Agents

| Agent | Model | Role |
|-------|-------|------|
| `orchestrator-ideation` | sonnet | Facilitates ideation sessions, creates task proposals + plans |
| `session-namer` | haiku | Generates 2-word session titles |
| `dependency-suggester` | haiku | Auto-applies dependency suggestions between proposals |
| `chat-task` | sonnet | Task-specific Q&A |
| `chat-project` | sonnet | Project-level questions |
| `ralphx-review-chat` | sonnet | Discuss review findings (when status = `review_passed`) |
| `ralphx-qa-prep` | sonnet | Generate acceptance criteria + test steps (background, on `ready`) |
| `ralphx-qa-executor` | sonnet | Browser-based QA via agent-browser |
| `ralphx-orchestrator` | opus | Complex multi-step coordination |
| `ralphx-supervisor` | haiku | Monitor agents for loops/stalls |
| `ralphx-deep-researcher` | opus | Thorough research |

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

## TaskEvent → Transition Dispatch

### User Events

| Event | From → To |
|-------|-----------|
| `Schedule` | `backlog` → `ready` |
| `StartExecution` | `ready` → `executing` |
| `Cancel` | (most states) → `cancelled` |
| `HumanApprove` | `review_passed`/`escalated` → `approved` |
| `HumanRequestChanges` | `review_passed`/`escalated` → `revision_needed` |
| `ForceApprove` | (override) → `approved` |
| `Retry` | `failed`/`cancelled`/`stopped`/`merged` → `ready` |
| `SkipQa` | `qa_failed` → `pending_review` |

### Agent Events

| Event | From → To |
|-------|-----------|
| `ExecutionComplete` | `executing`/`re_executing` → `qa_refining` (QA on) or `pending_review` (QA off) |
| `ExecutionFailed` | `executing`/`re_executing` → `failed` |
| `NeedsHumanInput` | `executing` → `blocked` |
| `QaRefinementComplete` | `qa_refining` → `qa_testing` |
| `QaTestsComplete(true)` | `qa_testing` → `qa_passed` |
| `QaTestsComplete(false)` | `qa_testing` → `qa_failed` |
| `ReviewComplete(approved)` | `reviewing` → `review_passed` |
| `ReviewComplete(!approved)` | `reviewing` → `revision_needed` |
| `MergeAgentFailed` | `merging` → `merge_conflict` |
| `MergeAgentError` | `merging` → `merge_incomplete` |

### System Events

| Event | From → To |
|-------|-----------|
| `StartReview` | `pending_review` → `reviewing` |
| `StartRevision` | `revision_needed` → `re_executing` |
| `StartMerge` | `approved` → `pending_merge` |
| `MergeComplete` | `pending_merge`/`merging` → `merged` |
| `MergeConflict` | `pending_merge` → `merging` |
| `ConflictResolved` | `merge_conflict`/`merge_incomplete` → `merged` |
| `BlockersResolved` | `blocked` → `ready` |
| `BlockerDetected` | `ready`/`re_executing` → `blocked` |

---

## Guard Conditions

| Guard | Location | Effect |
|-------|----------|--------|
| Dirty working tree (Local mode) | `on_enter(Executing)` | `Err(ExecutionBlocked)` — task cannot execute |
| `context.qa_enabled` | `ExecutionComplete` dispatch | Routes to `qa_refining` vs `pending_review` |
| `task.task_branch.is_none()` | `on_enter(Executing)` | Only creates branch on first execution (skip on re-entry) |
| Running task (Local mode) | `task_scheduler_service` | Blocks scheduling if another task is in a running state |
| Self-transition | `can_transition_to()` | No state can transition to itself |

---

## Key Files Index

| Component | Path |
|-----------|------|
| GitMode enum | `src-tauri/src/domain/entities/project.rs` |
| InternalStatus (24 variants) | `src-tauri/src/domain/entities/status.rs` |
| Valid transitions table | `src-tauri/src/domain/entities/status.rs:valid_transitions()` |
| TaskEvent enum | `src-tauri/src/domain/state_machine/events.rs` |
| State machine dispatcher | `src-tauri/src/domain/state_machine/machine/transitions.rs` |
| TransitionHandler + auto-transitions | `src-tauri/src/domain/state_machine/transition_handler/mod.rs` |
| on_enter side effects (branches, agents, merge) | `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs` |
| resolve_task_base_branch / resolve_merge_branches | `side_effects.rs:211-285` |
| attempt_programmatic_merge | `side_effects.rs:857-1148` |
| complete_merge_internal | `side_effects.rs:36-111` |
| GitService (all git ops) | `src-tauri/src/application/git_service.rs` |
| TaskTransitionService (app bridge) | `src-tauri/src/application/task_transition_service.rs` |
| Task scheduler (Local mode enforcement) | `src-tauri/src/application/task_scheduler_service.rs` |
| PlanBranch entity | `src-tauri/src/domain/entities/plan_branch.rs` |
| PlanBranch repo trait | `src-tauri/src/domain/repositories/plan_branch_repository.rs` |
| Agent configs (three-layer allowlist) | `src-tauri/src/infrastructure/agents/claude/agent_config.rs` |
| Agent spawner (CWD resolution) | `src-tauri/src/infrastructure/agents/spawner.rs` |
| ChatService contexts | `src-tauri/src/application/chat_service/chat_service_context.rs` |
| HTTP merge handlers | `src-tauri/src/http_server/handlers/git.rs` |
| Agent definitions | `ralphx-plugin/agents/*.md` |
| Plan branch commands | `src-tauri/src/commands/plan_branch_commands.rs` |
| Ideation apply (feature branch creation) | `src-tauri/src/commands/ideation_commands/ideation_commands_apply.rs` |
| Git settings UI | `src/components/settings/GitSettingsSection.tsx` |
| Frontend plan-branch API | `src/api/plan-branch.ts` |
| Frontend GitMode type | `src/types/project.ts` |
