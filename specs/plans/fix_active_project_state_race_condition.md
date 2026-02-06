# Fix: ActiveProjectState Race Condition on Startup

## Context

When the app restarts, tasks stuck in `executing` state should be resumed via `StartupJobRunner`. However, the runner sees "No active project set" and skips resumption because:

1. `ActiveProjectState` is **in-memory only** (`RwLock<Option<ProjectId>>`) — starts as `None` every restart
2. Frontend has `activeProjectId` persisted in localStorage (Zustand persist), sends it to backend via `setActiveProject()` IPC on mount
3. `StartupJobRunner` runs after a 500ms delay — but the frontend IPC may arrive at 500-700ms
4. **Race condition**: runner checks before frontend IPC lands

## Fix: Add `tokio::sync::Notify` to `ActiveProjectState`

When `set()` is called with `Some(project_id)`, it calls `notify_waiters()`. The startup runner uses a new `wait_for_project()` method with a 5-second timeout instead of a single `get()` check. No DB changes, no frontend changes.

## Files to Modify (3)

### Task 1: Add `Notify` to `ActiveProjectState` (BLOCKING)
**Dependencies:** None
**Atomic Commit:** Part of single commit (see Task 3)

**File:** `src-tauri/src/commands/execution_commands.rs` (lines 54-77)

- Add `Notify` field to `ActiveProjectState`
- Remove `Default` derive (can't auto-derive with `Notify`)
- Add manual `Default` impl (preserves existing callers)
- In `set()`: call `self.notify.notify_waiters()` when project is `Some`
- Add `wait_for_project(timeout: Duration) -> Option<ProjectId>` method:
  - Register `notified` future FIRST (guarantees delivery from `notify_waiters()`)
  - Fast path: if already set, return immediately
  - Slow path: `tokio::time::timeout(timeout, notified).await`, then re-check

**Compilation unit note:** Additive — `new()`, `get()`, `set()` signatures unchanged. All 5 files importing `ActiveProjectState` continue to compile.

### Task 2: Wire `wait_for_project()` into `StartupJobRunner`
**Dependencies:** Task 1 (uses `wait_for_project()` added in Task 1)
**Atomic Commit:** Part of single commit (see Task 3)

**File:** `src-tauri/src/application/startup_jobs.rs` (lines 42-57, 156-167)

- Add `active_project_wait_timeout: Duration` field to `StartupJobRunner` (default 5s)
- Add `with_active_project_timeout(Duration) -> Self` builder method (for tests)
- Replace `self.active_project_state.get().await` (line 157) with `self.active_project_state.wait_for_project(self.active_project_wait_timeout).await`
- Update log message to mention "after waiting"

### Task 3: Update tests + add new async wait test
**Dependencies:** Task 1, Task 2
**Atomic Commit:** `fix(startup): wait for active project before task resumption`

**File:** `src-tauri/src/application/startup_jobs/tests.rs`

- In `build_runner()`: add `.with_active_project_timeout(Duration::from_millis(10))` so tests that don't set active project don't wait 5 seconds
- Tests that DO set active project (e.g., `test_resumption_spawns_agents`) already set it before `run()`, so the fast path returns immediately
- Add new test `test_resumption_waits_for_active_project`: spawn a task that sets active project after 200ms delay, verify runner waits and successfully resumes

**Implementation note:** All 3 tasks form a single compilation unit and should be committed together. Tasks 1-2 are logical ordering only — they ship as one atomic commit.

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| Normal startup (IPC at ~600ms) | Fast path or <200ms Notify wait |
| Fresh install, no projects | Frontend sends `set_active_project(null)` → `set(None)` doesn't notify → 5s timeout → skip. Correct. |
| Frontend crashes before IPC | 5s timeout → skip. Acceptable. |
| Multiple `set()` calls | `notify_waiters()` is idempotent, only one waiter exists |

## Verification

1. `cargo test -p ralphx -- startup_jobs` — all existing tests pass, new test passes
2. `cargo clippy --all-targets --all-features -- -D warnings` — no warnings
3. Manual: `npm run tauri dev` → observe logs show "waiting for active project" then successful resumption (no more "No active project set, skipping")

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
- This plan is a **single commit** — all 3 files modified atomically as `fix(startup): wait for active project before task resumption`
