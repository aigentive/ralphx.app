# RalphX - Phase 6: Kanban UI

## Overview

This phase implements the core Kanban board UI with drag-and-drop functionality. The TaskBoard displays tasks across 7 columns (Draft, Backlog, To-do, Planned, In Progress, In Review, Done) with full drag-drop support, visual feedback, and status badge rendering. All components follow the anti-AI-slop design system with warm orange accents and dark theme.

## Dependencies

- **Phase 5 (Frontend Core)** must be complete:
  - Zustand stores (taskStore, uiStore)
  - TanStack Query setup
  - Tauri API wrappers
  - Event system (useTaskEvents hook)
  - TypeScript types (Task, InternalStatus, WorkflowSchema)

## Scope

### Included
- TaskBoard component with DndContext
- Column component with droppable zones
- TaskCard component with badges and drag handle
- TaskBoardSkeleton loading state
- Drag-drop validation and visual feedback
- Priority indicator badges
- Review status badges (AI Approved, Human Approved, Needs Changes)
- Keyboard shortcuts (P, B, T, Delete)
- Design system tokens (CSS variables)
- Column transition constraints enforcement

### Excluded (Later Phases)
- Task detail modal/panel (Phase 6b or separate)
- Bulk operations UI
- Re-run task confirmation dialogs
- Quick actions menu (context menu)
- Optimistic locking conflict dialogs

## Detailed Requirements

### Design System (MANDATORY - Anti-AI-Slop)

**Color Palette:**
```css
:root {
  /* Backgrounds - dark grays, NOT pure black */
  --bg-base: #0f0f0f;
  --bg-surface: #1a1a1a;
  --bg-elevated: #242424;
  --bg-hover: #2d2d2d;

  /* Text - off-white, NOT pure white */
  --text-primary: #f0f0f0;
  --text-secondary: #a0a0a0;
  --text-muted: #666666;

  /* Accent - warm, distinctive (NOT purple) */
  --accent-primary: #ff6b35;      /* Warm orange */
  --accent-secondary: #ffa94d;    /* Soft amber */

  /* Status */
  --status-success: #10b981;      /* Emerald */
  --status-warning: #f59e0b;      /* Amber */
  --status-error: #ef4444;        /* Red */
  --status-info: #3b82f6;         /* Blue (sparingly) */

  /* Borders & Dividers */
  --border-subtle: rgba(255, 255, 255, 0.06);
  --border-default: rgba(255, 255, 255, 0.1);
}
```

**Typography:**
```css
:root {
  --font-display: 'SF Pro Display', -apple-system, sans-serif;
  --font-body: 'SF Pro Text', -apple-system, sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', monospace;
}
```

**Spacing (8pt Grid):**
- `--space-1`: 4px
- `--space-2`: 8px
- `--space-3`: 12px
- `--space-4`: 16px
- `--space-6`: 24px
- `--space-8`: 32px
- `--space-12`: 48px

**Anti-Slop Guardrails (MUST FOLLOW):**
1. NO purple or blue-purple gradients anywhere
2. NO Inter font - use SF Pro or system fonts
3. NO generic icon grids (3 boxes with icons)
4. NO high-saturation colors on dark backgrounds
5. ALWAYS use CSS variables - never hardcode colors
6. ALWAYS follow 8pt grid - no random spacing
7. ALWAYS maintain 4.5:1 contrast ratio for text

### Kanban Board Structure

**7 Columns:**
| Column | Purpose | Behavior |
|--------|---------|----------|
| Draft | Ideas, brainstorming output | Editable, drag anywhere |
| Backlog | Confirmed but deferred | Drag to Planned/To-do |
| To-do | Ready to schedule | Drag to Planned |
| Planned | Auto-executes when capacity available | **Read-only once picked up** |
| In Progress | Currently running | **Locked - use Stop action** |
| In Review | AI reviewing | **Locked - wait for review** |
| Done | Approved, Skipped, Failed | Shows terminal badges |

### Drag & Drop Behavior

**Allowed Transitions:**
| Action | Allowed | Effect |
|--------|---------|--------|
| Drag within same column | ✓ | Reorder = change priority (higher = first) |
| Drag to Planned | ✓ | Auto-executes when capacity available |
| Drag from Planned back | ✓ | Removes from queue (if not yet started) |
| Drag to Backlog | ✓ | Defer for later |
| Drag out of In Progress | ✗ | Locked while running |
| Drag out of In Review | ✗ | Locked while reviewing |
| Drag to Done | ✗ | Can't manually complete |
| Drag within Done | ✗ | Terminal states, no reorder |

**Visual Feedback:**
- Valid drop target: column highlights with accent border (`--accent-primary`)
- Invalid drop: column shows ✗ icon, card snaps back
- Dragging: card becomes semi-transparent (opacity-50), shows ghost at cursor
- Drop: smooth animation to new position (150-200ms, ease-out)

**Validation to Planned:**
- Must have title & description
- Warn if no steps defined

**Race Condition (Planned → To-do/Backlog):**
- Backend checks if task already picked up
- If picked: show error toast "Task already started, use Stop to cancel"
- If not: allow move

### Component Architecture

**File Structure:**
```
src/components/tasks/TaskBoard/
├── index.tsx       # Public export
├── TaskBoard.tsx   # Main component (~150 lines max)
├── Column.tsx      # Droppable column
├── TaskCard.tsx    # Draggable card
├── TaskBoardSkeleton.tsx  # Loading state
└── hooks.ts        # useTaskBoard hook
```

**TaskBoard Props:**
```typescript
interface TaskBoardProps {
  projectId: string;
  workflowId: string;
}
```

**useTaskBoard Hook:**
- Fetches tasks via TanStack Query
- Fetches workflow for column definitions
- Provides `columns` array (memoized from workflow + filtered tasks)
- Provides `onDragEnd` callback for move mutation
- Returns `isLoading` for skeleton state

### TaskCard Design

**Card Contents:**
- Title (truncated if long)
- Category badge (if present)
- Priority indicator (position in column)
- Review status badge:
  - `✓ AI Approved` (green)
  - `✓✓ Human Approved` (blue)
  - `⚠ Needs Changes` (amber)
- QA status badge (if QA enabled):
  - Gray: pending
  - Yellow: preparing
  - Blue: ready
  - Purple: testing
  - Green: passed
  - Red: failed
- Checkpoint indicator (if task has checkpoint type)
- Drag handle on hover

**Dragging State:**
- `opacity-50` class when `isDragging={true}`
- Ghost card follows cursor

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `P` | Move selected task to Planned |
| `B` | Move selected task to Backlog |
| `T` | Move selected task to To-do |
| `Delete` | Move to Skipped (with confirmation) |

### WorkflowSchema Types

```typescript
interface WorkflowSchema {
  id: string;
  name: string;
  description: string;
  columns: WorkflowColumn[];
  defaults: {
    workerProfile?: string;
    reviewerProfile?: string;
  };
}

interface WorkflowColumn {
  id: string;
  name: string;
  color?: string;
  icon?: string;
  mapsTo: InternalStatus;
  behavior?: {
    skipReview?: boolean;
    autoAdvance?: boolean;
    agentProfile?: string;
  };
}
```

**Default Workflow:**
```typescript
const defaultWorkflow: WorkflowSchema = {
  id: "ralphx-default",
  name: "RalphX Default",
  columns: [
    { id: "draft", name: "Draft", mapsTo: "backlog" },
    { id: "backlog", name: "Backlog", mapsTo: "backlog" },
    { id: "todo", name: "To Do", mapsTo: "ready" },
    { id: "planned", name: "Planned", mapsTo: "ready" },
    { id: "in_progress", name: "In Progress", mapsTo: "executing" },
    { id: "in_review", name: "In Review", mapsTo: "pending_review" },
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
};
```

## Implementation Notes

### Drag-Drop Library
Use `@dnd-kit/core` and `@dnd-kit/sortable` for drag-drop:
- `DndContext` wraps the board
- `useDroppable` for columns
- `useDraggable` for task cards
- `useSortable` for reordering within columns

### Component Size Limits
| Component | Max Lines |
|-----------|-----------|
| TaskBoard.tsx | 150 |
| Column.tsx | 100 |
| TaskCard.tsx | 100 |
| hooks.ts | 100 |

### Testing Requirements
- **TDD mandatory** for all components
- Unit tests with Vitest + Testing Library
- Test drag-drop behavior with mocked DndContext
- Test keyboard shortcuts
- Visual verification for:
  - Board renders with 7 columns
  - Task cards display correctly
  - Drag handles visible
  - Status badges render

### Visual Verification Patterns
```bash
# Verify board renders
agent-browser open http://localhost:1420
agent-browser wait --load
agent-browser is visible "[data-testid='task-board']"
agent-browser screenshot screenshots/task-board-renders.png

# Verify drag-drop works
agent-browser drag @e5 @e8  # Task card to target column
agent-browser screenshot screenshots/kanban-drag-drop.png
```

## Task List

```json
[
  {
    "category": "setup",
    "description": "Install @dnd-kit dependencies",
    "steps": [
      "Run npm install @dnd-kit/core @dnd-kit/sortable @dnd-kit/utilities",
      "Verify packages installed in package.json",
      "Create basic DndContext test to verify import works"
    ],
    "passes": true
  },
  {
    "category": "setup",
    "description": "Create design system CSS variables",
    "steps": [
      "Create src/styles/design-tokens.css with all CSS variables from spec",
      "Include color palette (backgrounds, text, accent, status)",
      "Include typography (font-display, font-body, font-mono)",
      "Include spacing scale (space-1 through space-12)",
      "Import in main.tsx or App.tsx",
      "Write test to verify CSS variables are defined"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create WorkflowSchema and WorkflowColumn types",
    "steps": [
      "Write tests for WorkflowSchema type validation",
      "Create src/types/workflow.ts with Zod schemas",
      "Define WorkflowSchema interface with columns array",
      "Define WorkflowColumn interface with mapsTo, behavior fields",
      "Export defaultWorkflow constant with 7 columns",
      "Run tests to verify types work correctly"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Tauri API wrapper for workflows",
    "steps": [
      "Write tests for workflow API calls",
      "Add api.workflows.get() to src/lib/tauri.ts",
      "Add api.workflows.list() for future workflow switching",
      "Use Zod validation for response parsing",
      "Run tests to verify API wrapper"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create useTaskBoard hook",
    "steps": [
      "Write tests for useTaskBoard hook behavior",
      "Create src/components/tasks/TaskBoard/hooks.ts",
      "Implement task fetching with TanStack Query",
      "Implement workflow fetching with TanStack Query",
      "Create memoized columns computation (workflow.columns + filtered tasks)",
      "Implement moveMutation for task status changes",
      "Implement onDragEnd callback",
      "Return { columns, onDragEnd, isLoading }",
      "Keep under 100 lines",
      "Run tests to verify hook behavior"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create TaskBoardSkeleton component",
    "steps": [
      "Write tests for skeleton rendering",
      "Create src/components/tasks/TaskBoard/TaskBoardSkeleton.tsx",
      "Render 7 column placeholders with loading animation",
      "Use design system colors (--bg-surface, --bg-elevated)",
      "Add data-testid='task-board-skeleton'",
      "Run tests to verify skeleton renders"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create StatusBadge component",
    "steps": [
      "Write tests for badge variants (AI Approved, Human Approved, Needs Changes)",
      "Create src/components/ui/StatusBadge.tsx",
      "Implement badge with correct colors from design system",
      "Support review status variants with icons",
      "Support QA status variants (pending, preparing, ready, testing, passed, failed)",
      "Use CSS variables for all colors",
      "Run tests to verify all badge variants"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create TaskCard component",
    "steps": [
      "Write tests for TaskCard rendering (title, status badge, checkpoint indicator)",
      "Write tests for click handler (onSelect)",
      "Write tests for dragging state (opacity-50 class)",
      "Create src/components/tasks/TaskBoard/TaskCard.tsx",
      "Implement card with title, category badge, priority indicator",
      "Add StatusBadge for review status",
      "Add QA status badge when task.needs_qa is true",
      "Add checkpoint indicator when task.checkpointType exists",
      "Add drag handle that appears on hover",
      "Implement isDragging prop for opacity change",
      "Use useDraggable hook from @dnd-kit",
      "Add data-testid='task-card-{id}'",
      "Keep under 100 lines",
      "Run tests to verify all behavior"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create Column component",
    "steps": [
      "Write tests for Column rendering (header, task list)",
      "Write tests for droppable behavior",
      "Write tests for drop validation (locked columns)",
      "Create src/components/tasks/TaskBoard/Column.tsx",
      "Implement column header with name and task count",
      "Use useDroppable hook from @dnd-kit",
      "Implement isOver styling (accent border highlight)",
      "Implement invalid drop feedback (✗ icon)",
      "Handle locked columns (In Progress, In Review, Done)",
      "Add data-testid='column-{id}'",
      "Keep under 100 lines",
      "Run tests to verify droppable behavior"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create TaskBoard component",
    "steps": [
      "Write tests for TaskBoard rendering with columns",
      "Write tests for loading state (shows skeleton)",
      "Write tests for drag-drop between columns",
      "Create src/components/tasks/TaskBoard/TaskBoard.tsx",
      "Wrap with DndContext from @dnd-kit",
      "Render 7 Column components from useTaskBoard",
      "Show TaskBoardSkeleton when isLoading",
      "Implement onDragEnd handler with validation",
      "Add horizontal scroll with overflow-x-auto",
      "Add data-testid='task-board'",
      "Keep under 150 lines",
      "Run tests to verify board behavior"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create TaskBoard index.tsx with public exports",
    "steps": [
      "Create src/components/tasks/TaskBoard/index.tsx",
      "Export TaskBoard component",
      "Export TaskBoardProps type",
      "Verify exports work with import test"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement drag-drop validation logic",
    "steps": [
      "Write tests for all transition rules from spec",
      "Create validateDrop utility function",
      "Block drag out of In Progress",
      "Block drag out of In Review",
      "Block drag to Done column",
      "Block drag within Done column",
      "Validate Planned requires title and description",
      "Return validation result with error message",
      "Run tests to verify all rules enforced"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement visual feedback for drag-drop",
    "steps": [
      "Write tests for visual feedback states",
      "Add valid drop target styling (accent border)",
      "Add invalid drop styling (red border, ✗ icon)",
      "Add dragging card opacity (opacity-50)",
      "Add drop animation (150-200ms ease-out)",
      "Use CSS transitions for smooth animations",
      "Run tests to verify visual states"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement priority reordering within columns",
    "steps": [
      "Write tests for priority reordering",
      "Use useSortable from @dnd-kit/sortable",
      "Update task priority based on new position",
      "Higher position = higher priority (lower number)",
      "Create reorder mutation in useTaskBoard",
      "Run tests to verify priority updates"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement keyboard shortcuts",
    "steps": [
      "Write tests for P key (move to Planned)",
      "Write tests for B key (move to Backlog)",
      "Write tests for T key (move to To-do)",
      "Write tests for Delete key (move to Skipped with confirmation)",
      "Create useKeyboardShortcuts hook",
      "Integrate with selected task state from uiStore",
      "Show toast for successful moves",
      "Run tests to verify shortcuts work"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement race condition handling for Planned column",
    "steps": [
      "Write tests for race condition scenario",
      "Check if task already picked up before allowing move from Planned",
      "Show error toast if task already started",
      "Snap card back to original position on error",
      "Use optimistic update with rollback on failure",
      "Run tests to verify race condition handled"
    ],
    "passes": true
  },
  {
    "category": "integration",
    "description": "Integrate TaskBoard with App",
    "steps": [
      "Add TaskBoard to ProjectPage or main layout",
      "Pass projectId and workflowId props",
      "Verify board renders with mock data",
      "Test drag-drop end-to-end",
      "Visual verification: take screenshot of board"
    ],
    "passes": true
  },
  {
    "category": "testing",
    "description": "Visual verification of TaskBoard",
    "steps": [
      "Ensure app is running (check with `pgrep -f ralphx`; if not running, start with `npm run tauri dev`)",
      "Use agent-browser to open localhost",
      "Verify task-board element is visible",
      "Verify 7 columns render correctly",
      "Verify task cards display with badges",
      "Test drag-drop visually",
      "Take screenshots for verification",
      "Document results in activity log"
    ],
    "passes": true
  }
]
```
