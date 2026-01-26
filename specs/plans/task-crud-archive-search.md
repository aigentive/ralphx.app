# Phase 18: Task CRUD, Archive & Search

**Status**: Pending
**Prerequisites**: Phase 17 (Worker Artifact Context)
**Created**: 2026-01-26

---

## Overview

This phase completes the task management system with full CRUD operations, archive system (soft delete), search capabilities, infinite scroll pagination, and enhanced UI interactions. It builds on the existing foundation (Phases 1-17) which already provides:

- ✅ Task entity with state machine integration
- ✅ Kanban board with drag-drop (dnd-kit)
- ✅ Task creation form (modal)
- ✅ Task detail modal (read-only)
- ✅ Basic drag-drop validation
- ✅ State machine with 14 internal statuses
- ✅ TransitionHandler for status changes with entry actions

**What's New in This Phase:**

1. **Task Editing** - Edit mode in TaskDetailModal with status transitions
2. **Archive System** - Soft delete with restore capability and permanent delete
3. **Inline Quick-Add** - Ghost card on column hover for fast task creation
4. **Task Search** - Cmd+F search bar with column filtering and empty state UX
5. **Infinite Scroll** - Pagination per column with TanStack Query caching
6. **Context Menu** - Right-click actions on task cards
7. **Keyboard Shortcuts** - Cmd+N for create, Cmd+F for search, Escape to close
8. **Enhanced Drag-Drop** - Refined restrictions for system-controlled states

---

## Current State

### Backend (Rust/Tauri)

**What Exists:**
- Task entity: all core fields (id, projectId, category, title, description, priority, internalStatus, timestamps)
- Tauri commands: `list_tasks`, `get_task`, `create_task`, `update_task`, `delete_task` (hard delete), `move_task`
- Repository trait: TaskRepository with CRUD operations
- SQLite implementation: SqliteTaskRepository
- State machine: statig with 14 statuses and valid transitions

**What's Missing:**
- `archived_at` field in Task entity
- Archive commands: `archive_task`, `restore_task`, `permanently_delete_task`, `get_archived_count`
- Pagination support: offset/limit/has_more in `list_tasks`

### Frontend (React/TypeScript)

**What Exists:**
- Task type with Zod schema (matches backend)
- API wrappers: `api.tasks.list()`, `api.tasks.create()`, `api.tasks.update()`, `api.tasks.delete()`, `api.tasks.move()`
- Mutations: `useTaskMutation()` hook with createMutation, updateMutation, deleteMutation, moveMutation
- Query: `useTasks()` hook with TanStack Query caching
- Components: TaskBoard, TaskCard, Column, TaskDetailModal (read-only), TaskCreationForm (working)
- Stores: useTaskStore (task state), uiStore (modal states)

**What's Missing:**
- `archivedAt` field in TaskSchema
- Archive mutations: archiveMutation, restoreMutation, permanentlyDeleteMutation
- Edit mode in TaskDetailModal
- StatusDropdown component
- InlineTaskAdd component
- TaskSearchBar component
- TaskCardContextMenu component
- EmptySearchState component
- useInfiniteTasksQuery hook for pagination
- showArchived, boardSearchQuery state in uiStore
- Keyboard listeners for Cmd+N, Cmd+F, Escape

---

## Technical Architecture

### Status System

**14 Internal Statuses** (from state machine):
- **Idle**: backlog, ready, blocked
- **Active**: executing, execution_done, qa_refining, qa_testing, qa_passed, qa_failed, pending_review, revision_needed
- **Terminal**: approved, failed, cancelled

**Workflow Columns** (UI grouping):
- `draft` → maps to `backlog`
- `backlog` → maps to `backlog`
- `todo` → maps to `ready`
- `planned` → maps to `ready`
- `in_progress` → maps to `executing`
- `in_review` → maps to `pending_review`
- `done` → maps to `approved`

**Key Insight**: Multiple columns can map to the same internal status. The column is for UI organization; the internal status controls state machine behavior.

### State Machine Integration

**CRITICAL**: Always use `TransitionHandler` for status changes. NEVER update task status directly in the database.

- `move_task` command uses TransitionHandler
- Entry actions trigger agent spawning, review start, event emission
- Valid transitions enforced by statig state machine
- See `src-tauri/CLAUDE.md` for detailed architecture

### Drag-Drop Restrictions

**Source Restrictions** (which tasks can be dragged):
- **Draggable**: backlog, ready, blocked (idle states) + approved, failed, cancelled (terminal states)
- **Non-draggable**: executing, execution_done, qa_* (QA flow), pending_review, revision_needed (system-controlled)

**Target Restrictions** (which columns accept drops):
- **Accept drops**: draft, backlog, todo, planned (user-controlled columns)
- **No drops**: in_progress, in_review, done (system-controlled columns)

**Validation**: Drop allowed only if source status can transition to target column's mapped status.

---

## Part 1: Archive System (Soft Delete)

### Backend Changes

**Task Entity** - Add field:
```rust
// src-tauri/src/domain/entities/task.rs
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
| `archive_task` | `(task_id: String) -> Result<Task, AppError>` | Set `archived_at = now()`, return updated task |
| `restore_task` | `(task_id: String) -> Result<Task, AppError>` | Set `archived_at = NULL`, return updated task |
| `permanently_delete_task` | `(task_id: String) -> Result<(), AppError>` | Hard delete (only if archived), confirmation required |
| `get_archived_count` | `(project_id: String) -> Result<u32, AppError>` | Count archived tasks for badge |

**Repository Changes**:
```rust
// src-tauri/src/domain/repositories/task_repository.rs
pub trait TaskRepository: Send + Sync {
    // ... existing methods ...

    /// Get tasks by project, optionally including archived
    async fn get_by_project_filtered(
        &self,
        project_id: &str,
        include_archived: bool,
    ) -> Result<Vec<Task>, AppError>;

    /// Archive a task (soft delete)
    async fn archive(&self, task_id: &str) -> Result<Task, AppError>;

    /// Restore an archived task
    async fn restore(&self, task_id: &str) -> Result<Task, AppError>;

    /// Count archived tasks for a project
    async fn get_archived_count(&self, project_id: &str) -> Result<u32, AppError>;
}
```

**Default Behavior**: All existing queries (`get_by_project`, `list_tasks`) should exclude archived tasks by default. Use `include_archived: true` to see archived tasks.

### Frontend Changes

**Type Updates** (`src/types/task.ts`):
```typescript
export const TaskSchema = z.object({
  // ... existing fields ...
  archivedAt: z.string().datetime({ offset: true }).nullable(),
});
```

**API Bindings** (`src/lib/tauri.ts`):
```typescript
api.tasks = {
  // ... existing ...
  archive: (taskId: string): Promise<Task> =>
    invoke('archive_task', { taskId }),
  restore: (taskId: string): Promise<Task> =>
    invoke('restore_task', { taskId }),
  permanentlyDelete: (taskId: string): Promise<void> =>
    invoke('permanently_delete_task', { taskId }),
  getArchivedCount: (projectId: string): Promise<number> =>
    invoke('get_archived_count', { projectId }),
};
```

**Mutations** (`src/hooks/useTaskMutation.ts`):
```typescript
export function useTaskMutation() {
  const queryClient = useQueryClient();

  // ... existing mutations ...

  const archiveMutation = useMutation({
    mutationFn: api.tasks.archive,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
      queryClient.invalidateQueries({ queryKey: ['archived-count'] });
    },
  });

  const restoreMutation = useMutation({
    mutationFn: api.tasks.restore,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
      queryClient.invalidateQueries({ queryKey: ['archived-count'] });
    },
  });

  const permanentlyDeleteMutation = useMutation({
    mutationFn: api.tasks.permanentlyDelete,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
      queryClient.invalidateQueries({ queryKey: ['archived-count'] });
    },
  });

  return {
    // ... existing ...
    archiveMutation,
    restoreMutation,
    permanentlyDeleteMutation,
  };
}
```

**UI State** (`src/stores/uiStore.ts`):
```typescript
interface UiStore {
  // ... existing ...
  showArchived: boolean;
  setShowArchived: (show: boolean) => void;
}
```

**Board Header Toggle**:
```tsx
// In TaskBoard.tsx header
{archivedCount > 0 && (
  <ToggleButton
    pressed={showArchived}
    onPressedChange={setShowArchived}
  >
    <Archive className="w-4 h-4 mr-2" />
    Show archived ({archivedCount})
  </ToggleButton>
)}
```

**Archived Task Appearance** (when `showArchived` is true):
- Appear in their original column
- Reduced opacity: 60% (`opacity-60`)
- Grayed out priority stripe
- Small archive badge overlay (Archive icon, top-right corner)
- Click opens TaskDetailModal in "archived mode"

**TaskDetailModal - Archived Mode**:
- Show archive badge at top
- Hide edit button
- Replace archive button with "Restore" and "Delete Permanently" buttons
- Permanent delete shows confirmation dialog before deletion

**TaskDetailModal - Active Mode** (non-archived):
- Show edit button (pencil icon)
- Show archive button
- Normal functionality

---

## Part 2: Task Editing (Edit Mode in Detail Modal)

### Design

**TaskDetailModal Header** (active task):
```
┌──────────────────────────────────────────────────────────────┐
│ [P2]  Implement auth               [Ready ▼] [✏️] [🗄] [×]   │
│       ┌─────────┐ ┌────────┐              ↑                  │
│       │ feature │ │Backlog │         Status dropdown         │
└──────────────────────────────────────────────────────────────┘
```

**Edit Mode** (toggle via pencil icon):
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

**Editable Fields**: Title, Category, Description, Priority

**Non-Editable**: Status (use dropdown), Reviews, History, QA results, Timestamps

### Status Dropdown (Valid Transitions Only)

**Critical**: Status dropdown should query the state machine for valid transitions, not hardcode them.

**Backend Helper** (add to task_commands.rs):
```rust
#[tauri::command]
pub async fn get_valid_transitions(
    task_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let task = state.task_repo.get(&task_id).await?;
    let current_status = task.internal_status;

    // Query state machine for valid transitions
    let transitions = current_status.valid_transitions();

    Ok(transitions.into_iter().map(|s| s.to_string()).collect())
}
```

**Frontend Component**: `StatusDropdown.tsx`
```typescript
interface StatusDropdownProps {
  taskId: string;
  currentStatus: InternalStatus;
  onTransition: (newStatus: InternalStatus) => void;
}

// Fetches valid transitions from backend
const { data: validTransitions } = useQuery({
  queryKey: ['valid-transitions', taskId],
  queryFn: () => api.tasks.getValidTransitions(taskId),
});
```

**User-Allowed Actions** (typical transitions):

| Current Status | Dropdown Options (examples) |
|----------------|------------------------------|
| `backlog` | Ready for Work, Cancel |
| `ready` | Mark Blocked, Cancel |
| `blocked` | Unblock, Cancel |
| `revision_needed` | Cancel |
| `approved` | Re-open |
| `failed` | Retry |
| `cancelled` | Re-open |
| `qa_failed` | Skip QA |

**System-Controlled States** (no dropdown, badge only):
- `executing`, `execution_done`
- `qa_refining`, `qa_testing`, `qa_passed`
- `pending_review`

### Implementation

**New Component**: `TaskEditForm.tsx`
- Similar structure to TaskCreationForm
- Pre-populated with task data
- Uses `updateMutation` from `useTaskMutation()`
- Form validation with Zod schema

**TaskDetailModal Changes**:
- Add `isEditing` state
- Add edit button (pencil icon) - only visible for non-archived, non-system-controlled tasks
- Conditionally render TaskEditForm vs. read-only view
- Add StatusDropdown component (fetches valid transitions)

---

## Part 3: Inline Quick-Add (Ghost Card)

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
│  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┐    │  ← Appears on hover (NOT during drag)
│  │ + Add task                  │    │
│  └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┘    │
└─────────────────────────────────────┘
```

**Collapsed State** (ghost card):
- Dashed border: `2px dashed` with `--border-subtle`
- Text: `--text-muted`
- Hover: border becomes `--accent-primary` at 30% opacity

**Expanded State** (on click):
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
- Column hover (when NOT dragging) → show ghost card
- Click ghost card → expand to inline form, auto-focus input
- Type title + **Enter** → create task with minimal fields, collapse, task appears in column
- **Escape** → collapse without creating
- "More options" → opens full TaskCreationForm modal for advanced fields

### Implementation

**New Component**: `InlineTaskAdd.tsx`
```typescript
interface InlineTaskAddProps {
  projectId: string;
  columnId: string; // 'draft' or 'backlog'
  onCreated?: (task: Task) => void;
}

// Two states: collapsed (ghost) and expanded (form)
// Uses createMutation from useTaskMutation()
// Creates task with: { title, projectId, category: 'feature', status: columnId }
```

**Column.tsx Changes**:
- Add `isHovered` state
- Add `onMouseEnter` and `onMouseLeave` handlers
- Conditionally render `<InlineTaskAdd>` at bottom when:
  - Column is hovered
  - Column is `draft` or `backlog`
  - NOT currently dragging (check `isDragging` from dnd-kit context)

**Adjustment from Plan**: The plan didn't mention the drag-drop conflict. We MUST check if user is currently dragging before showing the ghost card, otherwise it interferes with drop zone detection.

---

## Part 4: Task Search (Cmd+F)

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

- `Cmd+F` (Mac) / `Ctrl+F` (Windows) while on board → shows search bar
- `e.preventDefault()` to avoid browser's native find dialog
- **Search scope**: Title and description only (case-insensitive)
- **Client-side filtering** for speed (tasks already loaded via infinite scroll)
- Columns with 0 matches are **hidden**
- Match count badge in column header during search
- Respects "Show archived" toggle (searches archived tasks if visible)
- `Escape` or click [×] → clear search, restore all columns
- Search persists when navigating away and back (stored in uiStore)

**Future Enhancement**: Server-side search for very large datasets (not in this phase).

### Implementation

**New Component**: `TaskSearchBar.tsx`
```typescript
interface TaskSearchBarProps {
  value: string;
  onChange: (value: string) => void;
  onClose: () => void;
  resultCount: number;
  showArchived: boolean;
  onToggleArchived: () => void;
}
```

**UI Store** (`src/stores/uiStore.ts`):
```typescript
interface UiStore {
  // ... existing ...
  boardSearchQuery: string | null;
  setBoardSearchQuery: (query: string | null) => void;
}
```

**TaskBoard Changes**:
- Add keyboard listener for Cmd+F (Mac) / Ctrl+F (Windows)
- Add `searchOpen` state
- Conditionally render `TaskSearchBar` at top when `searchOpen`
- Filter tasks before passing to columns:
  ```typescript
  const filteredTasks = useMemo(() => {
    if (!boardSearchQuery) return tasks;

    const query = boardSearchQuery.toLowerCase();
    return tasks.filter(task =>
      task.title.toLowerCase().includes(query) ||
      (task.description?.toLowerCase().includes(query) ?? false)
    );
  }, [tasks, boardSearchQuery]);
  ```
- Hide columns with 0 results during search
- Show match count badge in column header

### Empty Search State ("Message in a Bottle")

When search returns no results:

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

**New Component**: `EmptySearchState.tsx`
```typescript
interface EmptySearchStateProps {
  searchQuery: string;
  onCreateTask: () => void; // Opens TaskCreationForm with pre-filled title
  onClearSearch: () => void;
  showArchived: boolean;
}
```

**Key Elements**:
- Notepad icon (Lucide `FileText`)
- Quoted search term shown back to user
- Primary CTA: "Create [search term]" → opens TaskCreationForm with title pre-filled
- Secondary: "Clear Search" button
- Tip about archived toggle (only shown if `showArchived` is false)

---

## Part 5: Infinite Scroll (Pagination)

### Backend Changes

**Update `list_tasks` Command**:
```rust
#[tauri::command]
pub async fn list_tasks(
    project_id: String,
    status: Option<String>,      // Filter by internal status
    offset: Option<u32>,         // Pagination offset (default 0)
    limit: Option<u32>,          // Page size (default 20)
    include_archived: Option<bool>, // Include archived tasks (default false)
    state: tauri::State<'_, AppState>,
) -> Result<TaskListResponse, AppError> {
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(20);
    let include_archived = include_archived.unwrap_or(false);

    let tasks = state.task_repo.list_paginated(
        &project_id,
        status.as_deref(),
        offset,
        limit,
        include_archived,
    ).await?;

    let total = state.task_repo.count_tasks(&project_id, include_archived).await?;
    let has_more = (offset + tasks.len() as u32) < total;

    Ok(TaskListResponse { tasks, total, has_more, offset })
}

#[derive(Serialize)]
pub struct TaskListResponse {
    pub tasks: Vec<Task>,
    pub total: u32,
    pub has_more: bool,
    pub offset: u32,
}
```

**Repository Changes**:
```rust
pub trait TaskRepository: Send + Sync {
    // ... existing methods ...

    async fn list_paginated(
        &self,
        project_id: &str,
        status: Option<&str>,
        offset: u32,
        limit: u32,
        include_archived: bool,
    ) -> Result<Vec<Task>, AppError>;

    async fn count_tasks(
        &self,
        project_id: &str,
        include_archived: bool,
    ) -> Result<u32, AppError>;
}
```

### Frontend Implementation

**Hook**: `useInfiniteTasksQuery.ts`
```typescript
interface TaskListParams {
  projectId: string;
  status?: InternalStatus;
  includeArchived?: boolean;
}

export function useInfiniteTasksQuery({ projectId, status, includeArchived }: TaskListParams) {
  return useInfiniteQuery({
    queryKey: ['tasks', projectId, status, includeArchived],
    queryFn: ({ pageParam = 0 }) =>
      api.tasks.list({
        projectId,
        status,
        offset: pageParam,
        limit: 20,
        includeArchived,
      }),
    getNextPageParam: (lastPage) =>
      lastPage.hasMore ? lastPage.offset + 20 : undefined,
    staleTime: 10 * 60 * 1000,  // 10 minutes (longer for local-first app)
    gcTime: 30 * 60 * 1000,     // 30 minutes cache retention
  });
}
```

**Caching Strategy**:
- TanStack Query handles caching automatically
- Loaded pages cached in memory
- Cache key: `['tasks', projectId, status, includeArchived]`
- **Stale time: 10 minutes** (longer than plan's 5 min, better for local app)
- Background refetch on window focus (default TanStack behavior)
- Invalidate on task create/update/delete/archive/restore

**Column Changes**:
- Use intersection observer at bottom of task list
- Call `fetchNextPage()` when observer triggers
- Show loading spinner at bottom during fetch
- Pages merge automatically via TanStack Query
- Flatten pages: `const tasks = data?.pages.flatMap(p => p.tasks) ?? []`

**Adjustment from Plan**: Increased stale time to 10 minutes (from 5) because this is a local-first Tauri app with event-driven updates, not a remote API. Longer cache is more performant and still safe.

---

## Part 6: Context Menu (Right-Click Actions)

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
| Archived | View, Restore, Delete Permanently |

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

    {!task.archivedAt && (
      <ContextMenuItem onClick={() => archiveMutation.mutate(task.id)}>
        <Archive className="w-4 h-4 mr-2" /> Archive
      </ContextMenuItem>
    )}

    {task.archivedAt && (
      <>
        <ContextMenuItem onClick={() => restoreMutation.mutate(task.id)}>
          <RotateCcw className="w-4 h-4 mr-2" /> Restore
        </ContextMenuItem>
        <ContextMenuItem
          onClick={() => confirmPermanentDelete(task.id)}
          className="text-destructive"
        >
          <Trash className="w-4 h-4 mr-2" /> Delete Permanently
        </ContextMenuItem>
      </>
    )}

    {canCancel && (
      <ContextMenuItem onClick={() => moveMutation.mutate({ taskId: task.id, toStatus: 'cancelled' })}>
        <X className="w-4 h-4 mr-2" /> Cancel
      </ContextMenuItem>
    )}
  </ContextMenuContent>
</ContextMenu>
```

**Keyboard Accessibility** (shadcn handles this automatically):
- Arrow keys: Navigate menu items
- Enter/Space: Activate item
- Escape: Close menu

**Adjustment from Plan**: Added keyboard accessibility note (missing in original plan).

---

## Part 7: Keyboard Shortcuts

### Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+N` / `Ctrl+N` | Open full TaskCreationForm modal |
| `Cmd+F` / `Ctrl+F` | Open search bar |
| `Escape` | Close search bar (if open) |

### Implementation

**TaskBoard.tsx** - Add keyboard listener:
```typescript
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    // Ignore if user is typing in an input/textarea
    if (
      e.target instanceof HTMLInputElement ||
      e.target instanceof HTMLTextAreaElement
    ) {
      return;
    }

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
      setBoardSearchQuery(null);
    }
  };

  window.addEventListener('keydown', handleKeyDown);
  return () => window.removeEventListener('keydown', handleKeyDown);
}, [openModal, projectId, searchOpen, setBoardSearchQuery]);
```

**Note**: `e.preventDefault()` is critical to prevent browser's native find dialog on Cmd+F.

---

## Part 8: Enhanced Drag-Drop Restrictions

### Current State
- Basic drag-drop implemented with dnd-kit
- Column-to-column validation exists
- Visual feedback for valid/invalid drops

### Enhancements Needed

**Non-Draggable Tasks**:
- System-controlled statuses: `executing`, `execution_done`, `qa_*`, `pending_review`, `revision_needed`
- Visual indicators:
  - No grab cursor (`cursor-default` instead of `cursor-grab`)
  - Slightly muted appearance (`opacity-75`)
  - Tooltip on hover: "This task is being processed and cannot be moved manually"

**TaskCard.tsx Changes**:
```typescript
const isDraggable = useMemo(() => {
  const nonDraggableStatuses: InternalStatus[] = [
    'executing', 'execution_done',
    'qa_refining', 'qa_testing', 'qa_passed', 'qa_failed',
    'pending_review', 'revision_needed'
  ];
  return !nonDraggableStatuses.includes(task.internalStatus);
}, [task.internalStatus]);

// In JSX:
<div
  className={cn(
    "task-card",
    !isDraggable && "opacity-75 cursor-default"
  )}
  {...(isDraggable ? { ...attributes, ...listeners } : {})}
  title={!isDraggable ? "This task is being processed and cannot be moved manually" : undefined}
>
```

**Validation Function** for Drop Targets:
```typescript
function canDropOnColumn(
  sourceStatus: InternalStatus,
  targetColumn: string
): boolean {
  const targetStatus = columnToStatusMap[targetColumn];

  // Query state machine for valid transition
  return sourceStatus.canTransitionTo(targetStatus);
}
```

**Adjustment from Plan**: This was marked as "already implemented" but my exploration shows it's only partially done. The basic drop validation exists, but the non-draggable visual treatment and tooltips are missing.

---

## Testing Requirements

### Backend Tests

**Archive System**:
- [ ] Test archive_task sets `archived_at` correctly
- [ ] Test restore_task clears `archived_at`
- [ ] Test permanently_delete_task only works on archived tasks
- [ ] Test permanently_delete_task fails on non-archived tasks
- [ ] Test get_archived_count returns correct count
- [ ] Test list_tasks excludes archived by default
- [ ] Test list_tasks includes archived when flag is true

**Pagination**:
- [ ] Test list_tasks with offset/limit returns correct slice
- [ ] Test has_more is true when more tasks exist
- [ ] Test has_more is false on last page
- [ ] Test total count is accurate
- [ ] Test pagination works with status filter
- [ ] Test pagination respects archived filter

**Valid Transitions**:
- [ ] Test get_valid_transitions returns correct options per status
- [ ] Test state machine integration

### Frontend Tests

**Archive UI**:
- [ ] Test archive button archives task
- [ ] Test restore button restores task
- [ ] Test permanent delete shows confirmation dialog
- [ ] Test permanent delete only enabled for archived tasks
- [ ] Test "Show archived" toggle filters correctly
- [ ] Test archived tasks appear with reduced opacity
- [ ] Test archived badge displays

**Edit Mode**:
- [ ] Test edit button opens edit form
- [ ] Test edit form pre-populates with task data
- [ ] Test save updates task correctly
- [ ] Test cancel discards changes
- [ ] Test status dropdown shows only valid transitions
- [ ] Test edit button hidden for system-controlled statuses
- [ ] Test edit button hidden for archived tasks

**Inline Quick-Add**:
- [ ] Test ghost card appears on column hover (not during drag)
- [ ] Test ghost card only in draft/backlog columns
- [ ] Test click expands to form with auto-focus
- [ ] Test Enter creates task and collapses
- [ ] Test Escape cancels without creating
- [ ] Test "More options" opens full modal

**Search**:
- [ ] Test Cmd+F / Ctrl+F opens search bar
- [ ] Test search filters by title and description
- [ ] Test search respects archived toggle
- [ ] Test columns with 0 results hidden
- [ ] Test match count badge displays
- [ ] Test Escape closes search
- [ ] Test empty state shown when no results
- [ ] Test "Create from search" pre-fills title
- [ ] Test search query persists in uiStore

**Infinite Scroll**:
- [ ] Test initial load fetches 20 tasks
- [ ] Test scroll to bottom loads next page
- [ ] Test loading spinner shows during fetch
- [ ] Test pages merge correctly
- [ ] Test no duplicate tasks
- [ ] Test cache invalidation on mutations

**Context Menu**:
- [ ] Test right-click opens menu
- [ ] Test menu items vary by status
- [ ] Test View opens detail modal
- [ ] Test Edit opens edit mode
- [ ] Test Archive archives task
- [ ] Test Restore restores archived task
- [ ] Test keyboard navigation works

**Keyboard Shortcuts**:
- [ ] Test Cmd+N opens create modal
- [ ] Test Cmd+F opens search
- [ ] Test Escape closes search
- [ ] Test shortcuts ignored when typing in input

**Drag-Drop Restrictions**:
- [ ] Test system-controlled tasks not draggable
- [ ] Test non-draggable tasks have muted appearance
- [ ] Test tooltip shows on non-draggable hover
- [ ] Test drag validation uses state machine
- [ ] Test invalid drops rejected

### Visual Verification

Use `tauri-visual-test` skill for:
- [ ] Archive badge appearance
- [ ] Archived task opacity and styling
- [ ] "Show archived" toggle in header
- [ ] Edit mode form layout
- [ ] Status dropdown styling
- [ ] Inline quick-add ghost card
- [ ] Search bar layout
- [ ] Empty search state
- [ ] Context menu appearance
- [ ] Loading spinners at column bottom

---

## Design System Requirements

**MUST read `specs/DESIGN.md` before starting any UI work.**

**MUST invoke `/tailwind-v4-shadcn` skill before working on styling.**

### Key Design Principles

- **Warm orange accent** (`#ff6b35` / `--accent-primary`) - NOT purple/blue
- **SF Pro font** - NOT Inter
- **Layered shadows** for depth
- **5% accent rule** - use sparingly
- Use shadcn/ui components from `src/components/ui/`
- Use Lucide icons - NOT inline SVGs

### Component Styling

**Archive Badge**:
- Background: `--bg-subtle` (warm gray)
- Text: `--text-muted`
- Icon: Archive (Lucide)
- Position: Top-right corner with absolute positioning
- Shadow: `--shadow-sm`

**Ghost Card** (Inline Quick-Add):
- Border: `2px dashed` with `--border-subtle`
- Background: transparent
- Hover: border becomes `--accent-primary` at 30% opacity
- Transition: smooth border color change

**Search Bar**:
- Background: `--bg-base`
- Border: `1px solid --border-default`
- Shadow: `--shadow-md` (layered shadow for depth)
- Input: `--text-default` with placeholder `--text-muted`
- Close button: Ghost button style with hover effect

**Status Dropdown**:
- Use shadcn `DropdownMenu` component
- Options styled with status colors (from StatusBadge)
- Selected option has checkmark icon

**Context Menu**:
- Use shadcn `ContextMenu` component
- Background: `--bg-elevated` (higher than cards)
- Shadow: `--shadow-lg` (prominent depth)
- Items: hover state with `--bg-subtle`
- Destructive items (Delete) use `--text-destructive`

---

## Implementation Order

### Phase A: Archive System (Backend + Frontend)
**Estimated Tasks**: 8-10 atomic tasks

1. **Backend: Add archived_at field and migration**
   - Add `archived_at: Option<DateTime<Utc>>` to Task entity
   - Create migration: `ALTER TABLE tasks ADD COLUMN archived_at TEXT; CREATE INDEX ...`
   - Update Task serialization/deserialization

2. **Backend: Implement archive repository methods**
   - Add trait methods: `archive()`, `restore()`, `get_archived_count()`
   - Implement in SqliteTaskRepository
   - Implement in MemoryTaskRepository (for tests)

3. **Backend: Add archive Tauri commands**
   - `archive_task` command
   - `restore_task` command
   - `permanently_delete_task` command (with archived check)
   - `get_archived_count` command

4. **Backend: Update list_tasks for filtering**
   - Add `include_archived` parameter
   - Default to excluding archived tasks
   - Update repository methods to filter by `archived_at IS NULL`

5. **Backend: Write tests for archive system**
   - Test archive sets timestamp
   - Test restore clears timestamp
   - Test permanent delete requires archived
   - Test list filtering

6. **Frontend: Add archivedAt to Task type**
   - Update TaskSchema with `archivedAt: z.string().datetime({ offset: true }).nullable()`
   - Update CreateTask/UpdateTask schemas if needed

7. **Frontend: Add archive API bindings**
   - `api.tasks.archive(taskId)`
   - `api.tasks.restore(taskId)`
   - `api.tasks.permanentlyDelete(taskId)`
   - `api.tasks.getArchivedCount(projectId)`

8. **Frontend: Add archive mutations**
   - `archiveMutation` in useTaskMutation
   - `restoreMutation` in useTaskMutation
   - `permanentlyDeleteMutation` in useTaskMutation
   - Add query invalidation on success

9. **Frontend: Add showArchived state**
   - Add `showArchived` and `setShowArchived` to uiStore
   - Add "Show archived (N)" toggle in TaskBoard header
   - Fetch archived count and display in toggle

10. **Frontend: Implement archived task appearance**
    - Add `opacity-60` class to archived task cards
    - Gray out priority stripe
    - Add archive badge overlay (Archive icon, top-right)
    - Update TaskCard component

11. **Frontend: Add archive buttons to TaskDetailModal**
    - Archive button (non-archived tasks)
    - Restore + Delete Permanently buttons (archived tasks)
    - Hide edit button for archived tasks
    - Show archive badge at top for archived tasks
    - Add confirmation dialog for permanent delete

12. **Frontend: Write tests for archive UI**
    - Test toggle shows/hides archived tasks
    - Test archive button works
    - Test restore button works
    - Test permanent delete confirmation

### Phase B: Task Editing
**Estimated Tasks**: 6-8 atomic tasks

13. **Backend: Add get_valid_transitions command**
    - Query state machine for current status
    - Return list of valid transition target statuses
    - Include user-friendly labels

14. **Frontend: Create StatusDropdown component**
    - Fetch valid transitions from backend
    - Display as dropdown menu (shadcn DropdownMenu)
    - Trigger moveMutation on selection
    - Style with status colors

15. **Frontend: Create TaskEditForm component**
    - Similar structure to TaskCreationForm
    - Pre-populate with task data
    - Editable: title, category, description, priority
    - Non-editable: status (use dropdown), timestamps, reviews
    - Form validation with Zod

16. **Frontend: Add edit mode to TaskDetailModal**
    - Add `isEditing` state
    - Add edit button (pencil icon) in header
    - Show TaskEditForm when editing
    - Show read-only view when not editing
    - Hide edit button for system-controlled statuses
    - Hide edit button for archived tasks

17. **Frontend: Wire up edit form save**
    - Use `updateMutation` from useTaskMutation
    - Invalidate queries on success
    - Show toast notification
    - Exit edit mode on save

18. **Frontend: Add status dropdown to detail modal**
    - Add StatusDropdown component next to edit button
    - Only show for user-controlled statuses
    - Update task status via moveMutation

19. **Frontend: Write tests for edit mode**
    - Test edit button opens form
    - Test save updates task
    - Test cancel discards changes
    - Test edit hidden for system-controlled/archived
    - Test status dropdown shows valid transitions

### Phase C: Inline Quick-Add
**Estimated Tasks**: 4-6 atomic tasks

20. **Frontend: Create InlineTaskAdd component**
    - Collapsed state: ghost card with dashed border
    - Expanded state: input field with auto-focus
    - "More options" link to open full modal
    - Cancel button to collapse

21. **Frontend: Add hover state to Column**
    - Track `isHovered` state
    - Check if user is currently dragging (dnd-kit context)
    - Only show ghost card when hovered AND not dragging

22. **Frontend: Integrate InlineTaskAdd in Column**
    - Render at bottom of task list
    - Only in `draft` and `backlog` columns
    - Handle task creation with minimal fields
    - Collapse after creation

23. **Frontend: Wire up "More options" button**
    - Opens TaskCreationForm modal
    - Pre-fills with inline input value (if any)
    - Closes inline form

24. **Frontend: Write tests for inline add**
    - Test ghost card appears on hover (not during drag)
    - Test click expands form
    - Test Enter creates task
    - Test Escape cancels
    - Test "More options" opens modal

### Phase D: Search & Empty State
**Estimated Tasks**: 5-7 atomic tasks

25. **Frontend: Add boardSearchQuery to uiStore**
    - Add `boardSearchQuery: string | null`
    - Add `setBoardSearchQuery` action
    - Persist in local storage (optional)

26. **Frontend: Create TaskSearchBar component**
    - Input field with search icon
    - Close button (X)
    - Result count display
    - "Show archived" toggle integration

27. **Frontend: Add search keyboard shortcut**
    - Listen for Cmd+F / Ctrl+F in TaskBoard
    - Prevent default (browser find dialog)
    - Show search bar
    - Auto-focus input

28. **Frontend: Implement search filtering**
    - Filter tasks by title + description (case-insensitive)
    - Respect "Show archived" toggle
    - Hide columns with 0 results
    - Show match count badge in column headers

29. **Frontend: Add Escape to close search**
    - Listen for Escape key when search open
    - Clear search query
    - Restore all columns

30. **Frontend: Create EmptySearchState component**
    - Show when search returns 0 results
    - Display search term quoted
    - "Create [term]" button → opens TaskCreationForm with pre-filled title
    - "Clear Search" button
    - Tip about archived toggle (if not shown)

31. **Frontend: Write tests for search**
    - Test Cmd+F opens search
    - Test filtering by title/description
    - Test columns hide when 0 results
    - Test Escape closes search
    - Test empty state shown correctly
    - Test create from search pre-fills title

### Phase E: Infinite Scroll & Pagination
**Estimated Tasks**: 6-8 atomic tasks

32. **Backend: Add pagination parameters to list_tasks**
    - Add `offset: Option<u32>` parameter (default 0)
    - Add `limit: Option<u32>` parameter (default 20)
    - Return `TaskListResponse { tasks, total, has_more, offset }`

33. **Backend: Implement list_paginated in repository**
    - Add `list_paginated` method to TaskRepository trait
    - Implement in SqliteTaskRepository with LIMIT/OFFSET
    - Implement in MemoryTaskRepository

34. **Backend: Add count_tasks repository method**
    - Count total tasks for project (respecting archived filter)
    - Use for `has_more` calculation

35. **Backend: Write tests for pagination**
    - Test offset/limit returns correct slice
    - Test has_more correct on last page
    - Test total count accurate
    - Test pagination with status filter

36. **Frontend: Create useInfiniteTasksQuery hook**
    - Use TanStack Query's `useInfiniteQuery`
    - Query key: `['tasks', projectId, status, includeArchived]`
    - Pagination with offset increments
    - Stale time: 10 minutes
    - Cache time: 30 minutes

37. **Frontend: Update TaskBoard to use infinite query**
    - Replace `useTasks` with `useInfiniteTasksQuery`
    - Flatten pages: `data?.pages.flatMap(p => p.tasks)`
    - Pass flattened tasks to columns

38. **Frontend: Add intersection observer to Column**
    - Observe bottom of task list
    - Call `fetchNextPage()` when visible and `hasNextPage`
    - Show loading spinner at bottom during fetch

39. **Frontend: Handle cache invalidation**
    - Invalidate on create/update/delete/archive/restore mutations
    - TanStack Query handles refetch automatically

40. **Frontend: Write tests for infinite scroll**
    - Test initial load fetches 20 tasks
    - Test scroll loads next page
    - Test loading spinner shows
    - Test no duplicate tasks
    - Test cache invalidation works

### Phase F: Context Menu & Keyboard Shortcuts
**Estimated Tasks**: 4-6 atomic tasks

41. **Frontend: Create TaskCardContextMenu component**
    - Use shadcn `ContextMenu` component
    - Wrap TaskCard with ContextMenuTrigger
    - Menu items vary by status (use switch/case)
    - Style with design system tokens

42. **Frontend: Implement context menu actions**
    - View: opens TaskDetailModal
    - Edit: opens TaskDetailModal in edit mode
    - Archive: calls archiveMutation
    - Restore: calls restoreMutation (archived tasks)
    - Delete Permanently: shows confirmation, calls permanentlyDeleteMutation
    - Cancel/Block/Unblock: calls moveMutation with target status

43. **Frontend: Add keyboard shortcuts to TaskBoard**
    - Cmd+N / Ctrl+N: opens TaskCreationForm modal
    - Cmd+F / Ctrl+F: opens search bar (already done in Phase D)
    - Escape: closes search (already done in Phase D)
    - Ignore shortcuts when typing in input/textarea

44. **Frontend: Write tests for context menu**
    - Test right-click opens menu
    - Test menu items correct per status
    - Test actions trigger correct mutations
    - Test keyboard navigation (shadcn handles)

45. **Frontend: Write tests for keyboard shortcuts**
    - Test Cmd+N opens modal
    - Test shortcuts ignored in inputs

### Phase G: Enhanced Drag-Drop Restrictions
**Estimated Tasks**: 3-4 atomic tasks

46. **Frontend: Add isDraggable logic to TaskCard**
    - Calculate based on status
    - System-controlled statuses not draggable
    - Apply visual treatment: `opacity-75`, `cursor-default`
    - Add tooltip on hover

47. **Frontend: Update drag-drop validation**
    - Use state machine's `canTransitionTo()` for validation
    - Ensure drop validation queries state machine
    - Test invalid drops rejected

48. **Frontend: Write tests for drag-drop restrictions**
    - Test system-controlled tasks not draggable
    - Test visual treatment applied
    - Test tooltip shows
    - Test validation uses state machine

### Phase H: Integration & Polish
**Estimated Tasks**: 3-5 atomic tasks

49. **Integration: Test full workflow**
    - Create task via inline add
    - Edit task via detail modal
    - Archive task via context menu
    - Search for task
    - Restore archived task
    - Test infinite scroll with many tasks
    - Test drag-drop restrictions

50. **Visual verification with tauri-visual-test**
    - Capture screenshots of all new components
    - Verify design system compliance
    - Check spacing, colors, shadows, typography

51. **Performance testing**
    - Test with 1000+ tasks
    - Verify infinite scroll performance
    - Check search filter performance (client-side)
    - Monitor memory usage with cached pages

52. **Documentation**
    - Update CLAUDE.md with new components
    - Document new API commands in src-tauri/CLAUDE.md
    - Add examples for new patterns

---

## Files to Create

### Backend (Rust)

| File | Purpose |
|------|---------|
| `src-tauri/src/infrastructure/sqlite/migrations/008_add_archived_at.sql` | Add archived_at column and index |

### Frontend (React)

| File | Purpose |
|------|---------|
| `src/components/tasks/TaskEditForm.tsx` | Edit form for TaskDetailModal |
| `src/components/tasks/StatusDropdown.tsx` | Dropdown showing valid status transitions |
| `src/components/tasks/InlineTaskAdd.tsx` | Ghost card for quick task creation |
| `src/components/tasks/TaskSearchBar.tsx` | Search bar component |
| `src/components/tasks/EmptySearchState.tsx` | Creative empty state for search |
| `src/components/tasks/TaskCardContextMenu.tsx` | Right-click context menu |
| `src/hooks/useInfiniteTasksQuery.ts` | Infinite scroll query hook |
| `src/lib/statusTransitions.ts` | Utility for valid transition checks (optional if backend provides) |

### Tests

| File | Purpose |
|------|---------|
| `src-tauri/src/commands/task_commands.test.rs` | Tests for archive/pagination commands |
| `src-tauri/src/domain/repositories/task_repository.test.rs` | Tests for archive/pagination methods |
| `src/components/tasks/TaskEditForm.test.tsx` | Tests for edit form |
| `src/components/tasks/InlineTaskAdd.test.tsx` | Tests for inline add |
| `src/components/tasks/TaskSearchBar.test.tsx` | Tests for search |
| `src/components/tasks/TaskCardContextMenu.test.tsx` | Tests for context menu |
| `src/hooks/useInfiniteTasksQuery.test.ts` | Tests for infinite scroll hook |

---

## Files to Modify

### Backend (Rust)

| File | Changes |
|------|---------|
| `src-tauri/src/domain/entities/task.rs` | Add `archived_at: Option<DateTime<Utc>>` field |
| `src-tauri/src/commands/task_commands.rs` | Add archive/restore/permanently_delete/get_archived_count/get_valid_transitions commands; update list_tasks with pagination |
| `src-tauri/src/domain/repositories/task_repository.rs` | Add archive, restore, get_archived_count, list_paginated, count_tasks methods to trait |
| `src-tauri/src/infrastructure/sqlite/task_repository_impl.rs` | Implement new methods in SqliteTaskRepository |
| `src-tauri/src/infrastructure/memory/task_repository_impl.rs` | Implement new methods in MemoryTaskRepository |

### Frontend (React)

| File | Changes |
|------|---------|
| `src/types/task.ts` | Add `archivedAt: z.string().datetime({ offset: true }).nullable()` to TaskSchema |
| `src/lib/tauri.ts` | Add archive/restore/permanentlyDelete/getArchivedCount/getValidTransitions API bindings; update list with pagination params |
| `src/hooks/useTaskMutation.ts` | Add archiveMutation, restoreMutation, permanentlyDeleteMutation |
| `src/stores/uiStore.ts` | Add showArchived, setShowArchived, boardSearchQuery, setBoardSearchQuery |
| `src/components/tasks/TaskBoard/TaskBoard.tsx` | Add search bar, keyboard listeners, infinite scroll integration, "Show archived" toggle |
| `src/components/tasks/TaskBoard/Column.tsx` | Add hover state, InlineTaskAdd integration, intersection observer for infinite scroll, loading spinner |
| `src/components/tasks/TaskBoard/TaskCard.tsx` | Add isDraggable logic, muted appearance, tooltip, wrap with TaskCardContextMenu |
| `src/components/tasks/TaskDetailModal.tsx` | Add edit mode toggle, TaskEditForm integration, StatusDropdown, archive/restore buttons, archived badge |

---

## Success Criteria

Phase 18 is complete when:

- ✅ All tasks pass TDD tests (backend and frontend)
- ✅ Archive system works: tasks can be archived, restored, and permanently deleted
- ✅ Edit mode works: tasks can be edited in detail modal with valid status transitions
- ✅ Inline quick-add works: ghost card appears on column hover, creates tasks on Enter
- ✅ Search works: Cmd+F opens search, filters by title/description, shows empty state
- ✅ Infinite scroll works: columns load 20 tasks at a time, load more on scroll
- ✅ Context menu works: right-click shows menu with actions appropriate to status
- ✅ Keyboard shortcuts work: Cmd+N creates task, Cmd+F searches, Escape closes search
- ✅ Drag-drop restrictions enforced: system-controlled tasks not draggable, visual feedback
- ✅ All components follow design system (specs/DESIGN.md)
- ✅ Visual verification passes (tauri-visual-test)
- ✅ `cargo clippy --all-targets --all-features -- -D warnings` passes
- ✅ `npm run lint` passes
- ✅ Performance acceptable with 1000+ tasks

---

## Dependencies

**Backend**:
- chrono (for DateTime) - already in project
- axum, tokio (for HTTP server) - already in project
- statig (for state machine) - already in project

**Frontend**:
- @tanstack/react-query (for infinite queries) - already in project
- @dnd-kit/* (for drag-drop) - already in project
- shadcn/ui components (ContextMenu, DropdownMenu, etc.) - already in project
- Lucide React (icons) - already in project
- Zod (validation) - already in project

**No new dependencies required.**

---

## Future Enhancements (Out of Scope)

- Server-side search for very large datasets
- Bulk operations (archive multiple, restore multiple)
- Advanced filters (by category, priority, date range)
- Search highlighting (matching text in cards)
- Virtualization for columns with 500+ tasks
- Export archived tasks to file
- Archive retention policies (auto-delete after X days)

---

## Related Documentation

- Master plan: `specs/plan.md`
- Design system: `specs/DESIGN.md`
- Frontend patterns: `src/CLAUDE.md`
- Backend patterns: `src-tauri/CLAUDE.md`
- Original plan: `specs/plans/task-crud-archive-search.md`

---

## Notes

- This phase builds on solid foundation from Phases 1-11
- No breaking changes to existing task system
- Archive is soft-delete (preserves data for potential restore)
- Status transitions always use TransitionHandler (state machine integration)
- Client-side search for performance (server-side possible in future)
- Infinite scroll with aggressive caching (10 min stale time) for local-first performance
- All UI follows design system (warm orange accent, SF Pro font, layered shadows)
- TDD mandatory for all new backend logic
- Vitest tests mandatory for all new frontend components
