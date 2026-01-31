# Plan: Blocked Reason Feature

## Summary
Add a `blocked_reason` field to tasks and display blocked tasks in the Ready column.

## Problem
1. Blocked tasks disappear from Kanban (no column maps to `blocked` status)
2. No way to record WHY a task is blocked

## Solution
1. Add `blocked` as a group in the Ready column
2. Add `blocked_reason: Option<String>` field to tasks
3. Show dialog when blocking to capture reason
4. Display reason on task card (truncated + tooltip)

---

## Implementation Tasks

### Task 1: Database Migration (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(backend): add blocked_reason column migration`

**File:** `src-tauri/src/infrastructure/sqlite/migrations/v4_add_blocked_reason.rs`

```rust
pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "tasks", "blocked_reason", "TEXT DEFAULT NULL")?;
    Ok(())
}
```

**Also update `mod.rs`:**
- Add `mod v4_add_blocked_reason;`
- Add to MIGRATIONS array
- Bump `SCHEMA_VERSION` to 4

### Task 2: Backend Entity + Repository (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(backend): add blocked_reason to task entity and repository`

**Files:**
- `src-tauri/src/domain/entities/task.rs` — add `blocked_reason: Option<String>`
- `src-tauri/src/infrastructure/sqlite/sqlite_task_repo.rs` — update SELECT/INSERT/UPDATE

### Task 3: New Commands (block_task, unblock_task) (BLOCKING)
**Dependencies:** Task 2
**Atomic Commit:** `feat(backend): add block_task and unblock_task commands`

**File:** `src-tauri/src/commands/task_commands/mutation.rs`

```rust
#[tauri::command]
pub async fn block_task(task_id: String, reason: Option<String>, ...) -> Result<TaskResponse, String>

#[tauri::command]
pub async fn unblock_task(task_id: String, ...) -> Result<TaskResponse, String>
```

- `block_task`: transitions to Blocked + sets blocked_reason
- `unblock_task`: transitions to Ready + clears blocked_reason

**Also update:**
- `types.rs` — add `blocked_reason` to TaskResponse
- `lib.rs` — register new commands

### Task 4: Frontend Types (BLOCKING)
**Dependencies:** Task 3
**Atomic Commit:** `feat(frontend): add blockedReason to task types`

**File:** `src/types/task.ts`

```typescript
// Schema (snake_case from backend)
blocked_reason: z.string().nullable(),

// Transform to camelCase
blockedReason: raw.blocked_reason,
```

### Task 5: API Wrappers (BLOCKING)
**Dependencies:** Task 4
**Atomic Commit:** `feat(frontend): add blockTask and unblockTask API functions`

**File:** `src/api/tasks.ts`

```typescript
export async function blockTask(taskId: string, reason?: string): Promise<Task>
export async function unblockTask(taskId: string): Promise<Task>
```

### Task 6: Add Blocked Group to Workflow
**Dependencies:** None (can run in parallel with Tasks 1-5)
**Atomic Commit:** `feat(frontend): add blocked group to Ready column workflow`

**File:** `src/types/workflow.ts`

In `defaultWorkflow.columns[1].groups` (Ready column), add:

```typescript
{
  id: "blocked",
  label: "Blocked",
  statuses: ["blocked"],
  icon: "Ban",
  accentColor: "hsl(var(--warning))",
  canDragFrom: true,
  canDropTo: true,
},
```

### Task 7: BlockReasonDialog Component
**Dependencies:** None (can run in parallel with Tasks 1-6)
**Atomic Commit:** `feat(frontend): add BlockReasonDialog component`

**File:** `src/components/tasks/BlockReasonDialog.tsx`

Dialog with:
- Title: "Block Task"
- Textarea for optional reason
- Cancel / Block buttons

### Task 8: Update Context Menu
**Dependencies:** Task 5, Task 7
**Atomic Commit:** `feat(frontend): integrate BlockReasonDialog in context menu`

**File:** `src/components/tasks/TaskCardContextMenu.tsx`

- Change "Block" action to open BlockReasonDialog instead of simple confirm
- Add new prop `onBlockWithReason: (reason?: string) => void`

### Task 9: Display Blocked Reason on TaskCard
**Dependencies:** Task 4, Task 6
**Atomic Commit:** `feat(frontend): display blocked reason on task cards`

**File:** `src/components/tasks/TaskBoard/TaskCard.tsx`

When `task.internalStatus === "blocked" && task.blockedReason`:
- Show Ban icon + truncated reason text
- Full reason in tooltip on hover

### Task 10: Hook/Mutation Updates
**Dependencies:** Task 5
**Atomic Commit:** `feat(frontend): add blockTask and unblockTask mutations`

**File:** `src/hooks/useTaskMutation.ts` (or wherever mutations live)

Add `blockTask` and `unblockTask` mutations with query invalidation.

---

## Dependency Graph

```
Task 1 (Migration)
    ↓
Task 2 (Entity/Repo)
    ↓
Task 3 (Commands)
    ↓
Task 4 (Frontend Types)
    ↓
Task 5 (API Wrappers) ←── Task 6 (Workflow - parallel)
    ↓                      Task 7 (Dialog - parallel)
Task 8 (Context Menu) ←── Task 7
Task 9 (TaskCard) ←────── Task 4, Task 6
Task 10 (Mutations) ←──── Task 5
```

**Parallel execution opportunities:**
- Task 6, Task 7 can run independently from Tasks 1-5
- Task 8, Task 9, Task 10 can run in parallel after their dependencies complete

---

## Verification

1. **Block a Ready task** — confirm dialog appears, enter reason
2. **Check Ready column** — task appears in "Blocked" group
3. **Hover task card** — tooltip shows full reason
4. **Unblock task** — confirm it moves back to "Fresh Tasks" group
5. **Check database** — `blocked_reason` column populated/cleared correctly

```bash
# Backend
cargo clippy --all-targets --all-features -- -D warnings
cargo test

# Frontend
npm run lint && npm run typecheck
npm test
```

---

## Critical Files
- `src-tauri/src/infrastructure/sqlite/migrations/mod.rs`
- `src-tauri/src/domain/entities/task.rs`
- `src-tauri/src/commands/task_commands/mutation.rs`
- `src/types/task.ts`
- `src/types/workflow.ts`
- `src/components/tasks/TaskCardContextMenu.tsx`
- `src/components/tasks/TaskBoard/TaskCard.tsx`

---

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
