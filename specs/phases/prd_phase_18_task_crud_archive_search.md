# RalphX - Phase 18: Task CRUD, Archive & Search

## Overview

This phase completes the task management system with full CRUD operations, archive system (soft delete), search capabilities, infinite scroll pagination, and enhanced UI interactions.

**Reference Plan:**
- `specs/plans/task-crud-archive-search.md` - Complete implementation plan with architecture, components, and detailed specifications

## Goals

1. Add archive system (soft delete with restore and permanent delete)
2. Enable task editing in TaskDetailModal with status transition dropdown
3. Add inline quick-add ghost card on Kanban columns
4. Implement Cmd+F search with column filtering and creative empty state
5. Add infinite scroll pagination with TanStack Query caching
6. Add right-click context menu on task cards
7. Implement keyboard shortcuts (Cmd+N, Cmd+F, Escape)
8. Enhance drag-drop restrictions for system-controlled states

## Dependencies

### Phase 17 (Worker Artifact Context) - Required

| Dependency | Why Needed |
|------------|------------|
| Task entity with all fields | `sourceProposalId`, `planArtifactId` fields already exist |
| TaskDetailModal | Modal exists in read-only mode, needs edit mode |
| TaskBoard with dnd-kit | Kanban board with drag-drop already implemented |

### Phase 6 (Kanban UI) - Required

| Dependency | Why Needed |
|------------|------------|
| TaskCard component | Needs context menu wrapper and isDraggable logic |
| Column component | Needs hover state for inline add |
| TaskBoard component | Needs search bar and keyboard shortcuts |

### Phase 3 (State Machine) - Required

| Dependency | Why Needed |
|------------|------------|
| statig state machine | Status dropdown queries valid transitions |
| TransitionHandler | All status changes go through state machine |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/task-crud-archive-search.md`
2. Understand the architecture, data flow, and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write tests where appropriate
4. Run `npm run lint && npm run typecheck` and `cargo clippy --all-targets --all-features -- -D warnings`
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/task-crud-archive-search.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "backend",
    "description": "Add archived_at field and database migration",
    "plan_section": "Part 1: Archive System - Backend Changes",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 1: Archive System'",
      "Update src-tauri/src/domain/entities/task.rs:",
      "  - Add `archived_at: Option<DateTime<Utc>>` field to Task struct",
      "  - Update Default impl if needed",
      "Create migration file src-tauri/src/infrastructure/sqlite/migrations/XXX_add_archived_at.sql:",
      "  - ALTER TABLE tasks ADD COLUMN archived_at TEXT;",
      "  - CREATE INDEX idx_tasks_archived ON tasks(project_id, archived_at);",
      "Update SqliteTaskRepository to handle archived_at in CRUD operations",
      "Update MemoryTaskRepository for tests",
      "Run cargo test",
      "Commit: feat(task): add archived_at field for soft delete"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Add archive repository methods",
    "plan_section": "Part 1: Archive System - Repository Changes",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 1: Archive System - Repository Changes'",
      "Update src-tauri/src/domain/repositories/task_repository.rs trait:",
      "  - Add archive(&self, task_id: &str) -> Result<Task, AppError>",
      "  - Add restore(&self, task_id: &str) -> Result<Task, AppError>",
      "  - Add get_archived_count(&self, project_id: &str) -> Result<u32, AppError>",
      "  - Add get_by_project_filtered(&self, project_id: &str, include_archived: bool) -> Result<Vec<Task>, AppError>",
      "Implement in SqliteTaskRepository",
      "Implement in MemoryTaskRepository",
      "Write unit tests for archive/restore/count methods",
      "Run cargo test",
      "Commit: feat(repository): add archive methods to TaskRepository"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Add archive Tauri commands",
    "plan_section": "Part 1: Archive System - New Tauri Commands",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 1: Archive System'",
      "Add to src-tauri/src/commands/task_commands.rs:",
      "  - archive_task(task_id: String) -> Result<Task, AppError>",
      "  - restore_task(task_id: String) -> Result<Task, AppError>",
      "  - permanently_delete_task(task_id: String) -> Result<(), AppError> (only if archived)",
      "  - get_archived_count(project_id: String) -> Result<u32, AppError>",
      "Register commands in lib.rs invoke_handler",
      "Write unit tests",
      "Run cargo test",
      "Commit: feat(commands): add archive/restore/permanently_delete commands"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Add pagination to list_tasks command",
    "plan_section": "Part 5: Infinite Scroll - Backend Changes",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 5: Infinite Scroll'",
      "Update list_tasks command in src-tauri/src/commands/task_commands.rs:",
      "  - Add offset: Option<u32> parameter (default 0)",
      "  - Add limit: Option<u32> parameter (default 20)",
      "  - Add include_archived: Option<bool> parameter (default false)",
      "  - Return TaskListResponse { tasks, total, has_more, offset }",
      "Add list_paginated method to TaskRepository trait",
      "Add count_tasks method to TaskRepository trait",
      "Implement in SqliteTaskRepository with LIMIT/OFFSET",
      "Implement in MemoryTaskRepository",
      "Write tests for pagination edge cases",
      "Run cargo test",
      "Commit: feat(commands): add pagination support to list_tasks"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Add get_valid_transitions command",
    "plan_section": "Part 2: Task Editing - Status Dropdown",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 2: Task Editing - Status Dropdown'",
      "Add get_valid_transitions command to task_commands.rs:",
      "  - get_valid_transitions(task_id: String) -> Result<Vec<StatusTransition>, AppError>",
      "  - StatusTransition struct: { status: String, label: String }",
      "  - Query state machine for valid transitions from current status",
      "  - Map to user-friendly labels (e.g., 'ready' -> 'Ready for Work')",
      "Register command in lib.rs",
      "Write unit tests",
      "Run cargo test",
      "Commit: feat(commands): add get_valid_transitions for status dropdown"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add archivedAt to Task type and API bindings",
    "plan_section": "Part 1: Archive System - Frontend Changes",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 1: Archive System - Frontend Changes'",
      "Update src/types/task.ts:",
      "  - Add archivedAt: z.string().datetime({ offset: true }).nullable() to TaskSchema",
      "Update src/lib/tauri.ts api.tasks:",
      "  - Add archive: (taskId: string) => invoke('archive_task', { taskId })",
      "  - Add restore: (taskId: string) => invoke('restore_task', { taskId })",
      "  - Add permanentlyDelete: (taskId: string) => invoke('permanently_delete_task', { taskId })",
      "  - Add getArchivedCount: (projectId: string) => invoke('get_archived_count', { projectId })",
      "  - Add getValidTransitions: (taskId: string) => invoke('get_valid_transitions', { taskId })",
      "  - Update list to accept pagination params: { offset, limit, includeArchived }",
      "Run npm run typecheck",
      "Commit: feat(types): add archivedAt field and archive API bindings"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add archive mutations to useTaskMutation",
    "plan_section": "Part 1: Archive System - Mutations",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 1: Archive System - Mutations'",
      "Update src/hooks/useTaskMutation.ts:",
      "  - Add archiveMutation using api.tasks.archive",
      "  - Add restoreMutation using api.tasks.restore",
      "  - Add permanentlyDeleteMutation using api.tasks.permanentlyDelete",
      "  - Each mutation invalidates ['tasks'] and ['archived-count'] queries on success",
      "Run npm run typecheck",
      "Commit: feat(hooks): add archive/restore/permanentlyDelete mutations"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add showArchived and boardSearchQuery to uiStore",
    "plan_section": "Part 1: Archive System - UI State and Part 4: Search",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md sections 'Part 1' and 'Part 4'",
      "Update src/stores/uiStore.ts:",
      "  - Add showArchived: boolean (default false)",
      "  - Add setShowArchived: (show: boolean) => void",
      "  - Add boardSearchQuery: string | null (default null)",
      "  - Add setBoardSearchQuery: (query: string | null) => void",
      "Run npm run typecheck",
      "Commit: feat(stores): add showArchived and boardSearchQuery to uiStore"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create StatusDropdown component",
    "plan_section": "Part 2: Task Editing - Status Dropdown",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 2: Task Editing - Status Dropdown'",
      "Create src/components/tasks/StatusDropdown.tsx:",
      "  - Props: taskId, currentStatus, onTransition",
      "  - Fetch valid transitions via useQuery(['valid-transitions', taskId])",
      "  - Use shadcn DropdownMenu component",
      "  - Show loading state while fetching",
      "  - Style options with status colors (from existing StatusBadge)",
      "  - Call moveMutation on selection",
      "Create StatusDropdown.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add StatusDropdown for valid transitions"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create TaskEditForm component",
    "plan_section": "Part 2: Task Editing - Edit Mode",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 2: Task Editing'",
      "Create src/components/tasks/TaskEditForm.tsx:",
      "  - Props: task, onSave, onCancel",
      "  - Similar structure to TaskCreationForm",
      "  - Pre-populate with task data (title, category, description, priority)",
      "  - Form validation with Zod schema (UpdateTaskSchema)",
      "  - Use updateMutation from useTaskMutation on save",
      "  - Cancel button discards changes",
      "Create TaskEditForm.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add TaskEditForm for editing tasks"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add edit mode to TaskDetailModal",
    "plan_section": "Part 2: Task Editing - TaskDetailModal Changes",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 2: Task Editing'",
      "Update src/components/tasks/TaskDetailModal.tsx:",
      "  - Add isEditing state (default false)",
      "  - Add edit button (Pencil icon) in header - only for non-archived, non-system-controlled tasks",
      "  - Toggle between read-only view and TaskEditForm based on isEditing",
      "  - Add StatusDropdown next to edit button (for user-controlled statuses)",
      "  - Exit edit mode on save or cancel",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(modal): add edit mode to TaskDetailModal"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add archive buttons to TaskDetailModal",
    "plan_section": "Part 1: Archive System - TaskDetailModal",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 1: Archive System'",
      "Update src/components/tasks/TaskDetailModal.tsx:",
      "  - For non-archived tasks: Add Archive button (Archive icon)",
      "  - For archived tasks:",
      "    - Show archive badge at top",
      "    - Hide edit button",
      "    - Show Restore button (RotateCcw icon)",
      "    - Show Delete Permanently button (Trash icon, text-destructive)",
      "  - Permanent delete shows confirmation AlertDialog before deletion",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(modal): add archive/restore/delete buttons to TaskDetailModal"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add Show archived toggle to TaskBoard header",
    "plan_section": "Part 1: Archive System - Board Header Toggle",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 1: Archive System'",
      "Update src/components/tasks/TaskBoard/TaskBoard.tsx:",
      "  - Fetch archived count via useQuery(['archived-count', projectId])",
      "  - Add toggle button in header (only visible when archivedCount > 0)",
      "  - Use shadcn Toggle or Button with pressed state",
      "  - Display: 'Show archived (N)' with Archive icon",
      "  - Toggle updates showArchived in uiStore",
      "  - Pass includeArchived to task queries based on toggle state",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(board): add Show archived toggle to TaskBoard header"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add archived task appearance to TaskCard",
    "plan_section": "Part 1: Archive System - Archived Task Appearance",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 1: Archive System'",
      "Update src/components/tasks/TaskBoard/TaskCard.tsx:",
      "  - Check if task.archivedAt is set",
      "  - If archived:",
      "    - Add opacity-60 class",
      "    - Gray out priority stripe (use neutral color)",
      "    - Add small archive badge overlay (Archive icon, absolute top-right)",
      "  - Click still opens TaskDetailModal (in archived mode)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(card): add archived appearance to TaskCard"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create InlineTaskAdd component",
    "plan_section": "Part 3: Inline Quick-Add",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 3: Inline Quick-Add'",
      "Create src/components/tasks/InlineTaskAdd.tsx:",
      "  - Props: projectId, columnId, onCreated?",
      "  - Collapsed state: ghost card with dashed border, '+ Add task' text",
      "  - Expanded state: input field with auto-focus",
      "  - Enter key: create task with minimal fields (title, columnId as status), collapse",
      "  - Escape key: collapse without creating",
      "  - 'More options' link: opens TaskCreationForm modal with title pre-filled",
      "  - Use createMutation from useTaskMutation",
      "Create InlineTaskAdd.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add InlineTaskAdd ghost card"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add hover state and InlineTaskAdd to Column",
    "plan_section": "Part 3: Inline Quick-Add - Column Changes",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 3: Inline Quick-Add'",
      "Update src/components/tasks/TaskBoard/Column.tsx:",
      "  - Add isHovered state",
      "  - Add onMouseEnter/onMouseLeave handlers",
      "  - Check if user is currently dragging (from dnd-kit context)",
      "  - Render InlineTaskAdd at bottom of task list when:",
      "    - Column is hovered",
      "    - Column is 'draft' or 'backlog'",
      "    - NOT currently dragging (avoids interference with drop zones)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(column): add hover state and InlineTaskAdd integration"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create TaskSearchBar component",
    "plan_section": "Part 4: Search",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 4: Search'",
      "Create src/components/tasks/TaskSearchBar.tsx:",
      "  - Props: value, onChange, onClose, resultCount",
      "  - Input field with Search icon (Lucide)",
      "  - Close button (X icon)",
      "  - Display result count: 'N tasks found'",
      "  - Auto-focus input on mount",
      "  - Style per design system (layered shadow, proper colors)",
      "Create TaskSearchBar.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add TaskSearchBar component"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create EmptySearchState component",
    "plan_section": "Part 4: Search - Empty Search State",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 4: Search - Empty Search State'",
      "Create src/components/tasks/EmptySearchState.tsx:",
      "  - Props: searchQuery, onCreateTask, onClearSearch, showArchived",
      "  - Notepad icon (FileText from Lucide)",
      "  - Message: 'No tasks match \"[searchQuery]\"'",
      "  - 'Should this be a task?' prompt",
      "  - Primary CTA: '+ Create \"[searchQuery]\"' button",
      "  - Secondary: 'Clear Search' button",
      "  - Tip about archived toggle (only if showArchived is false)",
      "Create EmptySearchState.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add EmptySearchState message-in-a-bottle"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Integrate search into TaskBoard",
    "plan_section": "Part 4: Search - TaskBoard Changes",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 4: Search'",
      "Update src/components/tasks/TaskBoard/TaskBoard.tsx:",
      "  - Add searchOpen state",
      "  - Render TaskSearchBar at top when searchOpen",
      "  - Filter tasks client-side by title + description (case-insensitive)",
      "  - Respect showArchived toggle when filtering",
      "  - Hide columns with 0 results during search",
      "  - Show match count badge in column headers during search",
      "  - Show EmptySearchState when search returns 0 results",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(board): integrate search bar and filtering"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add keyboard shortcuts to TaskBoard",
    "plan_section": "Part 7: Keyboard Shortcuts",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 7: Keyboard Shortcuts'",
      "Update src/components/tasks/TaskBoard/TaskBoard.tsx:",
      "  - Add useEffect with keydown listener",
      "  - Cmd+N / Ctrl+N: open TaskCreationForm modal, e.preventDefault()",
      "  - Cmd+F / Ctrl+F: set searchOpen to true, e.preventDefault()",
      "  - Escape: if searchOpen, close search and clear query",
      "  - Ignore shortcuts when focus is in input/textarea",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(board): add keyboard shortcuts (Cmd+N, Cmd+F, Escape)"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create useInfiniteTasksQuery hook",
    "plan_section": "Part 5: Infinite Scroll - Frontend Implementation",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 5: Infinite Scroll'",
      "Create src/hooks/useInfiniteTasksQuery.ts:",
      "  - Props: projectId, status?, includeArchived?",
      "  - Use TanStack Query's useInfiniteQuery",
      "  - queryKey: ['tasks', projectId, status, includeArchived]",
      "  - queryFn: call api.tasks.list with offset from pageParam",
      "  - getNextPageParam: return offset + 20 if hasMore, else undefined",
      "  - staleTime: 10 * 60 * 1000 (10 minutes)",
      "  - gcTime: 30 * 60 * 1000 (30 minutes)",
      "Create useInfiniteTasksQuery.test.ts",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(hooks): add useInfiniteTasksQuery for pagination"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add infinite scroll to Column",
    "plan_section": "Part 5: Infinite Scroll - Column Changes",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 5: Infinite Scroll'",
      "Update src/components/tasks/TaskBoard/Column.tsx:",
      "  - Use intersection observer at bottom of task list",
      "  - When observer triggers and hasNextPage, call fetchNextPage()",
      "  - Show loading spinner at bottom during fetch (isFetchingNextPage)",
      "  - Flatten pages: data?.pages.flatMap(p => p.tasks)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(column): add infinite scroll with intersection observer"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create TaskCardContextMenu component",
    "plan_section": "Part 6: Context Menu",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 6: Context Menu'",
      "Create src/components/tasks/TaskCardContextMenu.tsx:",
      "  - Props: task, children",
      "  - Wrap children with shadcn ContextMenu",
      "  - Menu items vary by status (use switch/case):",
      "    - View Details (always)",
      "    - Edit (if canEdit)",
      "    - Archive (if not archived)",
      "    - Restore (if archived)",
      "    - Delete Permanently (if archived, destructive style)",
      "    - Cancel/Block/Unblock (based on current status)",
      "  - Use Lucide icons for each action",
      "Create TaskCardContextMenu.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add TaskCardContextMenu"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Wrap TaskCard with context menu",
    "plan_section": "Part 6: Context Menu - Integration",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 6: Context Menu'",
      "Update src/components/tasks/TaskBoard/TaskCard.tsx:",
      "  - Import TaskCardContextMenu",
      "  - Wrap the card content with TaskCardContextMenu",
      "  - Pass task and necessary callbacks",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(card): wrap TaskCard with context menu"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add isDraggable logic to TaskCard",
    "plan_section": "Part 8: Enhanced Drag-Drop Restrictions",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 8: Enhanced Drag-Drop Restrictions'",
      "Update src/components/tasks/TaskBoard/TaskCard.tsx:",
      "  - Add isDraggable useMemo based on task.internalStatus",
      "  - Non-draggable statuses: executing, execution_done, qa_*, pending_review, revision_needed",
      "  - If not draggable:",
      "    - Add opacity-75 class",
      "    - Set cursor-default instead of cursor-grab",
      "    - Add title attribute: 'This task is being processed and cannot be moved manually'",
      "    - Don't spread dnd-kit attributes/listeners",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(card): add isDraggable logic for system-controlled states"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Write integration tests for archive workflow",
    "plan_section": "Testing Requirements",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Testing Requirements'",
      "Write tests for archive workflow:",
      "  - Archive a task and verify it's hidden by default",
      "  - Toggle 'Show archived' and verify task appears with archived styling",
      "  - Restore a task and verify it's visible again",
      "  - Permanently delete an archived task and verify it's gone",
      "  - Verify edit button hidden for archived tasks",
      "Run npm run test",
      "Commit: test: add integration tests for archive workflow"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Write integration tests for search workflow",
    "plan_section": "Testing Requirements",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Testing Requirements'",
      "Write tests for search workflow:",
      "  - Open search with Cmd+F",
      "  - Filter by title (case-insensitive)",
      "  - Filter by description",
      "  - Verify columns with 0 results are hidden",
      "  - Verify empty state shown when no results",
      "  - Verify 'Create from search' pre-fills title",
      "  - Verify Escape closes search",
      "Run npm run test",
      "Commit: test: add integration tests for search workflow"
    ],
    "passes": false
  },
  {
    "category": "documentation",
    "description": "Update CLAUDE.md files for Phase 18",
    "plan_section": "Related Documentation",
    "steps": [
      "Update src/CLAUDE.md with:",
      "  - New components: StatusDropdown, TaskEditForm, InlineTaskAdd, TaskSearchBar, EmptySearchState, TaskCardContextMenu",
      "  - New hooks: useInfiniteTasksQuery",
      "  - New uiStore state: showArchived, boardSearchQuery",
      "Update src-tauri/CLAUDE.md with:",
      "  - Archive commands and repository methods",
      "  - Pagination parameters in list_tasks",
      "  - get_valid_transitions command",
      "Update logs/activity.md with Phase 18 completion summary",
      "Commit: docs: update documentation for Phase 18"
    ],
    "passes": false
  }
]
```

---

## Key Architecture Decisions

From the implementation plan:

| Decision | Rationale |
|----------|-----------|
| **Soft delete with `archived_at` timestamp** | Preserves data for restore, enables "Show archived" toggle |
| **Client-side search** | Tasks already loaded via infinite scroll; fast filtering without server round-trips |
| **10-minute cache stale time** | Local-first app with event-driven updates; longer cache is safe and performant |
| **Status dropdown queries state machine** | UI respects valid transitions from statig; no hardcoded transition logic in frontend |
| **Ghost card only when not dragging** | Avoids interference with dnd-kit drop zone detection |
| **System-controlled states non-draggable** | Prevents user from interrupting in-progress execution |

---

## Verification Checklist

After completing all tasks:

### Backend
- [ ] `archived_at` field added to Task entity
- [ ] Database migration creates column and index
- [ ] `archive_task` sets `archived_at` correctly
- [ ] `restore_task` clears `archived_at`
- [ ] `permanently_delete_task` only works on archived tasks
- [ ] `get_archived_count` returns correct count
- [ ] `list_tasks` excludes archived by default
- [ ] `list_tasks` includes archived when flag is true
- [ ] Pagination works with offset/limit
- [ ] `get_valid_transitions` returns correct options per status

### Frontend - Archive
- [ ] Archive button visible for active tasks
- [ ] Restore + Delete Permanently buttons for archived tasks
- [ ] Confirmation dialog for permanent delete
- [ ] "Show archived (N)" toggle in board header
- [ ] Archived tasks appear with reduced opacity + badge
- [ ] Edit button hidden for archived tasks

### Frontend - Edit
- [ ] Edit button opens TaskEditForm
- [ ] Form pre-populated with task data
- [ ] Save updates task and closes form
- [ ] Cancel discards changes
- [ ] StatusDropdown shows only valid transitions

### Frontend - Inline Add
- [ ] Ghost card appears on draft/backlog column hover (not during drag)
- [ ] Click expands to input form with auto-focus
- [ ] Enter creates task and collapses
- [ ] Escape cancels without creating
- [ ] "More options" opens full modal

### Frontend - Search
- [ ] Cmd+F / Ctrl+F opens search bar
- [ ] Filters by title + description (case-insensitive)
- [ ] Respects "Show archived" toggle
- [ ] Columns with 0 results hidden
- [ ] Match count badge in column headers
- [ ] Empty state shows "message in a bottle"
- [ ] Create from search pre-fills title
- [ ] Escape closes search

### Frontend - Infinite Scroll
- [ ] Initial load fetches 20 tasks per column
- [ ] Scroll to bottom loads next page
- [ ] Loading spinner at bottom during fetch
- [ ] No duplicate tasks
- [ ] Cache invalidation on mutations

### Frontend - Context Menu
- [ ] Right-click opens menu
- [ ] Menu items vary by status
- [ ] View/Edit/Archive/Restore/Delete actions work

### Frontend - Drag-Drop
- [ ] System-controlled tasks not draggable
- [ ] Non-draggable tasks have muted appearance + tooltip
- [ ] Invalid drops rejected

### Visual Verification
- [ ] Run tauri-visual-test for all new components
- [ ] Verify design system compliance (warm orange, SF Pro, layered shadows)
