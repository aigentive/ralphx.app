# RalphX - Phase 18: Task CRUD, Archive & Search

## Overview

This phase completes the task management system with full CRUD operations, archive system (soft delete), search capabilities, infinite scroll pagination, and enhanced UI interactions.

**Reference Plan:**
- `specs/plans/task-crud-archive-search.md` - Complete implementation plan with architecture, components, and detailed specifications

## Goals

1. Add archive system (soft delete with restore and permanent delete)
2. Enable task editing in TaskDetailModal with status transition dropdown
3. Add inline quick-add ghost card on Kanban columns
4. Implement Cmd+F search with **server-side search endpoint** and column filtering
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

### Phase 5 (Frontend Core) - Required

| Dependency | Why Needed |
|------------|------------|
| uiStore with openModal | Modal system for TaskCreationForm and TaskDetailModal |
| Tauri event listeners | Real-time updates for archive/restore events |

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
      "  - Update serialization/deserialization",
      "Create migration file src-tauri/src/infrastructure/sqlite/migrations/XXX_add_archived_at.sql:",
      "  - ALTER TABLE tasks ADD COLUMN archived_at TEXT;",
      "  - CREATE INDEX idx_tasks_archived ON tasks(project_id, archived_at);",
      "Update SqliteTaskRepository to handle archived_at in CRUD operations",
      "Update MemoryTaskRepository for tests",
      "Run cargo test",
      "Commit: feat(task): add archived_at field for soft delete"
    ],
    "passes": true
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
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add archive Tauri commands with event emission",
    "plan_section": "Part 1: Archive System - New Tauri Commands",
    "steps": [
      "Read specs/plans/task-crud-archive-search.md section 'Part 1: Archive System'",
      "Add to src-tauri/src/commands/task_commands.rs:",
      "  - archive_task(task_id: String) -> Result<Task, AppError>",
      "  - restore_task(task_id: String) -> Result<Task, AppError>",
      "  - permanently_delete_task(task_id: String) -> Result<(), AppError> (only if archived)",
      "  - get_archived_count(project_id: String) -> Result<u32, AppError>",
      "Emit Tauri events for real-time UI updates:",
      "  - archive_task emits 'task:archived' with { task_id, project_id }",
      "  - restore_task emits 'task:restored' with { task_id, project_id }",
      "  - permanently_delete_task emits 'task:deleted' with { task_id, project_id }",
      "Register commands in lib.rs invoke_handler",
      "Write unit tests",
      "Run cargo test",
      "Commit: feat(commands): add archive/restore/permanently_delete commands with events"
    ],
    "passes": true
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
      "Implement in SqliteTaskRepository with LIMIT/OFFSET and ORDER BY created_at DESC",
      "Implement in MemoryTaskRepository",
      "Write tests for pagination edge cases (empty, last page, offset beyond total)",
      "Run cargo test",
      "Commit: feat(commands): add pagination support to list_tasks"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add server-side search_tasks command",
    "plan_section": "Part 4: Search - Server-Side Search",
    "steps": [
      "Add search_tasks command to src-tauri/src/commands/task_commands.rs:",
      "  - search_tasks(project_id: String, query: String, include_archived: Option<bool>) -> Result<Vec<Task>, AppError>",
      "  - Search in title AND description (case-insensitive)",
      "  - Use SQL LIKE '%query%' or SQLite FTS if available",
      "  - Return all matching tasks (no pagination for search - results should be small)",
      "Add search method to TaskRepository trait:",
      "  - search(&self, project_id: &str, query: &str, include_archived: bool) -> Result<Vec<Task>, AppError>",
      "Implement in SqliteTaskRepository with parameterized query to prevent SQL injection",
      "Implement in MemoryTaskRepository with filter",
      "Register command in lib.rs",
      "Write unit tests (search by title, search by description, case-insensitive, no results)",
      "Run cargo test",
      "Commit: feat(commands): add search_tasks command for server-side search"
    ],
    "passes": true
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
      "  - Map to user-friendly labels (e.g., 'ready' -> 'Ready for Work', 'cancelled' -> 'Cancel')",
      "Register command in lib.rs",
      "Write unit tests for each status showing correct transitions",
      "Run cargo test",
      "Commit: feat(commands): add get_valid_transitions for status dropdown"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add TaskListResponse and archivedAt types",
    "plan_section": "Part 1 & Part 5: Frontend Types",
    "steps": [
      "Update src/types/task.ts:",
      "  - Add archivedAt: z.string().datetime({ offset: true }).nullable() to TaskSchema",
      "  - Add TaskListResponseSchema = z.object({",
      "      tasks: z.array(TaskSchema),",
      "      total: z.number(),",
      "      hasMore: z.boolean(),",
      "      offset: z.number()",
      "    })",
      "  - Add StatusTransitionSchema = z.object({ status: z.string(), label: z.string() })",
      "  - Export TaskListResponse and StatusTransition types",
      "Run npm run typecheck",
      "Commit: feat(types): add TaskListResponse, StatusTransition, and archivedAt"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add archive and search API bindings",
    "plan_section": "Part 1: Archive System - Frontend Changes",
    "steps": [
      "Update src/lib/tauri.ts api.tasks:",
      "  - Add archive: (taskId: string) => invoke('archive_task', { taskId })",
      "  - Add restore: (taskId: string) => invoke('restore_task', { taskId })",
      "  - Add permanentlyDelete: (taskId: string) => invoke('permanently_delete_task', { taskId })",
      "  - Add getArchivedCount: (projectId: string) => invoke('get_archived_count', { projectId })",
      "  - Add getValidTransitions: (taskId: string) => invoke('get_valid_transitions', { taskId })",
      "  - Add search: (projectId: string, query: string, includeArchived?: boolean) => invoke('search_tasks', {...})",
      "  - Update list signature: (params: { projectId, status?, offset?, limit?, includeArchived? }) => invoke('list_tasks', params)",
      "Run npm run typecheck",
      "Commit: feat(api): add archive, search, and pagination API bindings"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add archive mutations with loading states and error handling",
    "plan_section": "Part 1: Archive System - Mutations",
    "steps": [
      "Update src/hooks/useTaskMutation.ts:",
      "  - Add archiveMutation using api.tasks.archive",
      "  - Add restoreMutation using api.tasks.restore",
      "  - Add permanentlyDeleteMutation using api.tasks.permanentlyDelete",
      "  - Each mutation:",
      "    - Invalidates ['tasks'] and ['archived-count'] queries on success",
      "    - Shows toast notification on success (e.g., 'Task archived')",
      "    - Shows error toast on failure with error message",
      "  - Return isArchiving, isRestoring, isPermanentlyDeleting loading states",
      "Run npm run typecheck",
      "Commit: feat(hooks): add archive mutations with loading states and error handling"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add showArchived and boardSearchQuery to uiStore",
    "plan_section": "Part 1: Archive System - UI State and Part 4: Search",
    "steps": [
      "Update src/stores/uiStore.ts:",
      "  - Add showArchived: boolean (default false)",
      "  - Add setShowArchived: (show: boolean) => void",
      "  - Add boardSearchQuery: string | null (default null)",
      "  - Add setBoardSearchQuery: (query: string | null) => void",
      "  - Add isSearching: boolean (default false) - tracks if search request is in flight",
      "  - Add setIsSearching: (searching: boolean) => void",
      "Run npm run typecheck",
      "Commit: feat(stores): add showArchived, boardSearchQuery, and isSearching to uiStore"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create StatusDropdown component",
    "plan_section": "Part 2: Task Editing - Status Dropdown",
    "steps": [
      "Create src/components/tasks/StatusDropdown.tsx:",
      "  - Props: taskId, currentStatus, onTransition, disabled?",
      "  - Fetch valid transitions via useQuery(['valid-transitions', taskId], () => api.tasks.getValidTransitions(taskId))",
      "  - Use shadcn DropdownMenu component",
      "  - Show loading spinner while fetching transitions",
      "  - Style options with status colors (reuse StatusBadge color mapping)",
      "  - On selection: call onTransition callback (parent handles mutation)",
      "  - Disable dropdown during transition (use disabled prop)",
      "Create StatusDropdown.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add StatusDropdown for valid transitions"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create TaskEditForm component",
    "plan_section": "Part 2: Task Editing - Edit Mode",
    "steps": [
      "Create src/components/tasks/TaskEditForm.tsx:",
      "  - Props: task, onSave, onCancel, isSaving",
      "  - Similar structure to TaskCreationForm",
      "  - Pre-populate with task data (title, category, description, priority)",
      "  - Form validation with Zod schema (reuse UpdateTaskSchema or create EditTaskSchema)",
      "  - onSave callback receives edited data (parent handles mutation)",
      "  - Cancel button calls onCancel",
      "  - Disable form controls and show spinner when isSaving",
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
      "Update src/components/tasks/TaskDetailModal.tsx:",
      "  - Add isEditing state (default false)",
      "  - Add edit button (Pencil icon from Lucide) in header",
      "    - Only visible for non-archived AND non-system-controlled tasks",
      "    - System-controlled: executing, execution_done, qa_*, pending_review, revision_needed",
      "  - Toggle between read-only view and TaskEditForm based on isEditing",
      "  - Add StatusDropdown next to edit button (only for user-controlled statuses)",
      "  - On StatusDropdown selection: call moveMutation, show loading state",
      "  - On TaskEditForm save: call updateMutation, exit edit mode on success",
      "  - On TaskEditForm cancel: exit edit mode, discard changes",
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
      "Update src/components/tasks/TaskDetailModal.tsx:",
      "  - For non-archived tasks: Add Archive button (Archive icon from Lucide)",
      "    - On click: call archiveMutation, show loading spinner, close modal on success",
      "  - For archived tasks:",
      "    - Show archive badge at top (orange background, 'Archived' text)",
      "    - Hide edit button and StatusDropdown",
      "    - Show Restore button (RotateCcw icon)",
      "      - On click: call restoreMutation, show loading, close modal on success",
      "    - Show Delete Permanently button (Trash icon, text-destructive)",
      "      - On click: show shadcn AlertDialog confirmation",
      "      - Confirm: call permanentlyDeleteMutation, close both dialog and modal on success",
      "  - Use isArchiving, isRestoring, isPermanentlyDeleting for button loading states",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(modal): add archive/restore/delete buttons to TaskDetailModal"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create useInfiniteTasksQuery hook",
    "plan_section": "Part 5: Infinite Scroll - Frontend Implementation",
    "steps": [
      "Create src/hooks/useInfiniteTasksQuery.ts:",
      "  - Props: { projectId, status?, includeArchived? }",
      "  - Use TanStack Query's useInfiniteQuery",
      "  - queryKey: ['tasks', 'infinite', projectId, status, includeArchived]",
      "  - queryFn: call api.tasks.list with offset from pageParam (default 0), limit 20",
      "  - getNextPageParam: (lastPage) => lastPage.hasMore ? lastPage.offset + 20 : undefined",
      "  - staleTime: 10 * 60 * 1000 (10 minutes)",
      "  - gcTime: 30 * 60 * 1000 (30 minutes)",
      "  - Return: { data, fetchNextPage, hasNextPage, isFetchingNextPage, isLoading, isError, error }",
      "  - Helper: flattenPages(data) returns Task[] from all pages",
      "Create useInfiniteTasksQuery.test.ts",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(hooks): add useInfiniteTasksQuery for pagination"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create useTaskSearch hook",
    "plan_section": "Part 4: Search - Frontend Implementation",
    "steps": [
      "Create src/hooks/useTaskSearch.ts:",
      "  - Props: { projectId, query: string | null, includeArchived? }",
      "  - Use TanStack Query's useQuery",
      "  - queryKey: ['tasks', 'search', projectId, query, includeArchived]",
      "  - queryFn: call api.tasks.search(projectId, query, includeArchived)",
      "  - enabled: query !== null && query.length >= 2 (min 2 chars to search)",
      "  - staleTime: 30 * 1000 (30 seconds - search results change frequently)",
      "  - Return: { data: Task[], isLoading, isError }",
      "Create useTaskSearch.test.ts",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(hooks): add useTaskSearch for server-side search"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add infinite scroll orchestration to TaskBoard",
    "plan_section": "Part 5: Infinite Scroll - TaskBoard Orchestration",
    "steps": [
      "Update src/components/tasks/TaskBoard/TaskBoard.tsx:",
      "  - For each column, call useInfiniteTasksQuery with column's mapped status",
      "  - Example: const backlogQuery = useInfiniteTasksQuery({ projectId, status: 'backlog', includeArchived: showArchived })",
      "  - Create a map of columnId -> query result",
      "  - Pass to each Column: tasks, fetchNextPage, hasNextPage, isFetchingNextPage, isLoading",
      "  - Handle loading state: show skeleton cards while initial load",
      "  - Handle error state: show error message with retry button",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(board): orchestrate infinite scroll queries per column"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add infinite scroll to Column component",
    "plan_section": "Part 5: Infinite Scroll - Column Changes",
    "steps": [
      "Update src/components/tasks/TaskBoard/Column.tsx:",
      "  - Accept props: tasks, fetchNextPage, hasNextPage, isFetchingNextPage, isLoading",
      "  - Add ref for scroll container",
      "  - Use IntersectionObserver on a sentinel element at bottom of task list",
      "  - When sentinel is visible AND hasNextPage AND NOT isFetchingNextPage: call fetchNextPage()",
      "  - Show loading spinner (Loader2 icon, animate-spin) at bottom when isFetchingNextPage",
      "  - Show skeleton cards when isLoading (initial load)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(column): add infinite scroll with intersection observer"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add Show archived toggle to TaskBoard header",
    "plan_section": "Part 1: Archive System - Board Header Toggle",
    "steps": [
      "Update src/components/tasks/TaskBoard/TaskBoard.tsx:",
      "  - Fetch archived count via useQuery(['archived-count', projectId], () => api.tasks.getArchivedCount(projectId))",
      "  - Add toggle button in header (only visible when archivedCount > 0)",
      "  - Use shadcn Toggle component with pressed={showArchived}",
      "  - Display: Archive icon + 'Show archived (N)'",
      "  - onPressedChange: call setShowArchived from uiStore",
      "  - When showArchived changes, infinite queries refetch with new includeArchived param",
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
      "Update src/components/tasks/TaskBoard/TaskCard.tsx:",
      "  - Check if task.archivedAt is set",
      "  - If archived:",
      "    - Add opacity-60 class to card",
      "    - Gray out priority stripe (use bg-neutral-400 instead of priority color)",
      "    - Add small archive badge overlay:",
      "      - Position: absolute, top-2, right-2",
      "      - Style: bg-neutral-200 rounded-full p-1",
      "      - Content: Archive icon (w-3 h-3) from Lucide",
      "  - Click still opens TaskDetailModal via openModal('task-detail', { taskId: task.id })",
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
      "Create src/components/tasks/InlineTaskAdd.tsx:",
      "  - Props: projectId, columnId, onCreated?",
      "  - State: isExpanded (default false), title (string)",
      "  - Collapsed state (isExpanded=false):",
      "    - Dashed border card: border-2 border-dashed border-muted hover:border-primary/30",
      "    - Content: Plus icon + '+ Add task' text (text-muted)",
      "    - onClick: setIsExpanded(true)",
      "  - Expanded state (isExpanded=true):",
      "    - Input field with auto-focus (useEffect with ref.focus())",
      "    - onKeyDown: Enter -> create task, Escape -> collapse",
      "    - 'More options' text button: opens TaskCreationForm modal via openModal('task-create', { projectId, defaultTitle: title, defaultStatus: columnId })",
      "    - 'Cancel' text button: setIsExpanded(false), clear title",
      "  - On Enter: call createMutation with { projectId, title, internalStatus: columnId }",
      "    - On success: collapse, clear title, call onCreated callback",
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
      "Update src/components/tasks/TaskBoard/Column.tsx:",
      "  - Accept prop: columnId",
      "  - Add isHovered state (default false)",
      "  - Add onMouseEnter={() => setIsHovered(true)} and onMouseLeave={() => setIsHovered(false)}",
      "  - Get isDragging from dnd-kit's useDndContext() hook",
      "  - Render InlineTaskAdd at bottom of task list when ALL conditions met:",
      "    - isHovered === true",
      "    - columnId is 'draft' OR 'backlog' (user-addable columns)",
      "    - isDragging === false (from useDndContext)",
      "  - Pass projectId and columnId to InlineTaskAdd",
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
      "Create src/components/tasks/TaskSearchBar.tsx:",
      "  - Props: value, onChange, onClose, resultCount, isSearching",
      "  - Container: flex items-center gap-2, bg-background, border, rounded-lg, shadow-md, p-2",
      "  - Search icon (Lucide Search) on left",
      "  - Input field: flex-1, border-none, focus:ring-0, placeholder='Search tasks...'",
      "    - Auto-focus on mount via useEffect with inputRef.focus()",
      "    - value and onChange controlled",
      "  - Loading spinner (Loader2 animate-spin) shown when isSearching",
      "  - Result count: 'N tasks found' or 'No results' (text-muted, text-sm)",
      "  - Close button: X icon, onClick calls onClose",
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
      "Create src/components/tasks/EmptySearchState.tsx:",
      "  - Props: searchQuery, onCreateTask, onClearSearch, showArchived",
      "  - Container: flex flex-col items-center justify-center py-16 text-center",
      "  - FileText icon (Lucide, w-12 h-12, text-muted)",
      "  - Heading: 'No tasks match \"{searchQuery}\"' (text-lg font-medium)",
      "  - Subheading: 'Should this be a task?' (text-muted)",
      "  - Buttons row: flex gap-3 mt-4",
      "    - Primary: '+ Create \"{searchQuery}\"' button (variant default)",
      "      - onClick: onCreateTask (parent will open modal with pre-filled title)",
      "    - Secondary: 'Clear Search' button (variant outline)",
      "      - onClick: onClearSearch",
      "  - Tip (only if showArchived === false):",
      "    - Container: mt-6 p-3 bg-muted/50 rounded-lg",
      "    - Content: Lightbulb icon + 'Tip: Enable \"Show archived\" to search old tasks'",
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
      "Update src/components/tasks/TaskBoard/TaskBoard.tsx:",
      "  - Add searchOpen state (default false)",
      "  - Get boardSearchQuery and setBoardSearchQuery from uiStore",
      "  - When searchOpen AND boardSearchQuery has 2+ chars:",
      "    - Call useTaskSearch({ projectId, query: boardSearchQuery, includeArchived: showArchived })",
      "    - Replace infinite scroll data with search results",
      "    - Group search results by their internalStatus to distribute to columns",
      "  - Render TaskSearchBar at top when searchOpen:",
      "    - value={boardSearchQuery}, onChange={setBoardSearchQuery}",
      "    - onClose={() => { setSearchOpen(false); setBoardSearchQuery(null); }}",
      "    - resultCount={searchResults?.length ?? 0}",
      "    - isSearching={isSearchLoading}",
      "  - During search: hide columns with 0 matching tasks",
      "  - Add match count badge to Column header: '(N)' next to task count when searching",
      "  - Show EmptySearchState when search returns 0 results:",
      "    - onCreateTask: openModal('task-create', { projectId, defaultTitle: boardSearchQuery })",
      "    - onClearSearch: setBoardSearchQuery(null)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(board): integrate search bar with server-side search"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add keyboard shortcuts to TaskBoard",
    "plan_section": "Part 7: Keyboard Shortcuts",
    "steps": [
      "Update src/components/tasks/TaskBoard/TaskBoard.tsx:",
      "  - Add useEffect with keydown listener on window",
      "  - Guard: ignore if activeElement is input, textarea, or contenteditable",
      "  - Cmd+N / Ctrl+N: e.preventDefault(), openModal('task-create', { projectId })",
      "  - Cmd+F / Ctrl+F: e.preventDefault(), setSearchOpen(true)",
      "  - Escape: if searchOpen, setSearchOpen(false), setBoardSearchQuery(null)",
      "  - Cleanup: remove event listener on unmount",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(board): add keyboard shortcuts (Cmd+N, Cmd+F, Escape)"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create TaskCardContextMenu component",
    "plan_section": "Part 6: Context Menu",
    "steps": [
      "Create src/components/tasks/TaskCardContextMenu.tsx:",
      "  - Props: task, children, onViewDetails, onEdit, onArchive, onRestore, onPermanentDelete, onStatusChange",
      "  - Use shadcn ContextMenu, ContextMenuTrigger, ContextMenuContent, ContextMenuItem, ContextMenuSeparator",
      "  - Wrap children with ContextMenuTrigger (asChild)",
      "  - Menu items (with Lucide icons):",
      "    - 'View Details' (Eye icon) - always shown, calls onViewDetails",
      "    - 'Edit' (Pencil icon) - if canEdit (not archived, not system-controlled), calls onEdit",
      "    - Separator",
      "    - If NOT archived:",
      "      - 'Archive' (Archive icon) - calls onArchive",
      "      - Status actions based on current status (Cancel, Block, Unblock, etc.) - call onStatusChange",
      "    - If archived:",
      "      - 'Restore' (RotateCcw icon) - calls onRestore",
      "      - 'Delete Permanently' (Trash icon, className='text-destructive') - calls onPermanentDelete",
      "Create TaskCardContextMenu.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add TaskCardContextMenu"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Wrap TaskCard with context menu and add handlers",
    "plan_section": "Part 6: Context Menu - Integration",
    "steps": [
      "Update src/components/tasks/TaskBoard/TaskCard.tsx:",
      "  - Import TaskCardContextMenu",
      "  - Import openModal from uiStore",
      "  - Import archiveMutation, restoreMutation, permanentlyDeleteMutation, moveMutation from useTaskMutation",
      "  - Wrap the card content with TaskCardContextMenu",
      "  - Pass handlers:",
      "    - onViewDetails: () => openModal('task-detail', { taskId: task.id })",
      "    - onEdit: () => openModal('task-detail', { taskId: task.id, startInEditMode: true })",
      "    - onArchive: () => archiveMutation.mutate(task.id)",
      "    - onRestore: () => restoreMutation.mutate(task.id)",
      "    - onPermanentDelete: () => { show confirmation dialog, then permanentlyDeleteMutation.mutate(task.id) }",
      "    - onStatusChange: (newStatus) => moveMutation.mutate({ taskId: task.id, toStatus: newStatus })",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(card): wrap TaskCard with context menu and handlers"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add isDraggable logic to TaskCard",
    "plan_section": "Part 8: Enhanced Drag-Drop Restrictions",
    "steps": [
      "Update src/components/tasks/TaskBoard/TaskCard.tsx:",
      "  - Add isDraggable useMemo based on task.internalStatus:",
      "    - Non-draggable statuses: ['executing', 'execution_done', 'qa_refining', 'qa_testing', 'qa_passed', 'qa_failed', 'pending_review', 'revision_needed']",
      "    - return !nonDraggableStatuses.includes(task.internalStatus)",
      "  - Conditionally apply dnd-kit attributes/listeners only if isDraggable:",
      "    - {...(isDraggable ? { ...attributes, ...listeners } : {})}",
      "  - Apply visual styles when NOT draggable:",
      "    - Add opacity-75 class",
      "    - Add cursor-default class (instead of cursor-grab)",
      "  - Add title attribute when NOT draggable:",
      "    - title='This task is being processed and cannot be moved manually'",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(card): add isDraggable logic for system-controlled states"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Listen for archive/restore events for real-time updates",
    "plan_section": "Part 1: Archive System - Real-time Updates",
    "steps": [
      "Update src/components/tasks/TaskBoard/TaskBoard.tsx or create a hook:",
      "  - Use Tauri's listen() to subscribe to events:",
      "    - 'task:archived' -> invalidate ['tasks'] and ['archived-count'] queries",
      "    - 'task:restored' -> invalidate ['tasks'] and ['archived-count'] queries",
      "    - 'task:deleted' -> invalidate ['tasks'] and ['archived-count'] queries",
      "  - Use useEffect with cleanup to unlisten on unmount",
      "  - This ensures board updates when tasks are archived/restored from other places (e.g., modal, agent)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(board): listen for archive/restore events for real-time updates"
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
      "  - New hooks: useInfiniteTasksQuery, useTaskSearch",
      "  - New uiStore state: showArchived, boardSearchQuery, isSearching",
      "  - Archive mutations in useTaskMutation",
      "Update src-tauri/CLAUDE.md with:",
      "  - Archive commands: archive_task, restore_task, permanently_delete_task, get_archived_count",
      "  - Search command: search_tasks",
      "  - Pagination parameters in list_tasks",
      "  - get_valid_transitions command",
      "  - Events: task:archived, task:restored, task:deleted",
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
| **Server-side search** | Finds ALL matching tasks regardless of pagination; reliable results |
| **10-minute cache stale time for infinite scroll** | Local-first app with event-driven updates; longer cache is safe and performant |
| **30-second cache for search** | Search results can change frequently; shorter cache ensures freshness |
| **Status dropdown queries state machine** | UI respects valid transitions from statig; no hardcoded transition logic in frontend |
| **Ghost card only when not dragging** | Avoids interference with dnd-kit drop zone detection |
| **System-controlled states non-draggable** | Prevents user from interrupting in-progress execution |
| **Event emission for archive/restore** | Enables real-time UI updates when tasks archived from other contexts |
| **openModal pattern for all modals** | Consistent modal management through uiStore |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] All archive repository method tests pass
- [ ] All archive command tests pass
- [ ] Pagination tests pass (edge cases: empty, last page, offset beyond total)
- [ ] Search tests pass (title, description, case-insensitive, no results, SQL injection prevention)
- [ ] get_valid_transitions tests pass for each status

### Frontend - Run `npm run test`
- [ ] StatusDropdown.test.tsx passes
- [ ] TaskEditForm.test.tsx passes
- [ ] InlineTaskAdd.test.tsx passes
- [ ] TaskSearchBar.test.tsx passes
- [ ] EmptySearchState.test.tsx passes
- [ ] TaskCardContextMenu.test.tsx passes
- [ ] useInfiniteTasksQuery.test.ts passes
- [ ] useTaskSearch.test.ts passes

### Build Verification - Run `npm run build` and `cargo build --release`
- [ ] No TypeScript errors
- [ ] No Rust compilation errors
- [ ] No lint warnings (`npm run lint` and `cargo clippy`)

### Type Verification - Run `npm run typecheck`
- [ ] TaskListResponse type works with API
- [ ] StatusTransition type works with API
- [ ] archivedAt field properly typed
