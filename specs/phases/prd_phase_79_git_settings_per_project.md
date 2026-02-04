# RalphX - Phase 79: Git Settings Per-Project

## Overview

Make Git settings fully editable per project, including base branch updates, with automatic default-branch detection (preferring repo default via git, then main/master). Apply same detection in the project creation wizard.

Currently, the Git settings UI only allows changing git mode while showing base branch as read-only. Project creation defaults to `main`/`master` instead of detecting the repo's actual default branch. This phase adds a backend command for default-branch detection and wires it to both the settings UI and project creation wizard.

**Reference Plan:**
- `specs/plans/git_settings_per_project.plan.md` - Detailed implementation plan with task breakdown and dependency graph

## Goals

1. Add backend command for git default-branch detection with fallback chain
2. Make base branch and worktree directory editable and persisted per project
3. Use default-branch detection in project creation wizard
4. Provide "detect default" action for manual refresh

## Dependencies

### Phase 77 (Execution Settings Persistence) - Context

| Dependency | Why Needed |
|------------|------------|
| Settings persistence pattern | Similar pattern for per-project settings storage |
| `projectsApi.update` | Already exists for updating project fields |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/git_settings_per_project.plan.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/git_settings_per_project.plan.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add get_git_default_branch command with fallback chain",
    "plan_section": "Task 1: Add backend command for default-branch detection",
    "blocking": [2, 5],
    "blockedBy": [],
    "atomic_commit": "feat(projects): add get_git_default_branch command",
    "steps": [
      "Read specs/plans/git_settings_per_project.plan.md section 'Task 1'",
      "Add get_git_default_branch command to src-tauri/src/commands/project_commands.rs",
      "Implement fallback chain: origin/HEAD -> main -> master -> first branch",
      "Use git symbolic-ref or git remote show origin for detection",
      "Register command in Tauri app builder",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(projects): add get_git_default_branch command"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Add getGitDefaultBranch to projectsApi",
    "plan_section": "Task 2: Expose default-branch command in frontend API",
    "blocking": [3, 4],
    "blockedBy": [1],
    "atomic_commit": "feat(api): add getGitDefaultBranch to projectsApi",
    "steps": [
      "Read specs/plans/git_settings_per_project.plan.md section 'Task 2'",
      "Add getGitDefaultBranch function to src/api/projects.ts",
      "Add schema if needed in src/api/projects.schemas.ts",
      "Add types if needed in src/api/projects.types.ts",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(api): add getGitDefaultBranch to projectsApi"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Make base branch and worktree directory editable in settings UI",
    "plan_section": "Task 3: Make base branch + worktree dir editable in settings UI",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "feat(settings): add editable base branch and worktree directory",
    "steps": [
      "Read specs/plans/git_settings_per_project.plan.md section 'Task 3'",
      "Update GitSettingsSection.tsx to make base branch editable (dropdown or input)",
      "Add 'detect default' button that calls getGitDefaultBranch",
      "Wire base branch changes to projectsApi.update",
      "Make worktreeParentDirectory editable and persisted via projectsApi.update",
      "Ensure changes are scoped to active project only",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(settings): add editable base branch and worktree directory"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Use default-branch detection in project creation wizard",
    "plan_section": "Task 4: Use default-branch detection in creation wizard",
    "blocking": [],
    "blockedBy": [2],
    "atomic_commit": "feat(wizard): use default-branch detection in project creation",
    "steps": [
      "Read specs/plans/git_settings_per_project.plan.md section 'Task 4'",
      "Update ProjectCreationWizard.tsx to call getGitDefaultBranch when working directory is selected",
      "Fall back to main/master if detection fails",
      "Ensure selected base branch is included in project creation submit",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(wizard): use default-branch detection in project creation"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Add tests for default-branch detection",
    "plan_section": "Task 5: Add tests for default-branch detection",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "test(projects): add tests for default-branch detection",
    "steps": [
      "Read specs/plans/git_settings_per_project.plan.md section 'Task 5'",
      "Add unit tests for get_git_default_branch command in appropriate test file",
      "Test fallback chain behavior (origin/HEAD, main, master, first branch)",
      "Test error cases (no git repo, no branches)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: test(projects): add tests for default-branch detection"
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
| **Git plumbing for detection** | `git symbolic-ref refs/remotes/origin/HEAD` is more reliable than parsing remote output |
| **Fallback chain** | origin/HEAD -> main -> master -> first branch handles repos without remote, new repos, and non-standard defaults |
| **Per-project scope** | Settings changes affect only the active project, not global defaults |
| **Reuse existing update_project** | Backend already supports base_branch updates, no new endpoint needed |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] get_git_default_branch returns correct branch for repo with origin/HEAD
- [ ] Fallback to main works when origin/HEAD unavailable
- [ ] Fallback to master works when main unavailable
- [ ] Returns first branch when main/master unavailable
- [ ] Handles repos without remote gracefully

### Frontend - Run `npm run test`
- [ ] getGitDefaultBranch API wrapper calls correct command
- [ ] GitSettingsSection persists base branch changes
- [ ] ProjectCreationWizard uses detected default branch

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Open Settings > Git, change base branch, verify persisted after reload
- [ ] Open Settings > Git, click "detect default", verify correct branch selected
- [ ] Create new project with git repo, verify default branch auto-detected
- [ ] Create new project without git repo, verify fallback to main works
- [ ] Change worktree directory in settings, verify persisted

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (Settings page, Project wizard)
- [ ] New command registered in Tauri and callable from frontend
- [ ] API wrapper calls backend command
- [ ] Settings changes persist via projectsApi.update
- [ ] UI reflects persisted values on reload

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
