# RalphX - Phase 82: Project-Scoped Execution Control

## Overview

Scope execution controls, scheduler, and status to the active project, with per-project execution settings and a global concurrency cap shared across projects. This phase extends the Phase 80 stop/pause semantics with per-project scoping and builds on Phase 79's per-project settings patterns.

**Reference Plan:**
- `specs/plans/project_scoped_execution_control.md` - Detailed implementation plan with per-project execution state registry, backwards-compatible API changes, and global concurrency cap

## Goals

1. Introduce per-project execution state (paused flag, running count, max concurrent) keyed by `project_id`
2. Scope scheduler, pause/resume/stop, queue counts, recovery/resumption, and UI status/events to the active project
3. Add global max concurrent cap (default 20, UI max 50) that limits total concurrency across all projects
4. Expose global cap in Settings UI with clear copy; keep per-project max in project settings

## Dependencies

### Phase 80 (Execution Stop/Pause Semantics) - Required

| Dependency | Why Needed |
|------------|------------|
| Paused/Stopped statuses | Per-project scoping extends these new statuses |
| Updated stop/pause flows | This phase scopes them to active project |

### Phase 79 (Git Settings Per-Project) - Required

| Dependency | Why Needed |
|------------|------------|
| Per-project settings patterns | Reuse update patterns for execution settings |
| Settings UI infrastructure | Extend for execution settings per project |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/project_scoped_execution_control.md`
2. Understand the backwards compatibility requirement (optional project_id parameters)
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
2. **Read the ENTIRE implementation plan** at `specs/plans/project_scoped_execution_control.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add per-project execution state registry and active project context",
    "plan_section": "1) Backend: per-project execution state + active project context",
    "blocking": [2, 3],
    "blockedBy": [],
    "atomic_commit": "feat(execution): add per-project execution state registry and active project context",
    "steps": [
      "Read specs/plans/project_scoped_execution_control.md section '1) Backend: per-project execution state + active project context'",
      "Add ExecutionStateRegistry keyed by ProjectId replacing global ExecutionState in lib.rs",
      "Add ActiveProjectState { current: Option<ProjectId> } in AppState",
      "Update execution commands with OPTIONAL project_id parameter (backwards compatible):",
      "  - get_execution_status(project_id: Option<String>) - if None, use active project or aggregate",
      "  - pause_execution(project_id: Option<String>)",
      "  - resume_execution(project_id: Option<String>)",
      "  - stop_execution(project_id: Option<String>)",
      "Update event payloads to include projectId (execution:status_changed, execution:queue_changed)",
      "Adjust TaskSchedulerService to accept optional project_id and query only that project",
      "Update startup resumption to respect active project (scope to active, skip if none set)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(execution): add per-project execution state registry and active project context"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add per-project execution settings and global concurrency cap",
    "plan_section": "2) Backend: per-project execution settings + global cap",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(execution): add per-project execution settings and global concurrency cap",
    "steps": [
      "Read specs/plans/project_scoped_execution_control.md section '2) Backend: per-project execution settings + global cap'",
      "Add execution_settings table with project_id foreign key (or extend projects table)",
      "Add global execution settings record (single-row table) for global_max_concurrent (default 20, max 50)",
      "Create ExecutionSettingsRepository trait and SqliteExecutionSettingsRepo implementation",
      "Add commands with OPTIONAL project_id (backwards compatible):",
      "  - get_execution_settings(project_id: Option<String>) - if None, returns global defaults",
      "  - update_execution_settings(project_id: Option<String>, ...) - if None, updates global",
      "Enforce global cap: scheduler must not exceed global_max_concurrent across all projects",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(execution): add per-project execution settings and global concurrency cap"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Add per-project execution status store and API integration",
    "plan_section": "3) Frontend: per-project status store + API changes",
    "blocking": [4],
    "blockedBy": [1, 2],
    "atomic_commit": "feat(execution): add per-project execution status and API integration",
    "steps": [
      "Read specs/plans/project_scoped_execution_control.md section '3) Frontend: per-project status store + API changes'",
      "Update execution API wrappers (src/api/execution.ts) to pass projectId and reflect new schemas",
      "Change useExecutionStatus to accept projectId and store status per project in uiStore (executionStatusByProject)",
      "Update useExecutionEvents to read projectId from payload and update only that project's status",
      "Update App.tsx to:",
      "  - Pass currentProjectId to execution API calls",
      "  - Send set_active_project command on project selection",
      "  - Render ExecutionControlBar with active project's status",
      "Update SettingsView wiring to load/save execution settings per project, plus global cap setting",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(execution): add per-project execution status and API integration"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Add tests for per-project execution scoping",
    "plan_section": "4) Tests + verification (TDD-first)",
    "blocking": [],
    "blockedBy": [1, 2, 3],
    "atomic_commit": "test(execution): add per-project execution scoping tests",
    "steps": [
      "Read specs/plans/project_scoped_execution_control.md section '4) Tests + verification'",
      "Backend tests for per-project scoping:",
      "  - get_execution_status counts only Ready tasks in the specified project",
      "  - pause/resume/stop only affect agent-active tasks in the specified project",
      "  - Scheduler only transitions Ready tasks in the active project",
      "  - Event payloads include projectId",
      "Frontend tests (if applicable):",
      "  - useExecutionEvents updates only matching project status",
      "  - useExecutionControl uses query keys per project",
      "Run cargo test && npm run test",
      "Commit: test(execution): add per-project execution scoping tests"
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
| **Optional project_id parameters** | Maintains backwards compatibility - frontend can call without project_id until Task 3 wires it up |
| **Per-project execution state registry** | Keyed by ProjectId, replaces global singleton for proper multi-project isolation |
| **Global cap separate from per-project max** | Global cap (20 default, 50 max) prevents system overload; per-project max controls individual project limits |
| **Active project state in AppState** | Frontend notifies backend on project switch; commands use active project when project_id not specified |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] ExecutionStateRegistry returns correct per-project status
- [ ] pause/resume/stop only affect specified project's tasks
- [ ] Scheduler respects active project filter
- [ ] Global cap enforced across all projects
- [ ] Event payloads include projectId

### Frontend - Run `npm run test`
- [ ] useExecutionEvents updates correct project status
- [ ] useExecutionControl passes projectId to API calls
- [ ] SettingsView displays per-project and global settings

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Switch projects and verify execution status updates correctly
- [ ] Pause execution in one project, verify other projects unaffected
- [ ] Global cap limits total running tasks across all projects
- [ ] Settings UI shows per-project max and global cap with clear labels

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] set_active_project command called on project switch
- [ ] ExecutionControlBar receives active project's status
- [ ] pause/resume/stop buttons pass current project_id
- [ ] Settings changes persist per-project and globally

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
