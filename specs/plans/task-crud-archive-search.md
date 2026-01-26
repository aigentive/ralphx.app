# Task CRUD, Archive & Search Plan

**Created**: 2026-01-26
**Status**: Planning
**Related Files**:
- `src/components/tasks/TaskBoard/`
- `src/components/tasks/TaskDetailModal.tsx`
- `src/components/tasks/TaskCreationForm.tsx`
- `src-tauri/src/domain/entities/task.rs`
- `src-tauri/src/domain/entities/status.rs`
- `src-tauri/src/domain/entities/workflow.rs`

---

## Overview

This plan covers:
1. **Task Creation UI** - Inline quick-add on Kanban board
2. **Task Editing** - Edit mode in TaskDetailModal
3. **Archive System** - Soft delete with restore capability
4. **Search** - Cmd+F search with column filtering
5. **Infinite Scroll** - Pagination per column

---

## Current State Analysis

### What Exists

| Layer | Create | Update | Delete |
|-------|--------|--------|--------|
| Backend (Tauri) | `create_task` | `update_task` | `delete_task` |
| API bindings | `api.tasks.create()` | `api.tasks.update()` | `api.tasks.delete()` |
| Mutations | `createMutation` | `updateMutation` | `deleteMutation` |
| Store | `addTask()` | `updateTask()` | `removeTask()` |
| **UI Component** | `TaskCreationForm.tsx` | **MISSING** | **No button** |

### What's Missing

- No entry point to trigger task creation from Kanban board
- No edit mode in TaskDetailModal (read-only)
- No archive/soft-delete model
- No search functionality
- No pagination/infinite scroll

---

## Status System Deep Dive

### Internal Status (14 States)

| State | Type | Description |
|-------|------|-------------|
| `backlog` | Idle | Parked, not ready for work |
| `ready` | Idle | Ready to be picked up by scheduler |
| `blocked` | Idle | Waiting on dependencies or human input |
| `executing` | Active | Worker agent is running |
| `execution_done` | Active | Agent finished, routing to QA or review |
| `qa_refining` | Active | QA refining test criteria |
| `qa_testing` | Active | QA tests executing |
| `qa_passed` | Active | QA passed, going to review |
| `qa_failed` | Active | QA failed, needs revision |
| `pending_review` | Active | Awaiting AI/human review |
| `revision_needed` | Active | Reviewer requested changes |
| `approved` | Terminal | Complete and verified |
| `failed` | Terminal | Permanently failed |
| `cancelled` | Terminal | User cancelled |

### Valid Transitions (State Machine)

```
Backlog ────────► Ready ◄──────── Blocked
   │                │                 ▲
   │ Cancel    Block│                 │
   ▼                ▼                 │
Cancelled      Blocked ───────────────┘
                    │ Unblock
                    ▼
               Ready ◄─────────────────────────────┐
                    │                              │
              (System)                             │ Retry/Re-open
                    ▼                              │
               Executing ─────► Failed ────────────┤
                    │                              │
                    ▼                              │
             ExecutionDone                         │
                    │                              │
           ┌───────┴───────┐                       │
           ▼               ▼                       │
      QaRefining    PendingReview                  │
           │               │                       │
           ▼               │                       │
      QaTesting            │                       │
           │               │                       │
      ┌────┴────┐          │                       │
      ▼         ▼          │                       │
  QaPassed   QaFailed      │                       │
      │         │          │                       │
      ▼         ▼          │                       │
PendingReview◄──RevisionNeeded                     │
      │                    │                       │
      ├──────► Approved ───┴───────────────────────┘
      │
      └──────► RevisionNeeded ──► Executing
```

### Workflow Columns (Default RalphX)

| Column ID | Display Name | maps_to | Can Add Task? |
|-----------|--------------|---------|---------------|
| `draft` | Draft | Backlog | **YES** |
| `backlog` | Backlog | Backlog | **YES** |
| `todo` | To Do | Ready | No |
| `planned` | Planned | Ready | No |
| `in_progress` | In Progress | Executing | No (locked) |
| `in_review` | In Review | PendingReview | No (locked) |
| `done` | Done | Approved | No (locked) |

**Key Insight**: Multiple columns can map to the same internal status. The column is for UI organization; the internal status controls state machine behavior.

---

## Part 1: Task Creation (Inline Quick-Add)

### Design

Show ghost card **on column hover** in `draft` and `backlog` columns only:

```
┌─────────────────────────────────────┐
│ • Draft                         [2] │  ← Hover column
├─────────────────────────────────────┤
│  ┌─────────────────────────────┐    │
│  │ Existing task...            │    │
│  └─────────────────────────────┘    │
│                                     │
│  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┐    │  ← Appears on hover
│  │ + Add task                  │    │
│  └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┘    │
└─────────────────────────────────────┘
```

**Collapsed state** (ghost card):
- Dashed border: `2px dashed var(--border-subtle)`
- Text: `--text-muted`
- Hover: border becomes `--accent-primary` at 30% opacity

**Expanded state** (on click):
```
┌─────────────────────────────────────┐
│  ┌─────────────────────────────┐    │
│  │ ┌─────────────────────────┐ │    │
│  │ │ Task title...           │ │    │  ← Auto-focused input
│  │ └─────────────────────────┘ │    │
│  │ [More options]    [Cancel]  │    │  ← Ghost links
│  └─────────────────────────────┘    │
└─────────────────────────────────────┘
```

**Interactions**:
- Click ghost card → expand to inline form, auto-focus input
- Type title + **Enter** → create task, collapse, task appears
- **Escape** → collapse without creating
- "More options" → opens full `TaskCreationForm` modal

### Implementation

**New Component**: `InlineTaskAdd.tsx`

**Column.tsx Changes**:
- Add `onAddTask` prop
- Track hover state for column
- Show `InlineTaskAdd` at bottom when hovered + column allows adding

**Props**:
```typescript
interface InlineTaskAddProps {
  projectId: string;
  columnId: string; // 'draft' or 'backlog'
  onCreated?: (task: Task) => void;
  onOpenFullForm?: () => void;
}
```

---

## Part 2: Task Editing (Detail Modal Edit Mode)

### Design

**TaskDetailModal header with edit toggle**:
```
┌──────────────────────────────────────────────────────────────┐
│ [P2]  Implement auth               [Ready ▼] [✏️] [🗄] [×]   │
│       ┌─────────┐ ┌────────┐              ↑                  │
│       │ feature │ │Backlog │         Status dropdown         │
└──────────────────────────────────────────────────────────────┘
```

**Edit mode** (toggle via pencil icon):
```
┌──────────────────────────────────────────────────────────────┐
│       ┌─────────────────────────────────────────┐            │
│ [P ▼] │ Implement user auth                     │            │
│       └─────────────────────────────────────────┘            │
│       ┌──────────────┐                                       │
│       │ feature    ▼ │                                       │
│       └──────────────┘                                       │
├──────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────┐           │
│  │ Description text here...                      │           │
│  └───────────────────────────────────────────────┘           │
├──────────────────────────────────────────────────────────────┤
│                              [Cancel] [Save Changes]         │
└──────────────────────────────────────────────────────────────┘
```

**Editable fields**: Title, Category, Description, Priority
**Non-editable**: Status (use dropdown), Reviews, History, QA results

### Status Dropdown (Valid Transitions Only)

**User-Allowed Actions**:

| Current Status | Dropdown Options |
|----------------|------------------|
| `backlog` | Ready for Work, Cancel |
| `ready` | Mark Blocked, Cancel |
| `blocked` | Unblock, Cancel |
| `revision_needed` | Cancel |
| `approved` | Re-open |
| `failed` | Retry |
| `cancelled` | Re-open |
| `qa_failed` | Skip QA |

**System-controlled states** (no dropdown, badge only):
- `executing`, `execution_done`
- `qa_refining`, `qa_testing`, `qa_passed`
- `pending_review`

### Implementation

**TaskDetailModal.tsx Changes**:
- Add `isEditing` state
- Add `TaskEditForm` component (similar to `TaskCreationForm`)
- Add status dropdown with valid transitions
- Use `updateMutation` from `useTaskMutation`

**New Component**: `TaskEditForm.tsx`

**New Component**: `StatusDropdown.tsx`
```typescript
interface StatusDropdownProps {
  currentStatus: InternalStatus;
  onTransition: (newStatus: InternalStatus) => void;
}
```

---

## Part 3: Archive System (Soft Delete)

### Backend Changes

**Task Entity** - Add field:
```rust
pub struct Task {
    // ... existing fields ...
    /// When the task was archived (soft-deleted). None = active.
    pub archived_at: Option<DateTime<Utc>>,
}
```

**Database Migration**:
```sql
ALTER TABLE tasks ADD COLUMN archived_at TEXT;
CREATE INDEX idx_tasks_archived ON tasks(project_id, archived_at);
```

**New Tauri Commands**:

| Command | Signature | Purpose |
|---------|-----------|---------|
| `archive_task` | `(task_id: String)` | Set `archived_at = now()` |
| `restore_task` | `(task_id: String)` | Set `archived_at = NULL` |
| `permanently_delete_task` | `(task_id: String)` | Hard delete (only if archived) |
| `get_archived_count` | `(project_id: String) -> u32` | Count for badge |

**Repository Changes**:
- `get_by_project()` → exclude archived by default
- `get_by_project_with_archived(include_archived: bool)` → filter option

### Frontend Changes

**TaskDetailModal** (non-archived task):
```
[✏️ Edit] [🗄 Archive] [×]
```

**TaskDetailModal** (archived task) — **Restore only, no editing**:
```
┌──────────────────────────────────────────────────────────────┐
│ [P2]  Task title                  [↩️ Restore] [🗑 Delete] [×]│
│       ┌──────────────────────┐                               │
│       │ 🗄 Archived          │  ← Archived badge             │
│       └──────────────────────┘                               │
│                                                              │
│  No edit button — must restore first to make changes         │
└──────────────────────────────────────────────────────────────┘
```

**Kanban Header** (when archived count > 0):
```
┌─────────────────────────────────────────────────────────────────────┐
│ Workflow: [Default ▼]                        [☐ Show archived (3)] │
└─────────────────────────────────────────────────────────────────────┘
```

**Archived tasks appearance** (when toggle is on):
- Appear in their original column
- Reduced opacity (60%)
- Grayed out priority stripe
- Small archive badge overlay
- Click opens detail modal in "archived mode"

**Permanent Delete**:
- Only available for archived tasks
- Shows confirmation dialog before deletion

### Type Updates

**Frontend** (`types/task.ts`):
```typescript
export const TaskSchema = z.object({
  // ... existing fields ...
  archivedAt: z.string().datetime({ offset: true }).nullable(),
});
```

**API** (`lib/tauri.ts`):
```typescript
api.tasks = {
  // ... existing ...
  archive: (taskId: string) => ...,
  restore: (taskId: string) => ...,
  permanentlyDelete: (taskId: string) => ...,
  getArchivedCount: (projectId: string) => ...,
};
```

---

## Part 4: Search (Cmd+F)

### Design

```
┌─────────────────────────────────────────────────────────────────────┐
│ [🔍 Search tasks...                           ] [×]   [☐ Archived]  │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐            │
│  │ • Backlog   │     │ • In Prog.  │     │ • Done      │            │
│  │   [2 found] │     │   [1 found] │     │   [3 found] │            │
│  └─────────────┘     └─────────────┘     └─────────────┘            │
│    (columns with 0 matches hidden)                                  │
└─────────────────────────────────────────────────────────────────────┘
```

### Behavior

- `Cmd+F` (Mac) / `Ctrl+F` (Windows) while on board → Shows search bar
- **Search scope**: Title and description only (no category/priority/status filters)
- **Client-side filtering** for speed (tasks already loaded)
- Columns with 0 matches are **hidden**
- Match count badge in column header during search
- Respects "Show archived" toggle
- `Escape` or click [×] → clear search, restore all columns
- Matching text highlighted in task cards (optional enhancement)

### Implementation

**New Component**: `TaskSearchBar.tsx`
```typescript
interface TaskSearchBarProps {
  value: string;
  onChange: (value: string) => void;
  onClose: () => void;
  resultCount: number;
}
```

**TaskBoard Changes**:
- Add keyboard listener for Cmd+F
- Add `searchQuery` state
- Filter tasks before passing to columns
- Hide columns with 0 results

**Store Changes** (`uiStore.ts`):
- Add `boardSearchQuery: string | null`
- Add `setBoardSearchQuery(query: string | null)`

---

## Part 5: Infinite Scroll

### Design

Each column scrolls independently with infinite loading:

```
┌─────────────────────┐
│ • Backlog       [47]│
├─────────────────────┤
│ ┌─────────────────┐ │
│ │ Task 1          │ │
│ └─────────────────┘ │
│ ┌─────────────────┐ │
│ │ Task 2          │ │
│ └─────────────────┘ │
│        ...          │
│ ┌─────────────────┐ │
│ │ Loading...      │ │  ← Spinner when loading more
│ └─────────────────┘ │
└─────────────────────┘
```

### Behavior

- Initial load: **20 tasks per column**
- Load more when scrolled to **5 items from bottom**
- **No caching** - refetch on scroll
- **No virtualization** for now (keep it simple)
- Loading indicator at bottom of column during fetch

### Backend Changes

**Update `list_tasks` command**:
```rust
#[tauri::command]
pub async fn list_tasks(
    project_id: String,
    status: Option<String>,      // Filter by internal status
    offset: Option<u32>,         // Pagination offset
    limit: Option<u32>,          // Page size (default 20)
    include_archived: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> Result<TaskListResponse, AppError>

struct TaskListResponse {
    tasks: Vec<Task>,
    total: u32,
    has_more: bool,
}
```

### Frontend Implementation

**Hook**: `useInfiniteTasksQuery.ts`
```typescript
function useInfiniteTasksQuery(projectId: string, status: InternalStatus) {
  return useInfiniteQuery({
    queryKey: ['tasks', projectId, status],
    queryFn: ({ pageParam = 0 }) =>
      api.tasks.list(projectId, { status, offset: pageParam, limit: 20 }),
    getNextPageParam: (lastPage) =>
      lastPage.hasMore ? lastPage.offset + 20 : undefined,
    staleTime: 5 * 60 * 1000,  // Cache for 5 minutes
  });
}
```

**Caching Strategy**:
- TanStack Query handles caching automatically
- Loaded pages are cached in memory
- Cache key: `['tasks', projectId, status]`
- Stale time: 5 minutes (configurable)
- Background refetch on window focus (default TanStack behavior)
- Invalidate on task create/update/delete/archive

**Column Changes**:
- Use intersection observer at bottom of list
- Call `fetchNextPage()` when visible
- Show loading spinner during fetch
- Pages merge automatically via TanStack Query

---

## Part 6: Drag & Drop Restrictions

### Source Restrictions (Which tasks can be dragged?)

Tasks in **system-controlled states** cannot be dragged - they're being processed.

| Status | Can Drag? | Reason |
|--------|-----------|--------|
| `backlog` | **YES** | Idle, user can schedule or cancel |
| `ready` | **YES** | Idle, user can block or cancel |
| `blocked` | **YES** | Idle, user can unblock or cancel |
| `executing` | **NO** | Worker is running |
| `execution_done` | **NO** | Routing to QA/review |
| `qa_refining` | **NO** | QA flow in progress |
| `qa_testing` | **NO** | QA flow in progress |
| `qa_passed` | **NO** | QA flow in progress |
| `qa_failed` | **NO** | QA flow in progress |
| `pending_review` | **NO** | Awaiting review |
| `revision_needed` | **NO** | Auto-transitions to Executing |
| `approved` | **YES** | Terminal, user can re-open |
| `failed` | **YES** | Terminal, user can retry |
| `cancelled` | **YES** | Terminal, user can re-open |

### Target Restrictions (Which columns can receive drops?)

| Target Column | maps_to | Can Drop? | Valid Source Statuses |
|---------------|---------|-----------|----------------------|
| `draft` | Backlog | **YES** | Terminal states only (re-open) |
| `backlog` | Backlog | **YES** | Terminal states only |
| `todo` | Ready | **YES** | Backlog, Blocked, Terminal |
| `planned` | Ready | **YES** | Backlog, Blocked, Terminal |
| `in_progress` | Executing | **NO** | None (system-controlled) |
| `in_review` | PendingReview | **NO** | None (system-controlled) |
| `done` | Approved | **NO** | None (system-controlled) |

### Visual Feedback

1. **Non-draggable tasks**: No grab cursor, card appears slightly muted, tooltip on hover explains why
2. **Invalid drop target**: Red dashed border + X icon (already implemented)
3. **Valid drop target**: Orange glow (already implemented)

### Implementation

**TaskCard.tsx Changes**:
```typescript
const isDraggable = useMemo(() => {
  const nonDraggableStatuses = [
    'executing', 'execution_done',
    'qa_refining', 'qa_testing', 'qa_passed', 'qa_failed',
    'pending_review', 'revision_needed'
  ];
  return !nonDraggableStatuses.includes(task.internalStatus);
}, [task.internalStatus]);
```

**Validation function** for drop targets:
```typescript
function canDropOnColumn(sourceStatus: InternalStatus, targetColumn: string): boolean {
  const targetStatus = columnToStatusMap[targetColumn];
  return sourceStatus.canTransitionTo(targetStatus);
}
```

---

## Part 7: Task Card Context Menu

Right-click on a task card shows a context menu with quick actions.

### Design

```
┌───────────────────────────┐
│ [P2] Task title           │
│      feature              │  Right-click →  ┌──────────────────┐
└───────────────────────────┘                 │ 👁 View Details  │
                                              │ ✏️ Edit          │
                                              │ ─────────────────│
                                              │ 🗄 Archive       │
                                              │ ❌ Cancel        │
                                              └──────────────────┘
```

### Menu Items by Status

| Status | Available Actions |
|--------|-------------------|
| `backlog` | View, Edit, Archive, Cancel |
| `ready` | View, Edit, Archive, Block, Cancel |
| `blocked` | View, Edit, Archive, Unblock, Cancel |
| System-controlled | View only |
| `approved` | View, Archive, Re-open |
| `failed` | View, Archive, Retry |
| `cancelled` | View, Archive, Re-open |

### Implementation

**New Component**: `TaskCardContextMenu.tsx`

Uses shadcn `ContextMenu` component:
```tsx
<ContextMenu>
  <ContextMenuTrigger asChild>
    <TaskCard task={task} />
  </ContextMenuTrigger>
  <ContextMenuContent>
    <ContextMenuItem onClick={() => openModal('task-detail', { task })}>
      <Eye className="w-4 h-4 mr-2" /> View Details
    </ContextMenuItem>
    {canEdit && (
      <ContextMenuItem onClick={() => openModal('task-edit', { task })}>
        <Edit className="w-4 h-4 mr-2" /> Edit
      </ContextMenuItem>
    )}
    <ContextMenuSeparator />
    <ContextMenuItem onClick={() => archiveMutation.mutate(task.id)}>
      <Archive className="w-4 h-4 mr-2" /> Archive
    </ContextMenuItem>
    {canCancel && (
      <ContextMenuItem onClick={() => cancelMutation.mutate(task.id)}>
        <X className="w-4 h-4 mr-2" /> Cancel
      </ContextMenuItem>
    )}
  </ContextMenuContent>
</ContextMenu>
```

---

## Part 8: Keyboard Shortcuts

### Kanban Board Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+N` / `Ctrl+N` | Open full create task modal |
| `Cmd+F` / `Ctrl+F` | Open search bar |
| `Escape` | Close search bar (if open) |

### Implementation

**TaskBoard.tsx** - Add keyboard listener:
```typescript
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    // Cmd+N or Ctrl+N → Open create task modal
    if ((e.metaKey || e.ctrlKey) && e.key === 'n') {
      e.preventDefault();
      openModal('task-create', { projectId });
    }
    // Cmd+F or Ctrl+F → Open search
    if ((e.metaKey || e.ctrlKey) && e.key === 'f') {
      e.preventDefault();
      setSearchOpen(true);
    }
    // Escape → Close search
    if (e.key === 'Escape' && searchOpen) {
      setSearchOpen(false);
      setSearchQuery('');
    }
  };

  window.addEventListener('keydown', handleKeyDown);
  return () => window.removeEventListener('keydown', handleKeyDown);
}, [openModal, projectId, searchOpen]);
```

**Note**: Prevent default to avoid browser's native find dialog on Cmd+F.

---

## Part 9: Empty Search State ("Message in a Bottle")

When search returns no results, turn it into a task creation opportunity.

### Design

```
┌─────────────────────────────────────────────────────────────────────┐
│ [🔍 add user login                        ] [×]   [☐ Archived]      │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│                          📝                                         │
│                                                                     │
│              No tasks match "add user login"                        │
│                                                                     │
│                    Should this be a task?                           │
│                                                                     │
│           [+ Create "add user login"]    [Clear Search]             │
│                                                                     │
│     ┌─────────────────────────────────────────────────────┐        │
│     │  💡 Tip: Enable "Show archived" to search old tasks │        │
│     └─────────────────────────────────────────────────────┘        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Key elements**:
- Notepad icon (📝 or Lucide `FileText`)
- Quoted search term shown back to user
- Primary CTA: Create task with search term as title (pre-filled)
- Secondary: Clear search button
- Tip about archived toggle (only shown if toggle is off)

---

## Implementation Order

### Phase A: Core CRUD
1. **Backend: Archive system** (migration, commands, repository)
2. **Frontend: Task Edit Mode** (TaskEditForm, status dropdown in modal)
3. **Frontend: Archive UI** (buttons in modal, toggle in header, archived appearance)

### Phase B: Kanban Enhancements
4. **Frontend: Drag-drop restrictions** (source/target validation)
5. **Frontend: Context menu** (right-click on task cards)
6. **Frontend: Inline Quick-Add** (ghost card on column hover)

### Phase C: Data Management
7. **Backend: Pagination** (update list_tasks command with offset/limit)
8. **Frontend: Infinite Scroll** (useInfiniteQuery with caching)

### Phase D: Search & Keyboard
9. **Frontend: Keyboard shortcuts** (Cmd+N for create, Cmd+F for search)
10. **Frontend: Search bar** (column filtering, persistence)
11. **Frontend: Empty search state** ("message in a bottle" create prompt)

---

## Open Questions

### All Resolved

| Question | Answer |
|----------|--------|
| Should search filter by category/priority/status? | **No** - title and description only |
| Cache loaded pages for infinite scroll? | **Yes** - cache loaded pages (5 min stale time) |
| Use virtualization for large columns? | **No** - keep it simple for now |
| Bulk archive support? | **No** |
| Permanent delete confirmation? | **Yes** |
| Search highlighting? | **No** |
| Search persistence? | **Yes** - persist when navigating away and back |
| Archive from Kanban? | **Yes** - via right-click context menu |
| Archived task editing? | **Restore only** - must restore before editing |
| Empty state during search? | **"Message in a bottle"** - offer to create task from search term |

---

## Files to Create/Modify

### New Files

| File | Purpose |
|------|---------|
| `src/components/tasks/InlineTaskAdd.tsx` | Inline quick-add ghost card |
| `src/components/tasks/TaskEditForm.tsx` | Edit form for task detail modal |
| `src/components/tasks/StatusDropdown.tsx` | Status transition dropdown |
| `src/components/tasks/TaskSearchBar.tsx` | Search bar component |
| `src/components/tasks/TaskCardContextMenu.tsx` | Right-click context menu |
| `src/components/tasks/EmptySearchState.tsx` | Creative no-results state |
| `src/hooks/useInfiniteTasksQuery.ts` | Infinite scroll query hook with caching |
| `src/lib/statusTransitions.ts` | Utility for valid transition checks |

### Modified Files

| File | Changes |
|------|---------|
| `src-tauri/src/domain/entities/task.rs` | Add `archived_at` field |
| `src-tauri/src/infrastructure/sqlite/migrations.rs` | Add migration |
| `src-tauri/src/commands/task_commands.rs` | Add archive/restore/pagination |
| `src-tauri/src/domain/repositories/task_repository.rs` | Add archive-aware methods |
| `src/types/task.ts` | Add `archivedAt` field |
| `src/types/status.ts` | Add `canTransitionTo()` utility |
| `src/lib/tauri.ts` | Add archive/restore/pagination bindings |
| `src/hooks/useTaskMutation.ts` | Add archive/restore/cancel mutations |
| `src/stores/uiStore.ts` | Add `showArchived`, `boardSearchQuery` |
| `src/components/tasks/TaskBoard/Column.tsx` | Add hover state, inline add |
| `src/components/tasks/TaskBoard/TaskBoard.tsx` | Add search, infinite scroll, keyboard listener |
| `src/components/tasks/TaskBoard/TaskCard.tsx` | Add `isDraggable` logic, wrap with context menu |
| `src/components/tasks/TaskDetailModal.tsx` | Add edit mode, status dropdown, archive buttons |
