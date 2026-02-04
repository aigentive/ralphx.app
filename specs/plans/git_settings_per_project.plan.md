---
name: Git settings per-project
overview: Make Git settings fully editable per project, including base branch updates, with automatic default-branch detection (preferring repo default via git, then main/master). Apply same detection in the project creation wizard.
todos:
  - id: backend-default-branch
    content: Add git default-branch command with fallbacks
    status: pending
  - id: frontend-api
    content: Expose default-branch command in projects API
    status: pending
  - id: settings-ui
    content: Make base branch + worktree dir editable/persisted
    status: pending
  - id: creation-wizard
    content: Use default-branch detection in wizard
    status: pending
isProject: false
---

# Git Settings UI Fix Plan

## Context

- Git settings UI lives in [`/Users/lazabogdan/Code/ralphx/src/components/settings/GitSettingsSection.tsx`](/Users/lazabogdan/Code/ralphx/src/components/settings/GitSettingsSection.tsx) and currently only changes git mode and shows base branch read-only.
- Project creation already fetches branches but defaults to `main`/`master` instead of the repo default in [`/Users/lazabogdan/Code/ralphx/src/components/projects/ProjectCreationWizard/ProjectCreationWizard.tsx`](/Users/lazabogdan/Code/ralphx/src/components/projects/ProjectCreationWizard/ProjectCreationWizard.tsx).
- Backend already supports project updates including `base_branch` in [`/Users/lazabogdan/Code/ralphx/src-tauri/src/commands/project_commands.rs`](/Users/lazabogdan/Code/ralphx/src-tauri/src/commands/project_commands.rs) via `update_project`.

## Plan

### Task 1: Add backend command for default-branch detection (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(projects): add get_git_default_branch command`

Add a backend command for default-branch detection (`get_git_default_branch`) that uses git plumbing to resolve the repo's default branch (prefer `origin/HEAD`, fallback to `main`/`master` or first branch).

**Files:**
- [`src-tauri/src/commands/project_commands.rs`](/Users/lazabogdan/Code/ralphx/src-tauri/src/commands/project_commands.rs)

**Implementation:**
- Use `git symbolic-ref refs/remotes/origin/HEAD` or `git remote show origin`
- Fallback chain: origin/HEAD → `main` → `master` → first branch

---

### Task 2: Expose default-branch command in frontend API
**Dependencies:** Task 1
**Atomic Commit:** `feat(api): add getGitDefaultBranch to projectsApi`

Expose the new command in the frontend API (`projectsApi`) alongside `getGitBranches` so UI can request the default branch per working directory.

**Files:**
- [`src/api/projects.ts`](/Users/lazabogdan/Code/ralphx/src/api/projects.ts)
- `src/api/projects.schemas.ts` (if schema needed)
- `src/api/projects.types.ts` (if type needed)

---

### Task 3: Make base branch + worktree dir editable in settings UI
**Dependencies:** Task 2
**Atomic Commit:** `feat(settings): add editable base branch and worktree directory`

Update `GitSettingsSection` to make base branch editable (dropdown or input) and persist changes via `projectsApi.update` (calling `update_project`) while keeping changes scoped to the active project. Include a "detect default" action that fills base branch using the new command when unset or on demand.

Update the worktree settings section so `worktreeParentDirectory` updates are persisted per project (currently only kept in local state); wire it to `projectsApi.update` and local store updates.

**Files:**
- [`src/components/settings/GitSettingsSection.tsx`](/Users/lazabogdan/Code/ralphx/src/components/settings/GitSettingsSection.tsx)

---

### Task 4: Use default-branch detection in creation wizard
**Dependencies:** Task 2
**Atomic Commit:** `feat(wizard): use default-branch detection in project creation`

Update `ProjectCreationWizard` to use the default-branch detection (new command) when a working directory is selected, before falling back to `main`/`master`, and ensure the selected base branch is included on submit.

**Files:**
- [`src/components/projects/ProjectCreationWizard/ProjectCreationWizard.tsx`](/Users/lazabogdan/Code/ralphx/src/components/projects/ProjectCreationWizard/ProjectCreationWizard.tsx)

---

### Task 5: Add tests for default-branch detection
**Dependencies:** Task 1, Task 2
**Atomic Commit:** `test(projects): add tests for default-branch detection`

Add/adjust tests as needed for new command parsing and any UI logic that depends on default-branch detection (light unit tests or existing test patterns).

**Files:**
- `src-tauri/src/commands/project_commands_tests.rs` (or similar)
- Frontend test files as applicable

## Notes on UI behavior

- Scope remains "active project only" in Settings.
- Default-branch detection should use git (e.g., `git symbolic-ref refs/remotes/origin/HEAD` or `git remote show origin`) and fall back to `main`/`master` when no origin exists or resolution fails.

## Task Dependency Graph

```
Task 1 (Backend) ──┬──→ Task 2 (API) ──┬──→ Task 3 (Settings UI)
                   │                   └──→ Task 4 (Creation Wizard)
                   └──→ Task 5 (Tests)
```

**Notes:**
- Tasks 3 and 4 can run in parallel after Task 2 completes (both depend only on API layer)
- Task 5 can begin after Tasks 1 and 2 complete
- All tasks are additive (no renames/removes), so each forms a valid compilation unit

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)