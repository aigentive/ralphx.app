# RalphX - Phase 104: Reopen & Reset Ideation Sessions

## Overview

Accepted/archived ideation sessions are permanently read-only with no way to undo. Users need the ability to **reopen** a session back to Active (to continue editing proposals) and **reset & re-accept** a session (delete all tasks, clean up git resources, and re-apply proposals as fresh tasks). Both operations are destructive — they delete tasks, git branches, and worktrees — and require confirmation dialogs.

The approach uses a single backend command `reopen_ideation_session` that handles all cleanup. The two user-facing actions differ only in what the frontend does after: "Reopen" stops at the Active state, while "Reset & Re-accept" chains `reopen` → `apply_proposals_to_kanban` to create fresh tasks.

**Reference Plan:**
- `specs/plans/reopen_and_reset_ideation_sessions.md` - Full implementation plan with cleanup logic, edge cases, and compilation unit analysis

## Goals

1. Allow users to reopen accepted/archived sessions back to Active status
2. Allow users to reset & re-accept sessions (delete old tasks, re-apply proposals as fresh)
3. Clean up all git resources (branches, worktrees) and stop running agents during reopen
4. Provide confirmation dialogs with destructive action warnings

## Dependencies

### Phase 103 (Cascade Delete Tasks) - Partial Overlap

| Dependency | Why Needed |
|------------|------------|
| `get_by_ideation_session` on TaskRepository | Phase 103 Task 1 adds this — reopen service reuses it |
| Cascade delete pattern | Phase 103 establishes the force-stop + delete pattern; reopen follows same approach |

**Note:** Phase 103's `get_by_ideation_session` method (Task 1, already passing) is reused. The reopen service follows the same agent stop + task delete + git cleanup pattern but adds proposal cleanup and session status change.

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/reopen_and_reset_ideation_sessions.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/reopen_and_reset_ideation_sessions.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add new repository methods: clear_created_task_ids_by_session on TaskProposalRepository + all impls, fix update_status to clear archived_at/converted_at when setting Active",
    "plan_section": "Task 1: Backend — New Repository Methods",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(repos): add session reopen repository methods",
    "steps": [
      "Read specs/plans/reopen_and_reset_ideation_sessions.md section 'Task 1'",
      "Add clear_created_task_ids_by_session(&self, session_id: &IdeationSessionId) -> AppResult<()> to TaskProposalRepository trait",
      "Implement in SqliteTaskProposalRepository: UPDATE task_proposals SET created_task_id = NULL WHERE session_id = ?",
      "Implement in MemoryTaskProposalRepository: iterate and clear created_task_id for matching session_id",
      "Add mock stub in MockTaskProposalRepository (no-op Ok(()))",
      "Fix sqlite_ideation_session_repo.rs update_status: when setting Active, also clear archived_at and converted_at",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(repos): add session reopen repository methods"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Create SessionReopenService and reopen_ideation_session Tauri command with full cleanup: stop agents, delete tasks, clean git, clear proposals, reset session status",
    "plan_section": "Task 2: Backend — reopen_ideation_session Command",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(ideation): add reopen_ideation_session command and service",
    "steps": [
      "Read specs/plans/reopen_and_reset_ideation_sessions.md section 'Task 2'",
      "Create src-tauri/src/application/session_reopen_service.rs (~150 LOC) with reopen() method",
      "Service logic: validate session Accepted/Archived, get tasks via task_repo.get_by_ideation_session, stop running agents, abort rebase, checkout base branch, delete worktrees/branches/tasks, clean plan branch, clear proposal task IDs, set session Active, emit events",
      "Add pub mod session_reopen_service to src-tauri/src/application/mod.rs",
      "Add reopen_ideation_session command to ideation_commands_session.rs (~40 LOC): instantiate service, delegate",
      "Re-export in ideation_commands/mod.rs (already uses wildcard pub use)",
      "Register command in src-tauri/src/lib.rs invoke_handler",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(ideation): add reopen_ideation_session command and service"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Add reopen API wrapper to ideation.ts and mutation hooks (useReopenSession, useResetAndReaccept) to useIdeation.ts",
    "plan_section": "Task 3: Frontend — API + Hooks",
    "blocking": [4],
    "blockedBy": [2],
    "atomic_commit": "feat(ideation): add reopen session API wrapper and mutation hooks",
    "steps": [
      "Read specs/plans/reopen_and_reset_ideation_sessions.md section 'Task 3'",
      "Add sessions.reopen(sessionId) to ideationApi.sessions in src/api/ideation.ts: invoke('reopen_ideation_session', { id: sessionId })",
      "Add useReopenSession() mutation hook to src/hooks/useIdeation.ts: calls ideationApi.sessions.reopen, invalidates ideationKeys.sessions() and task query keys",
      "Add useResetAndReaccept() mutation hook: chains reopen → apply_proposals_to_kanban with all proposal IDs (follow existing useArchiveIdeationSession pattern for invalidation)",
      "Run npm run typecheck && npm run lint",
      "Commit: feat(ideation): add reopen session API wrapper and mutation hooks"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Create ReopenSessionDialog component with two modes (reopen/reset) using existing AlertDialog shadcn pattern",
    "plan_section": "Task 4: Frontend — Confirmation Dialog",
    "blocking": [5, 6],
    "blockedBy": [3],
    "atomic_commit": "feat(ideation): add reopen session confirmation dialog",
    "steps": [
      "Read specs/plans/reopen_and_reset_ideation_sessions.md section 'Task 4'",
      "Create src/components/Ideation/ReopenSessionDialog.tsx (~120 LOC)",
      "Props: open, onOpenChange, mode ('reopen' | 'reset'), sessionTitle, taskCount, onConfirm, isLoading",
      "Reopen mode: title 'Reopen Session', warn about task deletion, confirm 'Reopen'",
      "Reset mode: title 'Reset & Re-accept', warn about task deletion + re-apply, confirm 'Reset & Re-accept'",
      "Design: warm orange accent #ff6b35, destructive confirm button, SF Pro font",
      "Run npm run typecheck && npm run lint",
      "Commit: feat(ideation): add reopen session confirmation dialog"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Add Reopen/Reset context menu items to PlanBrowser history section with onReopenPlan and onResetReacceptPlan callback props",
    "plan_section": "Task 5: Frontend — PlanBrowser History Context Menu",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(ideation): add reopen/reset context menu to plan browser history",
    "steps": [
      "Read specs/plans/reopen_and_reset_ideation_sessions.md section 'Task 5'",
      "Add optional props to PlanItemProps: onReopenPlan?: (planId: string) => void, onResetReacceptPlan?: (planId: string) => void",
      "Add optional props to PlanBrowserProps: onReopenPlan?, onResetReacceptPlan?",
      "Add second menu block for history items: {!isEditing && isHistory && ( <DropdownMenu> ... )}",
      "Menu items: Reopen, Reset & Re-accept, separator, Delete (existing)",
      "Wire callbacks through from PlanBrowser to PlanItem",
      "Run npm run typecheck && npm run lint",
      "Commit: feat(ideation): add reopen/reset context menu to plan browser history"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Add Reopen/Reset action buttons to PlanningView header for accepted/archived read-only sessions",
    "plan_section": "Task 6: Frontend — PlanningView Header Actions",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(ideation): add reopen/reset header actions to planning view",
    "steps": [
      "Read specs/plans/reopen_and_reset_ideation_sessions.md section 'Task 6'",
      "In PlanningView.tsx, add state for dialog: reopenDialogOpen, reopenDialogMode",
      "When isReadOnly && (session.status === 'accepted' || session.status === 'archived'), render action buttons next to status badge",
      "Reopen button (subtle, secondary style) for both accepted and archived",
      "Reset & Re-accept button (for accepted sessions only — they have proposals to re-apply)",
      "Both buttons trigger ReopenSessionDialog with appropriate mode",
      "Wire dialog onConfirm to useReopenSession or useResetAndReaccept hooks",
      "Pass onReopenPlan and onResetReacceptPlan callbacks to PlanBrowser",
      "Run npm run typecheck && npm run lint",
      "Commit: feat(ideation): add reopen/reset header actions to planning view"
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
| **Single backend command + frontend orchestration** | Avoids duplicating apply_proposals_to_kanban logic. "Reset & Re-accept" = reopen + apply (existing command). |
| **Bypass TransitionHandler for cleanup** | Transient states (qa_passed, pending_review, etc.) have no valid transition to stopped/cancelled. Direct DB delete avoids spawning unwanted agents. |
| **Reuse Phase 103's get_by_ideation_session** | Already implemented and tested. SessionReopenService calls the same repo method. |
| **New service file (not inline in command)** | ~150 LOC cleanup logic is too complex for a thin command handler. Service is testable and follows existing patterns. |
| **Optional props for PlanBrowser** | Adding `onReopenPlan?` and `onResetReacceptPlan?` as optional avoids breaking existing callers. |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] `clear_created_task_ids_by_session` clears all proposal task links for a session
- [ ] `update_status(Active)` clears `archived_at` and `converted_at`
- [ ] `reopen_ideation_session` validates session is Accepted or Archived
- [ ] Reopen stops running agents, deletes tasks, cleans git, clears proposals, resets status

### Frontend - Run `npm run test`
- [ ] `useReopenSession` mutation invalidates correct query keys
- [ ] `useResetAndReaccept` chains reopen → apply
- [ ] ReopenSessionDialog renders correct content for both modes

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Accept a plan → verify session appears in history section
- [ ] History context menu shows "Reopen" and "Reset & Re-accept" items
- [ ] PlanningView header shows Reopen/Reset buttons for accepted sessions
- [ ] Reopen → confirm → session becomes Active, tasks deleted, proposals editable
- [ ] Reset & Re-accept → confirm → session reopened then re-accepted with fresh tasks
- [ ] Running agents are stopped during reopen (no orphan processes)
- [ ] Git branches and worktrees cleaned up after reopen

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] History context menu → PlanBrowser callback → PlanningView handler → dialog → mutation → backend
- [ ] Header button → dialog → mutation → backend
- [ ] `reopen_ideation_session` command registered in `lib.rs` invoke_handler
- [ ] `sessions.reopen()` in ideation API calls the correct Tauri command
- [ ] Events `ideation:session_reopened` and `task:list_changed` emitted and consumed

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
