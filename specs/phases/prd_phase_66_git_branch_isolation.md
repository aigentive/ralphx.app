# RalphX - Phase 66: Per-Task Git Branch Isolation

## Overview

Transform RalphX to use **per-task branch isolation** for all projects. Every executing task gets its own git branch (and optionally a separate worktree), enabling parallel execution and clean merge workflows.

This phase implements a two-phase merge strategy: programmatic rebase+merge for the fast path (no conflicts), with a dedicated merger agent for conflict resolution. It adds four new internal states (PendingMerge, Merging, MergeConflict, Merged) and updates the entire execution pipeline from task start to final merge.

**Reference Plan:**
- `specs/plans/per_task_git_branch_isolation.md` - Detailed architecture, state flows, clarifications, and implementation tasks

## Goals

1. Enable per-task branch isolation with automatic branch/worktree creation on execution start
2. Implement two-phase merge workflow: programmatic fast path + agent-assisted conflict resolution
3. Add Worktree mode as the default for parallel task execution
4. Provide complete UI integration: new task detail views, Done column subgroups, chat context for merge states

## Dependencies

### Phase 64 (Link Conversation IDs to Task State History) - Required

| Dependency | Why Needed |
|------------|------------|
| `conversationId`/`agentRunId` in state history | New merge views need correct chat scroll position for history navigation |

### Phase 65 (Activity Screen UX) - Complete

| Dependency | Why Needed |
|------------|------------|
| Stable activity events | Merge workflow will emit activity events |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/per_task_git_branch_isolation.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/per_task_git_branch_isolation.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Create GitService with branch, worktree, and merge operations",
    "plan_section": "1.1 GitService Creation (BLOCKING)",
    "blocking": [5, 7, 9, 11, 14, 16],
    "blockedBy": [],
    "atomic_commit": "feat(git): create GitService with branch, worktree, and merge operations",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '1.1 GitService Creation'",
      "Create src-tauri/src/application/git_service.rs with all methods",
      "Implement branch operations: create_branch, checkout_branch, delete_branch, get_current_branch",
      "Implement worktree operations: create_worktree (with create_dir_all), delete_worktree",
      "Implement commit operations: commit_all, has_uncommitted_changes",
      "Implement rebase/merge operations: fetch_origin, rebase_onto, abort_rebase, merge_branch, abort_merge, get_conflict_files, try_rebase_and_merge",
      "Implement query operations: get_commits_since, get_diff_stats",
      "Add MergeResult, RebaseResult, MergeAttemptResult enums",
      "Register in src-tauri/src/application/mod.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(git): create GitService with branch, worktree, and merge operations"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add task_branch, worktree_path, merge_commit_sha fields to Task entity",
    "plan_section": "1.2 Task Entity Extension (BLOCKING)",
    "blocking": [5, 8, 16, 20],
    "blockedBy": [],
    "atomic_commit": "feat(task): add task_branch, worktree_path, merge_commit_sha fields with migration",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '1.2 Task Entity Extension'",
      "Add task_branch: Option<String>, worktree_path: Option<String>, merge_commit_sha: Option<String> to Task entity",
      "Create migration v24_task_git_fields.rs with IF NOT EXISTS columns",
      "Register migration in MIGRATIONS array and bump SCHEMA_VERSION",
      "Update TaskRepository to handle new fields",
      "Add tests in v24_task_git_fields_tests.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(task): add task_branch, worktree_path, merge_commit_sha fields with migration"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add git_mode and worktree_parent_directory fields to Project entity",
    "plan_section": "1.3 Project Entity Extension (BLOCKING)",
    "blocking": [5, 6, 8, 17, 18],
    "blockedBy": [],
    "atomic_commit": "feat(project): add git_mode and worktree_parent_directory fields with migration",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '1.3 Project Entity Extension'",
      "Add GitMode enum (Local, Worktree) to project.rs or a shared types file",
      "Add git_mode: GitMode field to Project entity (default: Worktree)",
      "Add worktree_parent_directory: Option<String> to Project entity (default: ~/ralphx-worktrees)",
      "Create migration v25_project_git_fields.rs with git_mode and worktree_parent_directory columns",
      "Register migration and bump SCHEMA_VERSION",
      "Update ProjectRepository to handle new fields",
      "Update frontend types and Zod schemas for git_mode field",
      "Add tests",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(project): add git_mode and worktree_parent_directory fields with migration"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Add PendingMerge, Merging, MergeConflict, Merged internal states",
    "plan_section": "1.4 New Internal States (BLOCKING)",
    "blocking": [10, 11, 13, 14, 16, 19, 21],
    "blockedBy": [],
    "atomic_commit": "feat(status): add PendingMerge, Merging, MergeConflict, Merged states",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '1.4 New Internal States'",
      "Add PendingMerge, Merging, MergeConflict, Merged to InternalStatus enum in status.rs",
      "Update state machine transitions to include new states",
      "Update frontend types: src/types/task.ts - add new status values",
      "Update Zod schemas: src/api/tasks/tasks.schemas.ts - add new status values",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(status): add PendingMerge, Merging, MergeConflict, Merged states"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Create branch/worktree on task execution start",
    "plan_section": "2.1 Branch/Worktree Setup on Executing",
    "blocking": [7, 9],
    "blockedBy": [1, 2, 3],
    "atomic_commit": "feat(execution): create branch/worktree on task execution start",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '2.1 Branch/Worktree Setup on Executing'",
      "Add AppError::ExecutionBlocked(String) variant to error.rs if not exists",
      "Modify on_enter(Executing) in side_effects.rs",
      "For Local mode: check has_uncommitted_changes, create and checkout branch",
      "For Worktree mode: create worktree with branch using naming convention",
      "Store task_branch (both modes) and worktree_path (Worktree mode) on task",
      "Add error handling for uncommitted changes (return AppError::ExecutionBlocked)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(execution): create branch/worktree on task execution start"
    ],
    "passes": true
  },
  {
    "id": 6,
    "category": "backend",
    "description": "Enforce single running task per Local-mode project in scheduler",
    "plan_section": "2.1b Local Mode Queue Enforcement in Scheduler",
    "blocking": [],
    "blockedBy": [3],
    "atomic_commit": "feat(scheduler): enforce single running task per Local-mode project",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '2.1b Local Mode Queue Enforcement'",
      "Add has_task_in_states(project_id, statuses) method to TaskRepository",
      "Modify find_oldest_schedulable_task() in task_scheduler_service.rs",
      "Skip tasks from Local-mode projects that already have a running task",
      "Check running states: Executing, ReExecuting, Reviewing, Merging",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(scheduler): enforce single running task per Local-mode project"
    ],
    "passes": true
  },
  {
    "id": 7,
    "category": "backend",
    "description": "Auto-commit on task execution completion",
    "plan_section": "2.2 Auto-Commit on ExecutionDone",
    "blocking": [],
    "blockedBy": [1, 5],
    "atomic_commit": "feat(execution): auto-commit on task execution completion",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '2.2 Auto-Commit on ExecutionDone'",
      "Add auto-commit logic in on_exit(Executing) or before QA/PendingReview transition",
      "Use resolve_working_directory to get correct path",
      "Check settings.execution.auto_commit flag",
      "Format message as {commit_message_prefix}{task_title}",
      "Call git_service.commit_all if has_uncommitted_changes",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(execution): auto-commit on task execution completion"
    ],
    "passes": true
  },
  {
    "id": 8,
    "category": "backend",
    "description": "Update working directory resolution for worktree mode",
    "plan_section": "2.3 Working Directory Resolution Update",
    "blocking": [],
    "blockedBy": [2, 3],
    "atomic_commit": "feat(chat): update working directory resolution for worktree mode",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '2.3 Working Directory Resolution Update'",
      "Update resolve_working_directory() in chat_service_context.rs",
      "For Local mode: always return project.working_directory",
      "For Worktree mode: return task.worktree_path if exists, else project.working_directory",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(chat): update working directory resolution for worktree mode"
    ],
    "passes": true
  },
  {
    "id": 9,
    "category": "backend",
    "description": "Checkout task branch on re-executing/reviewing in local mode",
    "plan_section": "2.4 Branch Checkout for Local Mode (ReExecuting/Reviewing)",
    "blocking": [],
    "blockedBy": [1, 5],
    "atomic_commit": "feat(execution): checkout task branch on re-executing/reviewing in local mode",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '2.4 Branch Checkout for Local Mode'",
      "Add on_enter(ReExecuting) and on_enter(Reviewing) handlers in side_effects.rs",
      "For Local mode only: checkout task_branch if current branch differs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(execution): checkout task branch on re-executing/reviewing in local mode"
    ],
    "passes": true
  },
  {
    "id": 10,
    "category": "backend",
    "description": "Add auto-transition from Approved to PendingMerge",
    "plan_section": "2.5 Auto-Transition: Approved → PendingMerge (BLOCKING)",
    "blocking": [11],
    "blockedBy": [4],
    "atomic_commit": "feat(state-machine): add auto-transition from Approved to PendingMerge",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '2.5 Auto-Transition'",
      "Modify check_auto_transition() in transition_handler/mod.rs",
      "Add case: State::Approved => Some(State::PendingMerge)",
      "Note: PendingMerge does NOT auto-transition - side effect determines next state",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(state-machine): add auto-transition from Approved to PendingMerge"
    ],
    "passes": true
  },
  {
    "id": 11,
    "category": "backend",
    "description": "Implement programmatic rebase and merge on PendingMerge",
    "plan_section": "2.6 Programmatic Merge Attempt (Phase 1) (BLOCKING)",
    "blocking": [],
    "blockedBy": [1, 4, 10],
    "atomic_commit": "feat(merge): implement programmatic rebase and merge on PendingMerge",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '2.6 Programmatic Merge Attempt'",
      "Add on_enter(PendingMerge) handler in side_effects.rs",
      "Call git_service.try_rebase_and_merge(repo, task_branch, base_branch)",
      "On Success: set merge_commit_sha, transition to Merged, cleanup branch/worktree",
      "On NeedsAgent: store conflict_files in metadata, transition to Merging",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(merge): implement programmatic rebase and merge on PendingMerge"
    ],
    "passes": true
  },
  {
    "id": 12,
    "category": "agent",
    "description": "Add merger agent definition for conflict resolution",
    "plan_section": "3.1 Merger Agent Definition",
    "blocking": [13],
    "blockedBy": [],
    "atomic_commit": "feat(plugin): add merger agent definition for conflict resolution",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '3.1 Merger Agent Definition'",
      "Create ralphx-plugin/agents/merger.md with YAML frontmatter",
      "Configure trigger: status:merging, tools: Bash/Read/Edit",
      "Configure allowedMcpTools: complete_merge, report_conflict, get_task_context",
      "Write agent prompt for conflict resolution workflow",
      "Commit: feat(plugin): add merger agent definition for conflict resolution"
    ],
    "passes": true
  },
  {
    "id": 13,
    "category": "mcp",
    "description": "Add complete_merge and report_conflict MCP tools",
    "plan_section": "3.2 Merge MCP Tools (BLOCKING)",
    "blocking": [15],
    "blockedBy": [4, 12],
    "atomic_commit": "feat(mcp): add complete_merge and report_conflict tools",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '3.2 Merge MCP Tools'",
      "Add complete_merge tool to ralphx-mcp-server/src/tools.ts",
      "Add report_conflict tool to ralphx-mcp-server/src/tools.ts",
      "complete_merge: transitions Merging → Merged, triggers cleanup",
      "report_conflict: transitions Merging → MergeConflict, stores conflict files",
      "Run npm run lint && npm run typecheck in ralphx-mcp-server",
      "Commit: feat(mcp): add complete_merge and report_conflict tools"
    ],
    "passes": true
  },
  {
    "id": 14,
    "category": "backend",
    "description": "Add git handlers for merge operations",
    "plan_section": "3.3 HTTP Handlers for Git Operations (BLOCKING)",
    "blocking": [15],
    "blockedBy": [1, 4],
    "atomic_commit": "feat(http): add git handlers for merge operations",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '3.3 HTTP Handlers for Git Operations'",
      "Create src-tauri/src/http_server/handlers/git.rs",
      "Implement POST /git/tasks/{id}/complete-merge",
      "Implement POST /git/tasks/{id}/report-conflict",
      "Implement GET /git/tasks/{id}/commits",
      "Implement GET /git/tasks/{id}/diff-stats",
      "Register in mod.rs and wire routes",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(http): add git handlers for merge operations"
    ],
    "passes": true
  },
  {
    "id": 15,
    "category": "backend",
    "description": "Add ralphx-merger agent configuration",
    "plan_section": "3.4 Agent Config",
    "blocking": [],
    "blockedBy": [12, 13, 14],
    "atomic_commit": "feat(agents): add ralphx-merger agent configuration",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '3.4 Agent Config'",
      "Add ralphx-merger agent config in agent_config.rs",
      "Configure tool permissions for Bash, Read, Edit",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(agents): add ralphx-merger agent configuration"
    ],
    "passes": true
  },
  {
    "id": 16,
    "category": "backend",
    "description": "Add git commands for task commits, diff, merge, and cleanup",
    "plan_section": "4.1 Git Commands (BLOCKING)",
    "blocking": [18, 19],
    "blockedBy": [1, 2, 4],
    "atomic_commit": "feat(commands): add git commands for task commits, diff, merge, and cleanup",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '4.1 Git Commands'",
      "Create src-tauri/src/commands/git_commands.rs",
      "Implement get_task_commits, get_task_diff_stats commands",
      "Implement resolve_merge_conflict, retry_merge commands",
      "Implement cleanup_task_branch, change_project_git_mode commands",
      "Register commands in main.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(commands): add git commands for task commits, diff, merge, and cleanup"
    ],
    "passes": true
  },
  {
    "id": 17,
    "category": "frontend",
    "description": "Update project wizard with git mode selection and worktree defaults",
    "plan_section": "5.1 Project Creation Wizard",
    "blocking": [],
    "blockedBy": [3],
    "atomic_commit": "feat(ui): update project wizard with git mode selection and worktree defaults",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '5.1 Project Creation Wizard'",
      "Update ProjectCreationWizard to default to Worktree mode",
      "Add '(Not recommended for concurrent tasks)' label to Local mode",
      "Remove worktree_path input (auto-generated)",
      "Keep base_branch selector",
      "Add optional worktree_parent_directory in collapsed 'Advanced' section",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ui): update project wizard with git mode selection and worktree defaults"
    ],
    "passes": true
  },
  {
    "id": 18,
    "category": "frontend",
    "description": "Add GitSettingsSection for project git mode configuration",
    "plan_section": "5.2 Project Settings - Git Section (NEW)",
    "blocking": [],
    "blockedBy": [3, 16],
    "atomic_commit": "feat(ui): add GitSettingsSection for project git mode configuration",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '5.2 Project Settings - Git Section'",
      "Create src/components/settings/GitSettingsSection.tsx",
      "Add Git Mode selector: Worktree (Recommended) / Local Branches",
      "Display Base Branch as read-only",
      "Show Worktree Location setting when gitMode === 'worktree'",
      "Wire to changeGitMode Tauri command",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ui): add GitSettingsSection for project git mode configuration"
    ],
    "passes": true
  },
  {
    "id": 19,
    "category": "frontend",
    "description": "Add Merging, MergeConflict, and Merged task detail views",
    "plan_section": "5.3 New Task Detail Views",
    "blocking": [21],
    "blockedBy": [4, 16],
    "atomic_commit": "feat(ui): add Merging, MergeConflict, and Merged task detail views",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '5.3 New Task Detail Views'",
      "Create src/components/tasks/detail-views/MergingTaskDetail.tsx (handles pending_merge + merging)",
      "Create src/components/tasks/detail-views/MergeConflictTaskDetail.tsx",
      "Create src/components/tasks/detail-views/MergedTaskDetail.tsx",
      "Add 'merge' to TaskContextType in useTaskChat.ts",
      "Update TASK_DETAIL_VIEWS registry in TaskDetailPanel.tsx",
      "All views accept isHistorical?: boolean prop",
      "MergingTaskDetail: spinner for pending_merge, chat for merging",
      "MergeConflictTaskDetail: conflict files list, read-only chat, resolve button",
      "MergedTaskDetail: completion info, read-only historical chat",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ui): add Merging, MergeConflict, and Merged task detail views"
    ],
    "passes": true
  },
  {
    "id": 20,
    "category": "frontend",
    "description": "Add branch name badge to task cards",
    "plan_section": "5.4 Task Card Branch Badge",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "feat(ui): add branch name badge to task cards",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '5.4 Task Card Branch Badge'",
      "Update src/components/tasks/TaskCard.tsx",
      "Add branch name badge for tasks with active task_branch",
      "Style badge to be subtle but visible",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ui): add branch name badge to task cards"
    ],
    "passes": true
  },
  {
    "id": 21,
    "category": "frontend",
    "description": "Add merge state subgroups to Done column",
    "plan_section": "5.5 Kanban Done Column: Merge State Subgroups",
    "blocking": [],
    "blockedBy": [4, 19],
    "atomic_commit": "feat(ui): add merge state subgroups to Done column",
    "steps": [
      "Read specs/plans/per_task_git_branch_isolation.md section '5.5 Kanban Done Column: Merge State Subgroups'",
      "Update src/components/tasks/KanbanBoard.tsx with subgroup rendering",
      "Add subgroup types to src/types/kanban.ts",
      "Configure subgroups: Merging, Needs Attention (amber for merge_conflict), Completed, Terminal",
      "Style MergeConflict cards with warning border/badge",
      "Make group headers collapsible like InReview column",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ui): add merge state subgroups to Done column"
    ],
    "passes": true
  }
]
```

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Worktree mode as default** | Enables parallel task execution without branch switching conflicts |
| **Two-phase merge strategy** | Most merges succeed programmatically (fast path); agent only needed for actual conflicts |
| **Local mode uncommitted changes blocking** | Prevents silent data loss from branch switching with dirty working directory |
| **MergingTaskDetail handles two states** | Seamless UX during merge process; PendingMerge is typically very brief (1-3 seconds) |
| **Queue-based enforcement for Local mode** | Tasks stay visible in Ready queue while waiting; no manual intervention needed |
| **Separate git.rs HTTP handler** | Keeps git operations separate from task CRUD; aligns with domain-based handler separation |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] GitService branch operations work correctly
- [ ] GitService worktree operations create and delete worktrees
- [ ] GitService merge operations handle success and conflict cases
- [ ] New migrations run correctly
- [ ] State transitions for new states work

### Frontend - Run `npm run test`
- [ ] New task detail views render correctly
- [ ] TaskContextType includes "merge"
- [ ] Kanban Done column subgroups display

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing

#### Project Setup
- [ ] Create project in Worktree mode (default) → Verify stored correctly
- [ ] Create project in Local mode → Verify works
- [ ] Switch modes → Handles existing tasks gracefully

#### Execution Flow
- [ ] Execute task (Worktree) → Worktree + branch created
- [ ] Execute task (Local) → Branch created, checkout happens
- [ ] Worker runs → Operates in correct directory
- [ ] Task completes → Auto-commit with {prefix}{title} message

#### Merge Flow - Phase 1 (Programmatic)
- [ ] Approve task → Auto-transitions to PendingMerge
- [ ] Phase 1 success (no conflicts) → Skips Merging, goes directly to Merged
- [ ] Merged state → Branch/worktree cleaned up automatically

#### Merge Flow - Phase 2 (Agent)
- [ ] Phase 1 conflict → Transitions to Merging state
- [ ] Merger agent resolves → Merged state, cleanup happens
- [ ] Merger agent fails → MergeConflict state, UI shows conflict files

#### Merge Flow - Manual
- [ ] MergeConflict → User resolves in IDE
- [ ] "Conflicts Resolved" button → Transitions to Merged

#### Local Mode Enforcement
- [ ] Local mode with uncommitted changes → Error message, execution blocked
- [ ] Local mode parallel attempt → Second task stays in Ready queue
- [ ] Local mode queue visibility → "Running 0/1, Queued 2" shown

#### Chat Context
- [ ] Merging state → Live merger agent chat visible
- [ ] MergeConflict state → Read-only view of merger conversation
- [ ] History navigation → Clicking historical merge state shows correct conversation

#### Kanban Done Column
- [ ] MergeConflict visibility → Warning styling draws attention
- [ ] Subgroup collapsing → Groups collapse/expand like InReview column

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (click handler, route, event listener)
- [ ] New component is imported AND rendered (not behind disabled flag)
- [ ] API wrappers call backend commands
- [ ] State changes reflect in UI

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
