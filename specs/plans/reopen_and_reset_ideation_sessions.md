# Plan: Reopen & Reset Ideation Sessions

## Context

Accepted/archived ideation sessions are permanently read-only with no way to undo. Users need to:
1. **Reopen** a session back to Active to continue editing proposals
2. **Reset & Re-accept** a session (delete all tasks, clean up, re-apply proposals as fresh tasks)

Both operations are destructive (delete tasks, git branches, worktrees) and require confirmation.

## Approach: Single Backend Command + Frontend Orchestration

One new Tauri command `reopen_ideation_session` handles all cleanup. The two user-facing actions differ only in what the frontend does after:
- **Reopen**: Call `reopen` -> session becomes Active -> user edits freely
- **Reset & Re-accept**: Call `reopen` -> immediately call existing `apply_proposals_to_kanban` with all proposal IDs -> fresh tasks created

This avoids duplicating the complex apply logic.

## Key Design Decision: Bypass TransitionHandler for Cleanup

The 24-state machine has transient states (`qa_passed`, `pending_review`, `approved`, etc.) with no valid transition to `stopped` or `cancelled`. Running TransitionHandler would trigger unwanted side effects (spawning agents). Instead:
1. Stop running agents directly via `RunningAgentRegistry`
2. Delete tasks directly from DB
3. Clean up git resources via `GitService`

---

## Implementation

### Task 1: Backend — New Repository Methods (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(repos): add session reopen repository methods`

**Files to modify:**

| File | Change |
|------|--------|
| `src-tauri/src/domain/repositories/task_proposal_repository.rs` | Add `clear_created_task_ids_by_session(&self, session_id) -> AppResult<()>` to trait |
| `src-tauri/src/domain/repositories/task_repository.rs` | Add `get_by_ideation_session_id(&self, session_id) -> AppResult<Vec<Task>>` to trait |
| SQLite impl for task_proposal_repo | `UPDATE task_proposals SET created_task_id = NULL WHERE session_id = ?` |
| SQLite impl for task_repo | `SELECT * FROM tasks WHERE ideation_session_id = ?` |
| Memory impls for both repos | In-memory equivalents |
| Mock impls in both trait test modules | No-op stubs |
| `sqlite_ideation_session_repo.rs` | Fix `update_status` to clear `archived_at`/`converted_at` when setting Active |

**Compilation unit note:** Trait method additions require ALL implementors (SQLite, Memory, Mock) in the same commit — missing any impl breaks `cargo check`.

### Task 2: Backend — `reopen_ideation_session` Command (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(ideation): add reopen_ideation_session command and service`

**New file:** `src-tauri/src/application/session_reopen_service.rs` (~150 LOC)

Core cleanup logic:
```
1. Validate session is Accepted or Archived
2. Get all tasks via task_repo.get_by_ideation_session_id()
3. Stop running agents via RunningAgentRegistry (for executing/reviewing/merging tasks)
4. Abort any in-progress rebase (Local mode safety)
5. Checkout base branch (Local mode — ensure we're not on a branch about to be deleted)
6. For each task: delete worktree (if exists), delete task branch (if exists), delete task from DB
7. Get plan branch via plan_branch_repo.get_by_session_id() — if Active: delete git feature branch, mark Abandoned
8. Clear created_task_id on all proposals via clear_created_task_ids_by_session()
9. Set session status to Active (which clears converted_at/archived_at)
10. Emit events: ideation:session_reopened + task:list_changed
```

**Modify:** `src-tauri/src/commands/ideation_commands/ideation_commands_session.rs`
- Add `reopen_ideation_session` command (~40 LOC) that instantiates the service and delegates

**Modify:** `src-tauri/src/commands/ideation_commands/mod.rs` — re-export
**Modify:** `src-tauri/src/lib.rs` — register command
**Modify:** `src-tauri/src/application/mod.rs` — add module

### Task 3: Frontend — API + Hooks (BLOCKING)
**Dependencies:** Task 2
**Atomic Commit:** `feat(ideation): add reopen session API wrapper and mutation hooks`

**Modify:** `src/api/ideation.ts`
- Add `sessions.reopen(sessionId)` -> calls `reopen_ideation_session`

**Modify:** `src/hooks/useIdeation.ts` (or wherever mutation hooks live)
- Add `useReopenSession()` mutation that invalidates session + task queries
- Add `useResetAndReaccept()` that chains reopen -> apply_proposals_to_kanban

**Invalidation pattern note:** Follow existing `useDeleteIdeationSession` pattern — broad invalidation of `ideationKeys.sessions()` + task query keys since reopen deletes tasks.

### Task 4: Frontend — Confirmation Dialog
**Dependencies:** Task 3
**Atomic Commit:** `feat(ideation): add reopen session confirmation dialog`

**New file:** `src/components/Ideation/ReopenSessionDialog.tsx` (~120 LOC)

Uses existing AlertDialog pattern (shadcn). Two modes controlled by prop:
- **"reopen"**: Title "Reopen Session", warns about task deletion, confirm button "Reopen"
- **"reset"**: Title "Reset & Re-accept", warns about task deletion + immediate re-apply, confirm button "Reset & Re-accept"

Shows: session title, task count (from frontend task store filtered by `ideationSessionId`), warning about running agents being stopped.

Design: warm orange accent `#ff6b35`, destructive confirm button, SF Pro font.

### Task 5: Frontend — PlanBrowser History Context Menu
**Dependencies:** Task 4
**Atomic Commit:** `feat(ideation): add reopen/reset context menu to plan browser history`

**Modify:** `src/components/Ideation/PlanBrowser.tsx`

Currently line 228: `{!isEditing && !isHistory && (` blocks menu for history items.

Add a **second menu block** for history items:
```tsx
{!isEditing && isHistory && (
  <DropdownMenu>
    <DropdownMenuItem>Reopen</DropdownMenuItem>      {/* → opens dialog in "reopen" mode */}
    <DropdownMenuItem>Reset & Re-accept</DropdownMenuItem> {/* → opens dialog in "reset" mode */}
    <DropdownMenuSeparator />
    <DropdownMenuItem destructive>Delete</DropdownMenuItem>
  </DropdownMenu>
)}
```

**Props additions to `PlanBrowserProps` and `PlanItemProps`:**
- `onReopenPlan?: (planId: string) => void`
- `onResetReacceptPlan?: (planId: string) => void`

### Task 6: Frontend — PlanningView Header Actions
**Dependencies:** Task 4
**Atomic Commit:** `feat(ideation): add reopen/reset header actions to planning view`

**Modify:** `src/components/Ideation/PlanningView.tsx`

When `isReadOnly && (session.status === "accepted" || session.status === "archived")`, add action buttons in the header area next to the status badge:
- "Reopen" button (subtle, secondary style)
- "Reset & Re-accept" button (for accepted sessions only)

Both trigger the same confirmation dialog.

---

## File Change Summary

| File | Type | Est. LOC |
|------|------|----------|
| `src-tauri/src/domain/repositories/task_repository.rs` | Modify | +8 |
| `src-tauri/src/domain/repositories/task_proposal_repository.rs` | Modify | +5 |
| SQLite task repo impl | Modify | +15 |
| SQLite task proposal repo impl | Modify | +12 |
| Memory task repo impl | Modify | +10 |
| Memory task proposal repo impl | Modify | +8 |
| Mock stubs in both trait test modules | Modify | +8 |
| `sqlite_ideation_session_repo.rs` | Modify | +3 |
| `src-tauri/src/application/session_reopen_service.rs` | **New** | ~150 |
| `src-tauri/src/application/mod.rs` | Modify | +1 |
| `src-tauri/src/commands/ideation_commands/ideation_commands_session.rs` | Modify | +40 |
| `src-tauri/src/commands/ideation_commands/mod.rs` | Modify | +1 |
| `src-tauri/src/lib.rs` | Modify | +1 |
| `src/api/ideation.ts` | Modify | +10 |
| `src/hooks/useIdeation.ts` | Modify | +30 |
| `src/components/Ideation/ReopenSessionDialog.tsx` | **New** | ~120 |
| `src/components/Ideation/PlanBrowser.tsx` | Modify | +40 |
| `src/components/Ideation/PlanningView.tsx` | Modify | +25 |

**Total: ~487 LOC across 18 files (2 new, 16 modified)**

---

## Implementation Order (Compilation Units)

1. **Backend repos** (Task 1) — trait additions + all impls + session repo fix -> `cargo check`
2. **Backend service + command** (Task 2) — new service + command + registration -> `cargo check && cargo test`
3. **Frontend API + hooks** (Task 3) -> `npm run typecheck`
4. **Frontend UI** (Tasks 4-6) — dialog + PlanBrowser + PlanningView -> `npm run typecheck && npm run lint`

---

## Edge Cases

| Case | Handling |
|------|---------|
| Session already Active | Return error |
| No tasks exist (archived session without acceptance) | Skip cleanup, just update status |
| Running agents fail to stop | `RunningAgentRegistry::stop()` handles gracefully |
| Git branch already deleted | Ignore error, continue |
| Worktree missing | Ignore error, continue |
| Plan branch already Merged/Abandoned | Skip plan branch cleanup |
| Rebase in progress (Local mode) | Abort rebase before deleting branches |
| Reset re-accept fails after reopen | Session is Active, user can manually re-accept |

## Verification

1. **Backend**: `cargo clippy --all-targets --all-features -- -D warnings && cargo test`
2. **Frontend**: `npm run typecheck && npm run lint`
3. **Manual test**: Accept a plan -> verify history context menu appears -> Reopen -> verify session is Active and tasks deleted -> Re-accept -> verify fresh tasks created

---

## Dependency Graph

```
Task 1 (Backend repos) ──→ Task 2 (Service + command) ──→ Task 3 (API + hooks) ──→ Task 4 (Dialog)
                                                                                  ├─→ Task 5 (PlanBrowser)
                                                                                  └─→ Task 6 (PlanningView)
```

Tasks 5 and 6 are independent of each other but both depend on Task 4 (the dialog component they open).

## Compilation Unit Validation

| Task | Compiles Alone? | Reason |
|------|-----------------|--------|
| 1 | ✅ | Trait methods + ALL impls (SQLite, Memory, Mock) in same commit |
| 2 | ✅ | New service + command + registration — all additive, uses Task 1 methods |
| 3 | ✅ | New API wrapper + hooks — additive to existing `ideationApi` object |
| 4 | ✅ | New file — uses hooks from Task 3, no existing code modified |
| 5 | ✅ | Additive prop additions (optional `?` props) — no breaking changes |
| 6 | ✅ | Additive button additions — no breaking changes |

**No chicken-egg problems detected.** All changes are additive (new methods, new props with `?`, new files). No renames or signature changes.

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
