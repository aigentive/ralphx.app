# Per-Task Git Branch Isolation

## Overview

Transform RalphX to use **per-task branch isolation** for all projects. Every executing task gets its own git branch (and optionally a separate worktree), enabling parallel execution and clean merge workflows.

## User Requirements

- **Both modes use task branches** - Local mode creates branches in main repo, Worktree mode creates separate worktrees
- **Worktree mode as DEFAULT** for new projects (recommended)
- Per-TASK branches/worktrees (not per-project)
- Automated branch/worktree management (user doesn't manage paths)
- **Auto-commit** with programmatic message: `{commit_message_prefix}{task_title}` (prefix default: "feat: ")
- **Merge workflow** after approval (both modes):
  1. Programmatic rebase + merge (fast path, no agent)
  2. If conflicts → merger agent attempts resolution
  3. If agent fails → manual resolution by user
- Allow switching git mode after project creation

---

## Architecture

### Git Mode Comparison

| Aspect | Local Mode | Worktree Mode (Default) |
|--------|------------|-------------------------|
| Branch creation | `git checkout -b {branch}` | `git worktree add` |
| Working directory | Main repo (switches branches) | Separate worktree per task |
| Parallel execution | ❌ One task at a time | ✅ Multiple concurrent tasks |
| Merge workflow | ✅ Same as Worktree | ✅ Same as Local |
| Recommended | No | **Yes** |

### Branch/Worktree Naming

```
Branch: ralphx/{project-slug}/task-{task-id}

Worktree path (Worktree mode only):
{worktree_parent}/
└── {project-slug}/
    ├── task-{task-id-1}/
    ├── task-{task-id-2}/
    └── task-{task-id-3}/
```

### New Internal States

Add to `InternalStatus` enum:

| State | Purpose |
|-------|---------|
| `PendingMerge` | Approved, waiting for merge (auto-transition) |
| `Merging` | Merge agent attempting auto-merge/conflict resolution |
| `MergeConflict` | Merge failed, needs manual resolution |
| `Merged` | Successfully merged to base branch |

### State Flow (Both Modes)

```
Ready → Executing [create branch/worktree]
    ↓
ExecutionDone [auto-commit] → QA → PendingReview → Reviewing
    ↓
ReviewPassed → (human approve) → Approved → PendingMerge (auto)
    ↓
PendingMerge [programmatic rebase + merge attempt]
    ↓
[success] → Merged → [cleanup branch/worktree]
    ↓
[conflict] → Merging (agent) → [success] → Merged
                            → [conflict] → MergeConflict → (manual) → Merged
```

**Key insight:** Most merges succeed programmatically (fast-forward after rebase). Agent only invoked for actual conflicts.

### Two-Phase Merge Strategy

**Phase 1: Programmatic Merge (Fast Path)**
- Triggered on `on_enter(PendingMerge)`
- Steps:
  1. Fetch latest from origin (if remote configured)
  2. Attempt `git rebase {base_branch}` on task branch
  3. If rebase succeeds: `git checkout {base} && git merge {task_branch}` (fast-forward)
  4. If merge succeeds → Transition directly to `Merged` (skip agent)
  5. If fails at any step → Transition to `Merging` (agent phase)

**Phase 2: Agent-Assisted Resolution**
- Triggered on `on_enter(Merging)` (only if Phase 1 failed)
- Agent analyzes conflicts and attempts resolution
- If resolved → `Merged`
- If cannot resolve → `MergeConflict` (manual)

### New Agent: `ralphx-merger`

| Property | Value |
|----------|-------|
| Purpose | Resolve merge conflicts when programmatic merge fails |
| CLI Tools | `Bash`, `Read`, `Edit` (for conflict resolution) |
| MCP Tools | `complete_merge`, `report_conflict` |
| Triggers | `on_enter(Merging)` (Phase 2 only) |

### Working Directory Resolution

```rust
fn resolve_working_directory(task: &Task, project: &Project) -> PathBuf {
    match project.git_mode {
        GitMode::Local => {
            // Local mode: always main repo (branch switches handle isolation)
            PathBuf::from(&project.working_directory)
        }
        GitMode::Worktree => {
            // Worktree mode: use task's worktree if exists
            task.worktree_path
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(&project.working_directory))
        }
    }
}
```

### Auto-Commit Message Format (Programmatic)

```
Format: {commit_message_prefix}{task_title}

Examples (with default prefix "feat: "):
- Task title: "Add user authentication flow" → "feat: Add user authentication flow"
- Task title: "Resolve login timeout issue" → "feat: Resolve login timeout issue"

With custom prefix "fix: ":
- Task title: "Resolve login timeout issue" → "fix: Resolve login timeout issue"
```

Uses existing `ExecutionSettings.commit_message_prefix` setting.

### State-Aware Operations

| Transition | Local Mode | Worktree Mode |
|------------|------------|---------------|
| Ready → Executing | `git checkout -b {branch}` | `git worktree add` |
| Executing → ExecutionDone | Auto-commit if enabled | Auto-commit if enabled |
| → ReExecuting | Already on branch | Already in worktree |
| → Reviewing | Already on branch | Already in worktree |
| Approved → PendingMerge | Auto-transition | Auto-transition |
| PendingMerge | **Phase 1:** Try rebase + merge programmatically | Same |
| → Merged (success) | Delete branch, checkout base | Delete worktree + branch |
| → Merging (conflict) | **Phase 2:** Spawn merger agent | Same |
| Merging → Merged | Agent resolved, cleanup | Agent resolved, cleanup |
| Merging → MergeConflict | Agent failed, keep branch | Agent failed, keep worktree |
| MergeConflict → Merged | Manual resolve, cleanup | Manual resolve, cleanup |

---

## Clarifications & Design Decisions

### 1. Local Mode: Uncommitted Changes Handling

**Decision:** Block execution if uncommitted changes exist in the working directory.

When attempting to checkout a task branch in Local mode:
1. Check `git status --porcelain` for uncommitted changes
2. If changes exist → Return error: "Cannot execute task: uncommitted changes in working directory. Please commit or stash your changes first."
3. Do NOT auto-stash (risky if task never completes)

This ensures clean branch isolation and makes the user aware of their repository state.

### 2. Worktree Parent Directory Creation

**Decision:** Auto-create worktree parent directory if it doesn't exist.

In `GitService.create_worktree()`:
```rust
// Ensure parent directory exists
std::fs::create_dir_all(&worktree_parent)?;
```

### 3. HTTP Layer for MCP Tools

**Decision:** Create new `git.rs` handler file for git-related endpoints.

**File:** `src-tauri/src/http_server/handlers/git.rs` (NEW)

| Endpoint | Purpose |
|----------|---------|
| `POST /git/tasks/{id}/complete-merge` | Merger agent signals successful resolution |
| `POST /git/tasks/{id}/report-conflict` | Merger agent signals unresolvable conflict |
| `GET /git/tasks/{id}/commits` | Get commits on task branch |
| `GET /git/tasks/{id}/diff-stats` | Get diff statistics for task branch |

This keeps git operations separate from task CRUD (tasks.rs) and aligns with existing domain-based handler separation.

### 4. Local Mode: Queue-Based Execution Enforcement

**Decision:** Modify scheduler to skip Ready tasks from Local-mode projects that already have a running task.

Instead of blocking the transition to Ready, we enforce at the scheduler level:

```rust
// In TaskSchedulerService.find_oldest_schedulable_task()
for task in oldest_ready_tasks {
    let project = get_project(task.project_id);
    if project.git_mode == GitMode::Local {
        // Check if this project already has an executing task
        let running_states = [Executing, ReExecuting, Reviewing, Merging];
        let has_running = task_repo.has_task_in_states(&project.id, &running_states).await?;
        if has_running {
            continue;  // Skip, try next task in queue
        }
    }
    return Some(task);  // This one is schedulable
}
```

**Benefits:**
- Tasks stay in Ready queue (visible in UI as "Queued")
- No manual user intervention needed
- Integrates with existing `max_concurrent` setting
- Worktree mode unaffected (parallel execution allowed)

### 5. Kanban Done Column: Subgroup Support

**Decision:** Add subgroups to Done column similar to InReview column grouping.

| Group | States | Visual Style |
|-------|--------|--------------|
| Merging | `pending_merge`, `merging` | Default |
| Needs Attention | `merge_conflict` | Warning (amber/orange) |
| Completed | `merged`, `approved` | Success (green) |
| Terminal | `failed`, `cancelled` | Muted |

`MergeConflict` should be visually distinct with warning styling to draw user attention.

### 6. Chat Context for Merge States

**Decision:** Add new `merge` context type. MergeConflict shows read-only view (no interactive helper agent for now).

| State | Context Type | Chat Behavior |
|-------|--------------|---------------|
| `pending_merge` | N/A | No chat (programmatic, no agent) |
| `merging` | `merge` | Live merger agent conversation |
| `merge_conflict` | `merge` (read-only) | Historical view of merger agent conversation |
| `merged` | `merge` (read-only) | Historical view of merger agent conversation |

**Implementation:**
1. Add `merge` to `TaskContextType` union: `"task" | "task_execution" | "review" | "merge"`
2. Wire `MergingTaskDetail` to show chat panel with `useTaskChat(taskId, "merge")`
3. `MergeConflictTaskDetail` and `MergedTaskDetail` show read-only chat (no send capability)

### 7. Task Detail Views: Registry Integration

**Decision:** Combine `pending_merge` and `merging` into single `MergingTaskDetail` component.

| State | View Component | Rationale |
|-------|----------------|-----------|
| `pending_merge` | `MergingTaskDetail` | Same component, shows "Attempting merge..." |
| `merging` | `MergingTaskDetail` | Same component, shows agent + conflict files |
| `merge_conflict` | `MergeConflictTaskDetail` | Distinct actions needed |
| `merged` | `MergedTaskDetail` | Completion view |

**Why combine pending_merge and merging:**
1. Seamless user experience during merge process
2. PendingMerge is typically very brief (1-3 seconds)
3. Single component with conditional content based on `task.internal_status`
4. Smooth visual transition if conflicts occur

**Registry additions in `TaskDetailPanel.tsx`:**
```typescript
const TASK_DETAIL_VIEWS: Record<InternalStatus, ComponentType<TaskDetailProps>> = {
  // ... existing mappings
  pending_merge: MergingTaskDetail,
  merging: MergingTaskDetail,
  merge_conflict: MergeConflictTaskDetail,
  merged: MergedTaskDetail,
};
```

**History navigation integration:**
- All new views accept `isHistorical?: boolean` prop
- When `isHistorical=true`, action buttons are hidden
- Views integrate with Phase 64's `conversationId`/`agentRunId` metadata for correct chat scroll position

---

## Implementation Tasks

### Phase 1: Core Infrastructure

#### 1.1 GitService Creation (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(git): create GitService with branch, worktree, and merge operations`

**File:** `src-tauri/src/application/git_service.rs` (NEW)

```rust
pub struct GitService;

impl GitService {
    // Branch operations (both modes)
    pub fn create_branch(repo: &Path, branch: &str, base: &str) -> Result<()>
    pub fn checkout_branch(repo: &Path, branch: &str) -> Result<()>
    pub fn delete_branch(repo: &Path, branch: &str, force: bool) -> Result<()>
    pub fn get_current_branch(repo: &Path) -> Result<String>

    // Worktree operations (Worktree mode only)
    // NOTE: create_worktree must call create_dir_all on parent directory
    pub fn create_worktree(repo: &Path, worktree: &Path, branch: &str, base: &str) -> Result<()>
    pub fn delete_worktree(repo: &Path, worktree: &Path) -> Result<()>

    // Commit operations
    pub fn commit_all(path: &Path, message: &str) -> Result<Option<String>>
    pub fn has_uncommitted_changes(path: &Path) -> Result<bool>

    // Rebase operations (Phase 1 - fast path)
    pub fn fetch_origin(repo: &Path) -> Result<()>
    pub fn rebase_onto(path: &Path, base: &str) -> Result<RebaseResult>
    pub fn abort_rebase(path: &Path) -> Result<()>

    // Merge operations
    pub fn merge_branch(repo: &Path, source: &str, target: &str) -> Result<MergeResult>
    pub fn abort_merge(repo: &Path) -> Result<()>
    pub fn get_conflict_files(repo: &Path) -> Result<Vec<PathBuf>>

    // Combined operation for Phase 1
    pub fn try_rebase_and_merge(repo: &Path, task_branch: &str, base: &str) -> Result<MergeAttemptResult>

    // Query operations
    pub fn get_commits_since(path: &Path, base: &str) -> Result<Vec<CommitInfo>>
    pub fn get_diff_stats(path: &Path, base: &str) -> Result<DiffStats>
}

// IMPORTANT: has_uncommitted_changes() is used to block execution in Local mode
// if there are uncommitted changes in the working directory (see Clarification #1)

pub enum MergeResult {
    Success { commit_sha: String },
    Conflict { files: Vec<PathBuf> },
    FastForward { commit_sha: String },
}

pub enum RebaseResult {
    Success,
    Conflict { files: Vec<PathBuf> },
}

pub enum MergeAttemptResult {
    Success { commit_sha: String },       // Rebase + merge worked
    NeedsAgent { conflict_files: Vec<PathBuf> },  // Conflict, needs agent
}
```

#### 1.2 Task Entity Extension (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(task): add task_branch, worktree_path, merge_commit_sha fields with migration`

**Files:** `src-tauri/src/domain/entities/task.rs`, migration

```rust
// Add fields (both modes use branch, only Worktree mode uses worktree_path)
pub task_branch: Option<String>,        // Branch name for this task
pub worktree_path: Option<String>,      // Worktree path (Worktree mode only)
pub merge_commit_sha: Option<String>,   // After successful merge
```

#### 1.3 Project Entity Extension (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(project): add git_mode and worktree_parent_directory fields with migration`

**Files:** `src-tauri/src/domain/entities/project.rs`, migration

```rust
// Add GitMode enum
pub enum GitMode {
    Local,
    Worktree,
}

// Add fields
pub git_mode: GitMode,                          // Default: Worktree
pub worktree_parent_directory: Option<String>,  // Default: ~/ralphx-worktrees
```

> **Note:** The `git_mode` field is used throughout the codebase (2.1, 2.1b, 2.3, 2.4, 5.1, 5.2) to distinguish between Local and Worktree modes. Frontend types and Zod schemas must also be updated.

#### 1.4 New Internal States (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(status): add PendingMerge, Merging, MergeConflict, Merged states`

**Files:** `src-tauri/src/domain/entities/status.rs`, frontend types

Add: `PendingMerge`, `Merging`, `MergeConflict`, `Merged`

> **Note:** This task must include both backend status.rs changes AND frontend type definitions for runtime correctness. While each compiles independently, Zod validation fails at runtime if backend sends states the frontend schema doesn't recognize. New states are additive so no existing code breaks.

### Phase 2: Transition Integration

#### 2.1 Branch/Worktree Setup on Executing
**Dependencies:** Task 1.1 (GitService), Task 1.2 (Task entity), Task 1.3 (Project entity)
**Atomic Commit:** `feat(execution): create branch/worktree on task execution start`

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`

```rust
// on_enter(Executing)
if task.task_branch.is_none() {
    let branch = format!("ralphx/{}/task-{}", slugify(&project.name), task.id);

    match project.git_mode {
        GitMode::Local => {
            // IMPORTANT: Block if uncommitted changes exist (Clarification #1)
            if git_service.has_uncommitted_changes(&project.working_directory)? {
                return Err(AppError::ExecutionBlocked(
                    "Cannot execute task: uncommitted changes in working directory. \
                     Please commit or stash your changes first.".to_string()
                ));
            }
            // Create and checkout branch in main repo
            git_service.create_branch(&project.working_directory, &branch, &project.base_branch)?;
            git_service.checkout_branch(&project.working_directory, &branch)?;
        }
        GitMode::Worktree => {
            // Create worktree with new branch (parent dir auto-created in GitService)
            let worktree_path = format!("{}/{}/task-{}",
                project.worktree_parent_directory.as_deref().unwrap_or("~/ralphx-worktrees"),
                slugify(&project.name),
                task.id
            );
            git_service.create_worktree(&project.working_directory, &worktree_path, &branch, &project.base_branch)?;
            task.worktree_path = Some(worktree_path);
        }
    }

    task.task_branch = Some(branch);
    task_repo.update(&task).await?;
}
```

#### 2.1b Local Mode Queue Enforcement in Scheduler
**Dependencies:** Task 1.3 (Project entity - for git_mode field)
**Atomic Commit:** `feat(scheduler): enforce single running task per Local-mode project`

**File:** `src-tauri/src/application/task_scheduler_service.rs`

Modify `find_oldest_ready_task()` to skip tasks from Local-mode projects that already have a running task:

```rust
async fn find_oldest_schedulable_task(&self) -> Option<Task> {
    let ready_tasks = self.task_repo.get_oldest_ready_tasks().await.ok()?;

    for task in ready_tasks {
        let project = self.project_repo.get_by_id(&task.project_id).await.ok()??;

        if project.git_mode == GitMode::Local {
            // Check if this project already has an executing task
            let running_states = vec![
                InternalStatus::Executing,
                InternalStatus::ReExecuting,
                InternalStatus::Reviewing,
                InternalStatus::Merging,
            ];
            let has_running = self.task_repo
                .has_task_in_states(&project.id, &running_states)
                .await
                .unwrap_or(false);

            if has_running {
                continue;  // Skip, try next task in queue
            }
        }

        return Some(task);
    }
    None
}
```

> **Note:** Requires new repository method `has_task_in_states(project_id, statuses) -> bool`

#### 2.2 Auto-Commit on ExecutionDone
**Dependencies:** Task 1.1 (GitService), Task 2.1 (Branch setup)
**Atomic Commit:** `feat(execution): auto-commit on task execution completion`

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`

```rust
// on_exit(Executing) or before QA/PendingReview
if settings.execution.auto_commit {
    let working_path = resolve_working_directory(&task, &project);

    if git_service.has_uncommitted_changes(&working_path)? {
        // Programmatic message: {prefix}{task_title}
        let message = format!("{}{}",
            settings.execution.commit_message_prefix,
            task.title
        );
        git_service.commit_all(&working_path, &message)?;
    }
}
```

#### 2.3 Working Directory Resolution Update
**Dependencies:** Task 1.2 (Task entity), Task 1.3 (Project entity)
**Atomic Commit:** `feat(chat): update working directory resolution for worktree mode`

**File:** `src-tauri/src/application/chat_service/chat_service_context.rs`

Update `resolve_working_directory()` to use new logic (check task.worktree_path for Worktree mode).

#### 2.4 Branch Checkout for Local Mode (ReExecuting/Reviewing)
**Dependencies:** Task 1.1 (GitService), Task 2.1 (Branch setup)
**Atomic Commit:** `feat(execution): checkout task branch on re-executing/reviewing in local mode`

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`

```rust
// on_enter(ReExecuting) or on_enter(Reviewing) - Local mode only
if project.git_mode == GitMode::Local {
    if let Some(branch) = &task.task_branch {
        let current = git_service.get_current_branch(&project.working_directory)?;
        if current != *branch {
            git_service.checkout_branch(&project.working_directory, branch)?;
        }
    }
}
```

#### 2.5 Auto-Transition: Approved → PendingMerge (BLOCKING)
**Dependencies:** Task 1.4 (New states)
**Atomic Commit:** `feat(state-machine): add auto-transition from Approved to PendingMerge`

**File:** `src-tauri/src/domain/state_machine/transition_handler/mod.rs`

```rust
fn check_auto_transition(&self, state: &State) -> Option<State> {
    match state {
        State::Approved => Some(State::PendingMerge),  // Both modes
        // NOTE: PendingMerge does NOT auto-transition - side effect determines next state
        // ... existing
    }
}
```

#### 2.6 Programmatic Merge Attempt (Phase 1) (BLOCKING)
**Dependencies:** Task 1.1 (GitService), Task 1.4 (New states), Task 2.5 (Auto-transition)
**Atomic Commit:** `feat(merge): implement programmatic rebase and merge on PendingMerge`

**File:** `src-tauri/src/domain/state_machine/transition_handler/side_effects.rs`

```rust
// on_enter(PendingMerge)
let result = git_service.try_rebase_and_merge(
    &project.working_directory,
    task.task_branch.as_ref().unwrap(),
    &project.base_branch.as_deref().unwrap_or("main")
)?;

match result {
    MergeAttemptResult::Success { commit_sha } => {
        // Fast path: merge succeeded without agent
        task.merge_commit_sha = Some(commit_sha);
        transition_to(State::Merged);  // Skip Merging state
        cleanup_branch_and_worktree(&task, &project);
    }
    MergeAttemptResult::NeedsAgent { conflict_files } => {
        // Store conflict context for agent
        task.metadata.insert("conflict_files", conflict_files);
        transition_to(State::Merging);  // Agent will handle
    }
}
```

### Phase 3: Merge Agent & Workflow

#### 3.1 Merger Agent Definition
**Dependencies:** None (plugin file, no compilation dependencies)
**Atomic Commit:** `feat(plugin): add merger agent definition for conflict resolution`

**File:** `ralphx-plugin/agents/merger.md` (NEW)

```yaml
---
name: ralphx-merger
trigger: status:merging
agent: merger
tools:
  - Bash
  - Read
  - Edit
preapprovedTools:
  - Bash
  - Read
  - Edit
allowedMcpTools:
  - complete_merge
  - report_conflict
  - get_task_context
---

You are the RalphX Merger Agent. Your job is to resolve merge conflicts that the programmatic merge couldn't handle.

## Context
A programmatic rebase + merge was already attempted and failed. Conflict files are stored in task metadata.

## Process
1. Get task context to understand what was changed
2. Review the conflict files (stored in task.metadata.conflict_files)
3. For each conflict:
   a. Read the file to understand the conflict markers
   b. Analyze the incoming vs current changes
   c. Determine the correct resolution
   d. Edit the file to resolve the conflict
4. After resolving all conflicts:
   a. Stage changes: `git add .`
   b. Complete the merge: `git commit -m "Merge branch '{task_branch}'"`
   c. Call `complete_merge` with the commit SHA
5. If you cannot resolve a conflict (complex logic, ambiguous changes):
   a. Call `report_conflict` with the remaining conflict files
   b. The user will resolve manually
```

#### 3.2 Merge MCP Tools (BLOCKING)
**Dependencies:** Task 1.4 (New states - for status transitions)
**Atomic Commit:** `feat(mcp): add complete_merge and report_conflict tools`

**File:** `ralphx-plugin/ralphx-mcp-server/src/tools.ts`

```typescript
complete_merge: async (taskId: string, commitSha: string) => {
    // 1. Transition task: Merging → Merged
    // 2. Trigger cleanup (delete branch, delete worktree if applicable)
}

report_conflict: async (taskId: string, conflictFiles: string[]) => {
    // 1. Transition task: Merging → MergeConflict
    // 2. Store conflict file list in task metadata
    // 3. Keep branch/worktree for manual resolution
}
```

#### 3.3 HTTP Handlers for Git Operations (BLOCKING)
**Dependencies:** Task 1.1 (GitService), Task 1.4 (New states)
**Atomic Commit:** `feat(http): add git handlers for merge operations`

**File:** `src-tauri/src/http_server/handlers/git.rs` (NEW)

```rust
// POST /git/tasks/{id}/complete-merge
pub async fn complete_merge(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(payload): Json<CompleteMergeRequest>,
) -> Result<Json<Task>, AppError> {
    // 1. Validate task is in Merging state
    // 2. Set task.merge_commit_sha = payload.commit_sha
    // 3. Transition task: Merging → Merged
    // 4. Trigger cleanup (delete branch, delete worktree if applicable)
}

// POST /git/tasks/{id}/report-conflict
pub async fn report_conflict(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(payload): Json<ReportConflictRequest>,
) -> Result<Json<Task>, AppError> {
    // 1. Validate task is in Merging state
    // 2. Store conflict file list in task metadata
    // 3. Transition task: Merging → MergeConflict
    // 4. Keep branch/worktree for manual resolution
}

// GET /git/tasks/{id}/commits
pub async fn get_task_commits(...) -> Result<Json<Vec<CommitInfo>>, AppError>

// GET /git/tasks/{id}/diff-stats
pub async fn get_task_diff_stats(...) -> Result<Json<DiffStats>, AppError>
```

**Register in `mod.rs`:** Add `pub mod git;` and wire routes.

#### 3.4 Agent Config
**Dependencies:** Task 3.1 (Merger agent definition), Task 3.2 (MCP tools), Task 3.3 (HTTP handlers)
**Atomic Commit:** `feat(agents): add ralphx-merger agent configuration`

**File:** `src-tauri/src/infrastructure/agents/claude/agent_config.rs`

Add `ralphx-merger` with tool permissions.

### Phase 4: Tauri Commands

#### 4.1 Git Commands (BLOCKING)
**Dependencies:** Task 1.1 (GitService), Task 1.2 (Task entity), Task 1.4 (New states)
**Atomic Commit:** `feat(commands): add git commands for task commits, diff, merge, and cleanup`

**File:** `src-tauri/src/commands/git_commands.rs` (NEW)

```rust
#[tauri::command]
pub async fn get_task_commits(task_id: String) -> Result<Vec<CommitInfo>>

#[tauri::command]
pub async fn get_task_diff_stats(task_id: String) -> Result<DiffStats>

#[tauri::command]
pub async fn resolve_merge_conflict(task_id: String) -> Result<()>
// User clicked "Conflicts Resolved" after manual resolution

#[tauri::command]
pub async fn retry_merge(task_id: String) -> Result<()>
// Re-attempt merge after user made changes

#[tauri::command]
pub async fn cleanup_task_branch(task_id: String) -> Result<()>
// Manual cleanup for failed/cancelled tasks

#[tauri::command]
pub async fn change_project_git_mode(project_id: String, mode: String) -> Result<()>
// Switch between Local/Worktree
```

### Phase 5: UI Updates

#### 5.1 Project Creation Wizard
**Dependencies:** Task 1.3 (Project entity - for worktree_parent_directory field)
**Atomic Commit:** `feat(ui): update project wizard with git mode selection and worktree defaults`

**Files:** `src/components/projects/ProjectCreationWizard/`

- Set **Worktree as DEFAULT**
- Add "(Not recommended for concurrent tasks)" label to Local mode
- Remove worktree_path input (auto-generated)
- Keep base_branch selector
- Add optional `worktree_parent_directory` in collapsed "Advanced" section

#### 5.2 Project Settings - Git Section (NEW)
**Dependencies:** Task 1.3 (Project entity), Task 4.1 (Git commands - for change_project_git_mode)
**Atomic Commit:** `feat(ui): add GitSettingsSection for project git mode configuration`

**File:** `src/components/settings/GitSettingsSection.tsx` (NEW)

```tsx
<SectionCard icon={<GitBranch />} title="Git" description="Version control settings">
  <SelectSettingRow
    label="Git Mode"
    value={project.gitMode}
    options={[
      { value: 'worktree', label: 'Isolated Worktrees (Recommended)' },
      { value: 'local', label: 'Local Branches' }
    ]}
    onChange={changeGitMode}
  />
  <DisplayRow label="Base Branch" value={project.baseBranch} />
  {project.gitMode === 'worktree' && (
    <TextSettingRow
      label="Worktree Location"
      value={project.worktreeParentDirectory || '~/ralphx-worktrees'}
      onChange={updateWorktreeParent}
    />
  )}
</SectionCard>
```

#### 5.3 New Task Detail Views
**Dependencies:** Task 1.4 (New states), Task 4.1 (Git commands - for retry_merge, resolve_merge_conflict)
**Atomic Commit:** `feat(ui): add Merging, MergeConflict, and Merged task detail views`

**Files:** `src/components/tasks/detail-views/`

| Component | States Handled | Features |
|-----------|----------------|----------|
| `MergingTaskDetail.tsx` | `pending_merge`, `merging` | Combined view (see below) |
| `MergeConflictTaskDetail.tsx` | `merge_conflict` | Conflict files, actions, read-only chat |
| `MergedTaskDetail.tsx` | `merged` | Completion info, read-only chat |

**MergingTaskDetail (handles both pending_merge and merging):**
```tsx
function MergingTaskDetail({ task, isHistorical }: TaskDetailProps) {
  const isProgrammaticPhase = task.internalStatus === 'pending_merge';
  const isAgentPhase = task.internalStatus === 'merging';

  return (
    <div>
      {isProgrammaticPhase && (
        <>
          <Header>Merging...</Header>
          <Content>Attempting to merge branch into {task.project.baseBranch}</Content>
          <Spinner />
          {/* No chat panel - programmatic merge has no agent */}
        </>
      )}
      {isAgentPhase && (
        <>
          <Header>Resolving Merge Conflicts</Header>
          <ConflictFileList files={task.metadata.conflict_files} />
          <ChatPanel contextType="merge" taskId={task.id} />
        </>
      )}
    </div>
  );
}
```

**Chat context integration:**
- Add `"merge"` to `TaskContextType` in `useTaskChat.ts`
- `MergingTaskDetail` (merging state): Live chat with `useTaskChat(taskId, "merge")`
- `MergeConflictTaskDetail`: Read-only chat (disable send, show historical)
- `MergedTaskDetail`: Read-only historical chat

**History navigation (Phase 64 integration):**
- All views accept `isHistorical?: boolean` prop
- When `isHistorical=true`: hide action buttons, show read-only state
- Use `conversationId` and `agentRunId` from state history metadata for correct chat scroll position

**Registry update in `TaskDetailPanel.tsx`:**
```typescript
const TASK_DETAIL_VIEWS: Record<InternalStatus, ComponentType<TaskDetailProps>> = {
  // ... existing mappings
  pending_merge: MergingTaskDetail,
  merging: MergingTaskDetail,  // Same component handles both
  merge_conflict: MergeConflictTaskDetail,
  merged: MergedTaskDetail,
};
```

**Compilation unit requirements:**
- **MUST be same commit:** View components + registry update (TypeScript import error if registry references non-existent components)
- **SHOULD be same commit:** "merge" context type (logically coupled, but compiles independently)

#### 5.4 Task Card Branch Badge
**Dependencies:** Task 1.2 (Task entity - for task_branch field)
**Atomic Commit:** `feat(ui): add branch name badge to task cards`

**File:** `src/components/tasks/TaskCard.tsx`

Show branch name badge for tasks with active branches.

#### 5.5 Kanban Done Column: Merge State Subgroups
**Dependencies:** Task 1.4 (New states), Task 5.3 (Detail views)
**Atomic Commit:** `feat(ui): add merge state subgroups to Done column`

**Decision:** Keep merge states in Done column with subgroup support (similar to InReview column).

**Subgroup configuration:**

| Group | States | Visual Style | Sort Order |
|-------|--------|--------------|------------|
| Merging | `pending_merge`, `merging` | Default | 1 (top) |
| Needs Attention | `merge_conflict` | Warning (amber) | 2 |
| Completed | `merged`, `approved` | Success (muted green) | 3 |
| Terminal | `failed`, `cancelled` | Muted | 4 (bottom) |

**Implementation:**
1. Add `subgroups` config to Done column definition
2. `MergeConflict` cards get warning border/badge styling
3. Group headers collapsible like InReview column

**Files to modify:**
- `src/components/tasks/KanbanBoard.tsx` - column subgroup rendering
- `src/types/kanban.ts` - add subgroup types
- Column configuration constants

---

## Critical Files

| File | Changes |
|------|---------|
| `src-tauri/src/application/git_service.rs` | NEW - Core git operations |
| `src-tauri/src/application/task_scheduler_service.rs` | Local mode queue enforcement |
| `src-tauri/src/commands/git_commands.rs` | NEW - Tauri commands |
| `src-tauri/src/http_server/handlers/git.rs` | NEW - HTTP endpoints for MCP |
| `src-tauri/src/domain/entities/status.rs` | Add 4 new states |
| `src-tauri/src/domain/entities/task.rs` | Add task_branch, worktree_path, merge_commit_sha |
| `src-tauri/src/domain/entities/project.rs` | Add worktree_parent_directory |
| `src-tauri/src/domain/state_machine/transition_handler/` | Branch/worktree lifecycle, uncommitted check |
| `src-tauri/src/application/chat_service/chat_service_context.rs` | Working directory resolution |
| `src-tauri/src/infrastructure/agents/claude/agent_config.rs` | Add merger agent |
| `ralphx-plugin/agents/merger.md` | NEW - Merger agent definition |
| `ralphx-plugin/ralphx-mcp-server/src/tools.ts` | Add merge MCP tools |
| `src/hooks/useTaskChat.ts` | Add "merge" context type |
| `src/components/settings/GitSettingsSection.tsx` | NEW - Git settings UI |
| `src/components/projects/ProjectCreationWizard/` | Default to Worktree |
| `src/components/tasks/detail-views/` | 3 new merge state views (MergingTaskDetail handles 2 states) |
| `src/components/tasks/TaskDetailPanel.tsx` | Registry updates for new states |
| `src/components/tasks/TaskCard.tsx` | Branch name badge |
| `src/components/tasks/KanbanBoard.tsx` | Done column subgroups |
| `src/types/kanban.ts` | Add subgroup types for Done column |

---

## Migration: Mode Switching

**Local → Worktree:**
1. Terminal tasks (Merged, Cancelled, Failed) remain unchanged
2. In-progress tasks complete in Local mode (no worktree created mid-execution)
3. New tasks get worktrees

**Worktree → Local:**
1. Offer to complete pending merges or keep worktrees for manual handling
2. In-progress tasks continue in their worktrees
3. New tasks use branch-only mode

---

## Verification

### Project Setup
1. **Create project** in Worktree mode (default) → Verify stored correctly
2. **Create project** in Local mode → Verify works
3. **Switch modes** → Handles existing tasks gracefully

### Execution Flow
4. **Execute task (Worktree)** → Worktree + branch created
5. **Execute task (Local)** → Branch created, checkout happens
6. **Worker runs** → Operates in correct directory
7. **Task completes** → Auto-commit with `{prefix}{title}` message
8. **Reviewer runs** → Same branch/worktree

### Merge Flow - Phase 1 (Programmatic)
9. **Approve task** → Auto-transitions to PendingMerge
10. **Phase 1 success** (no conflicts) → Skips Merging, goes directly to Merged
11. **Merged state** → Branch/worktree cleaned up automatically

### Merge Flow - Phase 2 (Agent)
12. **Phase 1 conflict** → Transitions to Merging state
13. **Merger agent resolves** → Merged state, cleanup happens
14. **Merger agent fails** → MergeConflict state, UI shows conflict files

### Merge Flow - Manual
15. **MergeConflict** → User resolves in IDE
16. **"Conflicts Resolved" button** → Transitions to Merged

### Local Mode Enforcement
17. **Local mode with uncommitted changes** → Error message, execution blocked
18. **Local mode parallel attempt** → Second task stays in Ready queue, scheduled when first completes
19. **Local mode queue visibility** → "Running 0/1, Queued 2" shown in execution bar

### Edge Cases
20. **Task cancelled/failed with branch** → Manual cleanup option available
21. **Worktree parent doesn't exist** → Auto-created on first worktree

### Chat Context
22. **Merging state** → Live merger agent chat visible
23. **MergeConflict state** → Read-only view of merger conversation
24. **History navigation** → Clicking historical merge state shows correct conversation + scroll position

### Kanban Done Column
25. **MergeConflict visibility** → Warning styling draws attention
26. **Subgroup collapsing** → Groups collapse/expand like InReview column

---

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

## Task Dependency Graph

```
Phase 1 (Infrastructure - No Dependencies, Parallel OK):
┌──────────────────────────────────────────────────────────────────┐
│  1.1 GitService        1.2 Task Entity    1.3 Project Entity    │
│  (BLOCKING)            (BLOCKING)         (BLOCKING)            │
│       │                     │                  │                │
│       └─────────────────────┼──────────────────┘                │
│                             │                                    │
│                       1.4 New States (BLOCKING)                  │
│                             │                                    │
└─────────────────────────────┼────────────────────────────────────┘
                              │
Phase 2 (Transitions - Depends on Phase 1):
┌──────────────────────────────────────────────────────────────────┐
│                             ▼                                    │
│  2.1 Branch Setup          2.1b Scheduler Queue                  │
│  [Deps: 1.1,1.2,1.3]       [Deps: 1.3] (PARALLEL)               │
│       │                         │                                │
│       ├─────────────────────────┤                                │
│       ▼                         ▼                                │
│  2.2 Auto-Commit           2.3 Working Dir                       │
│       │                         │                                │
│       └──────────┬──────────────┘                                │
│                  ▼                                               │
│             2.4 Branch Checkout                                  │
│                  │                                               │
│                  ▼                                               │
│             2.5 Auto-Transition (BLOCKING)                       │
│                  │                                               │
│                  ▼                                               │
│             2.6 Programmatic Merge (BLOCKING)                    │
└──────────────────────────────────────────────────────────────────┘
                               │
Phase 3 (Merge Agent & HTTP - Partial Dependency):
┌──────────────────────────────┼───────────────────────────────────┐
│  3.1 Agent Definition        │                                   │
│       │                      │                                   │
│       └───► 3.2 MCP Tools ◄──┤                                   │
│                  │           │                                   │
│             3.3 HTTP Git ◄───┘  (NEW - endpoints for MCP)        │
│              Handlers (BLOCKING)                                 │
│                  │                                               │
│                  └───► 3.4 Agent Config                          │
└──────────────────────────────────────────────────────────────────┘

Phase 4 (Tauri Commands - Depends on Phase 1):
┌──────────────────────────────────────────────────────────────────┐
│  4.1 Git Commands (BLOCKING)                                     │
│  [Depends on: 1.1, 1.2, 1.4]                                     │
└──────────────────────────────┼───────────────────────────────────┘
                               │
Phase 5 (UI - Depends on Phases 1, 4):
┌──────────────────────────────┼───────────────────────────────────┐
│  5.1 Project Wizard          │                                   │
│  [Depends on: 1.3]           │                                   │
│                              │                                   │
│  5.2 Git Settings ◄──────────┤                                   │
│  [Depends on: 1.3, 4.1]      │                                   │
│                              │                                   │
│  5.3 Task Detail Views ◄─────┤  (MergingTaskDetail handles 2 states)
│  [Depends on: 1.4, 4.1]      │  + chat context "merge" integration │
│                              │                                   │
│  5.4 Task Card Badge         │                                   │
│  [Depends on: 1.2]           │                                   │
│                              │                                   │
│  5.5 Kanban Done Subgroups   │  (Merging, Needs Attention, etc.) │
│  [Depends on: 1.4, 5.3]      │                                   │
└──────────────────────────────────────────────────────────────────┘
```

## Compilation Unit Notes

The following tasks form complete compilation units and can be safely committed independently:

1. **Phase 1 tasks (1.1-1.4):** All are additive (new files, new fields, new states). No breaking changes.
   - **1.4 (New States):** Backend + frontend types together for runtime correctness (not compilation - Zod validation fails if mismatched).

2. **Phase 2 tasks (2.1-2.6):** Each adds behavior without breaking existing code. GitService methods are called only when conditions are met.
   - **2.1 and 2.1b are PARALLEL** (different dependencies, can be done in any order).
   - **2.1b (Scheduler Queue):** Requires new `has_task_in_states()` repo method - include in same commit.

3. **Phase 3 tasks (3.1-3.4):** Plugin files, MCP server, and HTTP handlers are separate compilation targets.
   - **3.2 and 3.3 are PARALLEL** (both depend on 1.4, can be done in any order).
   - **3.3 (HTTP Handlers):** Must register in mod.rs + wire routes in same commit.

4. **Phase 4 tasks (4.1):** New Tauri commands are additive to the command registry.

5. **Phase 5 tasks (5.1-5.5):** Frontend components are additive. New views are registered but only rendered for new states.
   - **5.3 (Task Detail Views):** MUST include view components + registry update in same commit (import dependency).
   - **5.3 (Chat Context):** SHOULD include `"merge"` context type (logically coupled, compiles independently).

**No chicken-egg problems detected:** All tasks are additive. No renames, no removals, no signature changes that would break compilation.
