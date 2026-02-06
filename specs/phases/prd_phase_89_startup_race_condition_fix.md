# RalphX - Phase 89: Fix ActiveProjectState Race Condition on Startup

## Overview

When the app restarts, tasks stuck in `executing` state should be resumed via `StartupJobRunner`. However, the runner sees "No active project set" and skips resumption because `ActiveProjectState` is in-memory only (starts as `None` every restart), and the frontend IPC that sets the active project races with the startup runner's 500ms delay.

This phase adds a `tokio::sync::Notify`-based wait mechanism to `ActiveProjectState` so the startup runner waits (up to 5 seconds) for the frontend to set the active project before proceeding, eliminating the race condition.

**Reference Plan:**
- `specs/plans/fix_active_project_state_race_condition.md` - Detailed implementation plan with Notify-based wait mechanism

## Goals

1. Eliminate the race condition between `StartupJobRunner` and frontend `setActiveProject()` IPC
2. Add `wait_for_project()` method with configurable timeout to `ActiveProjectState`
3. Ensure all existing tests pass with minimal timeout for fast execution

## Dependencies

### Phase 82 (Project-Scoped Execution Control) - Required

| Dependency | Why Needed |
|------------|------------|
| `ActiveProjectState` | Introduced in Phase 82, this phase extends it with Notify |
| Per-project startup scoping | The race condition exists because of Phase 82's project-scoped design |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_active_project_state_race_condition.md`
2. Understand the Notify pattern and fast-path/slow-path design
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
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_active_project_state_race_condition.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

**NOTE:** All 3 tasks form a single compilation unit and should be committed as one atomic commit: `fix(startup): wait for active project before task resumption`

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add tokio::sync::Notify to ActiveProjectState with wait_for_project() method",
    "plan_section": "Task 1: Add Notify to ActiveProjectState",
    "blocking": [2, 3],
    "blockedBy": [],
    "atomic_commit": "fix(startup): wait for active project before task resumption",
    "steps": [
      "Read specs/plans/fix_active_project_state_race_condition.md section 'Task 1'",
      "Add `notify: Notify` field to `ActiveProjectState` struct in execution_commands.rs",
      "Remove `Default` derive, add manual `Default` impl with `Notify::new()`",
      "Update `new()` to initialize `notify: Notify::new()`",
      "In `set()`: call `self.notify.notify_waiters()` when project_id is `Some`",
      "Add `wait_for_project(timeout: Duration) -> Option<ProjectId>` method: register notified future FIRST, fast-path check, slow-path timeout wait",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Do NOT commit yet — continues in Task 2"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Wire wait_for_project() into StartupJobRunner with configurable timeout",
    "plan_section": "Task 2: Wire wait_for_project() into StartupJobRunner",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "fix(startup): wait for active project before task resumption",
    "steps": [
      "Read specs/plans/fix_active_project_state_race_condition.md section 'Task 2'",
      "Add `active_project_wait_timeout: Duration` field to `StartupJobRunner` struct (default 5s in new())",
      "Add `with_active_project_timeout(Duration) -> Self` builder method",
      "Replace `self.active_project_state.get().await` (line 157) with `self.active_project_state.wait_for_project(self.active_project_wait_timeout).await`",
      "Update log message to mention 'after waiting'",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Do NOT commit yet — continues in Task 3"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Update existing tests with short timeout + add new async wait test",
    "plan_section": "Task 3: Update tests + add new async wait test",
    "blocking": [],
    "blockedBy": [1, 2],
    "atomic_commit": "fix(startup): wait for active project before task resumption",
    "steps": [
      "Read specs/plans/fix_active_project_state_race_condition.md section 'Task 3'",
      "In build_runner(): chain `.with_active_project_timeout(Duration::from_millis(10))` so non-project tests don't wait 5s",
      "Verify tests that set active_project before run() still pass via fast path",
      "Add new test `test_resumption_waits_for_active_project`: spawn tokio task that sets active project after 200ms, verify runner waits and resumes",
      "Run cargo test -p ralphx -- startup_jobs",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit all 3 files atomically: fix(startup): wait for active project before task resumption"
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
| **`tokio::sync::Notify` over channel** | Notify is lightweight, no allocation per signal, supports multiple waiters, and `notify_waiters()` is idempotent |
| **Register `notified()` before fast-path check** | Prevents TOCTOU race: if `set()` fires between check and await, the notification is already captured |
| **5-second default timeout** | Generous enough for slow frontend boot, short enough to not block startup indefinitely |
| **Configurable timeout via builder** | Tests use 10ms timeout to avoid 5s waits when active project is intentionally unset |
| **No DB/frontend changes** | Pure backend fix — minimal blast radius, no migration needed |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] All existing startup_jobs tests pass unchanged (with 10ms timeout)
- [ ] New `test_resumption_waits_for_active_project` passes (async 200ms delay scenario)
- [ ] `ActiveProjectState::wait_for_project()` returns immediately when already set (fast path)
- [ ] `ActiveProjectState::wait_for_project()` returns `None` after timeout when never set

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes

### Manual Testing
- [ ] `npm run tauri dev` → logs show "waiting for active project" then successful resumption
- [ ] No more "No active project set, skipping task resumption" on normal startup
- [ ] Fresh install (no projects): graceful 5s timeout, then skip (acceptable)

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `ActiveProjectState::set()` calls `notify_waiters()` when `Some`
- [ ] `StartupJobRunner::run()` calls `wait_for_project()` instead of `get()`
- [ ] `build_runner()` in tests chains `.with_active_project_timeout()`

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No functions exported but never called
- [ ] `wait_for_project()` is actually called from `run()` (not dead code)

See `.claude/rules/gap-verification.md` for full verification workflow.
