# RalphX Task Execution UX Redesign Plan

## Overview

This plan addresses the fragmented task execution experience by making tasks visually reactive on the Kanban board and introducing a full-screen task view with integrated chat.

**Problem Statement:**
The current execution experience requires users to navigate between Kanban (start tasks) → Activity (see logs) → Chat (interact with worker). Task cards are static and provide no feedback about execution progress.

**Goal:**
Create a unified experience where:
1. Task cards visually communicate execution state
2. Opening a task shows both details AND live execution chat
3. Progress is visible at-a-glance without leaving Kanban

---

## Available Progress Data

### What We Have (Backend Data)

| Data Point | Source | Update Frequency | Query Method |
|------------|--------|------------------|--------------|
| **Status** (14 states) | `task.internalStatus` | On state transition | Task store |
| **Started timestamp** | `task.startedAt` | Once (→ Executing) | Task store |
| **Completed timestamp** | `task.completedAt` | Once (terminal state) | Task store |
| **Activity messages** | `agent:message` event | High frequency | Activity store (100 max) |
| **Execution status** | `useExecutionStatus()` | 5s polling + events | TanStack Query |
| **Tool calls** | `execution:tool_call` event | Per tool use | Chat store |
| **QA phases** | `qaStore.taskQA[id]` | Per phase transition | QA store |
| **Review status** | `review:update` event | Per review change | Review query |
| **Supervisor alerts** | `supervisor:alert` event | On anomaly | Activity store |

### Available Events (Real-time)

```typescript
// Agent activity (high frequency)
AgentMessageEvent {
  taskId: string
  type: "thinking" | "tool_call" | "tool_result" | "text" | "error"
  content: string
  timestamp: number
}

// Execution streaming
"execution:chunk"        // Text from worker
"execution:tool_call"    // Tool invocation
"execution:run_completed" // Worker finished

// QA events
QATestEvent {
  taskId, type, totalSteps, passedSteps, failedSteps
}
```

### What We CANNOT Show

| Metric | Why Not Available |
|--------|-------------------|
| % completion | No way to know total work upfront |
| Estimated time remaining | No historical data for estimation |
| Steps remaining | Worker execution is dynamic |

### Realistic Progress Indicators

| Indicator | Visual | Data Source |
|-----------|--------|-------------|
| **Phase** | Status pill with icon | `task.internalStatus` |
| **Activity pulse** | Animated border/glow | `agent:message` events (recent) |
| **Duration** | "Running 2m 34s" | `now - task.startedAt` |
| **Last action** | "Edit auth.ts 5s ago" | Most recent `agent:message` |
| **Operations count** | "12 operations" | Count of `tool_call` events |
| **QA phase** | ○○○ → ●○○ → ●●○ → ●●● | QA store timestamps |

---

## Design Specification

### Part 1: Reactive Task Cards

#### 1.1 Execution State Visuals

**Status → Visual Mapping:**

| Status | Border | Background | Animation | Icon |
|--------|--------|------------|-----------|------|
| `executing` | 2px solid orange | Subtle orange tint | Pulsing glow | Spinner |
| `qa_refining` | 2px solid orange | Subtle tint | Pulsing | Microscope |
| `qa_testing` | 2px solid orange | Subtle tint | Pulsing | FlaskConical |
| `pending_review` | 2px solid amber | Amber tint | Static | Eye |
| `revision_needed` | 2px solid amber | Amber tint | Attention pulse | AlertTriangle |
| `blocked` | 2px solid red | Red tint | Static | Ban |
| `qa_passed` | 2px solid green | Green tint | Fade in | CheckCircle |
| `qa_failed` | 2px solid red | Red tint | Static | XCircle |

**Activity Indicator (Top-right corner when executing):**
```tsx
// Animated dots for active execution
<div className="flex gap-0.5">
  <span className="w-1.5 h-1.5 rounded-full bg-accent-primary animate-bounce [animation-delay:0ms]" />
  <span className="w-1.5 h-1.5 rounded-full bg-accent-primary animate-bounce [animation-delay:150ms]" />
  <span className="w-1.5 h-1.5 rounded-full bg-accent-primary animate-bounce [animation-delay:300ms]" />
</div>
```

**Duration Badge (Bottom of card when executing):**
```tsx
<Badge variant="secondary" className="text-[10px]">
  <Clock className="w-3 h-3 mr-1" />
  2m 34s
</Badge>
```

#### 1.2 CSS Animations

```css
/* Pulsing glow for executing state */
@keyframes executing-pulse {
  0%, 100% { box-shadow: 0 0 0 2px rgba(255, 107, 53, 0.2); }
  50% { box-shadow: 0 0 0 4px rgba(255, 107, 53, 0.4); }
}

.task-card-executing {
  animation: executing-pulse 2s ease-in-out infinite;
  border-left: 3px solid var(--accent-primary);
}

/* Attention pulse for needs-attention states */
@keyframes attention-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.7; }
}
```

#### 1.3 Hover Preview (Future Enhancement)

When hovering on an executing task, show a tooltip with:
- Last tool call name and time
- Operations count
- Duration

---

### Part 2: Full-Screen Task View

#### 2.1 Layout Structure

Replace the current 640px centered modal with a full-screen overlay:

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Header: [← Back] Task Title                    [Edit] [Archive] [Status ▼] [×] │
├───────────────────────────────────┬────────────────────────────────────────┤
│                                   │                                         │
│   LEFT PANEL (50%)                │   RIGHT PANEL (50%)                    │
│   ────────────────                │   ─────────────────                    │
│                                   │                                         │
│   Task Details                    │   Context-Aware Chat                   │
│   • Priority, Category            │   • Execution log when executing       │
│   • Status with progress          │   • Review chat when in review         │
│   • Description                   │   • Task discussion otherwise          │
│                                   │                                         │
│   Context Section                 │   ┌─────────────────────────────────┐  │
│   • Source proposal               │   │ Message list (scrollable)       │  │
│   • Implementation plan           │   │ • Tool calls with expand        │  │
│                                   │   │ • Text responses                │  │
│   QA Progress (if applicable)     │   │ • Errors highlighted            │  │
│   ○ Prep  ● Refine  ○ Test       │   └─────────────────────────────────┘  │
│                                   │                                         │
│   History Timeline                │   ┌─────────────────────────────────┐  │
│   • State transitions             │   │ Input: Send message...          │  │
│                                   │   └─────────────────────────────────┘  │
│                                   │                                         │
├───────────────────────────────────┴────────────────────────────────────────┤
│ Footer: [Pause] [Stop]                Running 1/3 • Queued: 2              │
└────────────────────────────────────────────────────────────────────────────┘
```

**Dimensions:**
- Full viewport minus 24px margin on all sides (Raycast-style)
- Minimum panel width: 360px
- Resizable via drag handle between panels
- Default split: 50/50

#### 2.2 Right Panel Context Modes

The right panel's chat context changes based on task state:

| Task Status | Context Type | Chat Behavior |
|-------------|--------------|---------------|
| `executing` | `task_execution` | Shows worker agent output, tool calls, streaming |
| `qa_*` | `task_execution` | Shows QA agent output |
| `pending_review` | `task` | Review discussion |
| `revision_needed` | `task` | Revision discussion |
| Other | `task` | General task Q&A |

**Context Switch Animation:**
- Fade out old content (150ms)
- Show "Switching context..." indicator
- Fade in new content (150ms)

#### 2.3 Header Design

```tsx
<header className="flex items-center justify-between h-14 px-6 border-b">
  {/* Left: Back button + Title */}
  <div className="flex items-center gap-3">
    <Button variant="ghost" size="sm" onClick={onClose}>
      <ArrowLeft className="w-4 h-4 mr-2" />
      Back
    </Button>
    <PriorityBadge priority={task.priority} />
    <h1 className="text-lg font-semibold truncate max-w-[400px]">
      {task.title}
    </h1>
    <StatusBadge status={task.internalStatus} />
  </div>

  {/* Right: Actions */}
  <div className="flex items-center gap-2">
    {canEdit && <Button variant="ghost" size="icon"><Pencil /></Button>}
    {!isArchived && <Button variant="ghost" size="icon"><Archive /></Button>}
    <StatusDropdown ... />
    <Button variant="ghost" size="icon" onClick={onClose}><X /></Button>
  </div>
</header>
```

#### 2.4 Left Panel Sections

1. **Task Info Section**
   - Priority badge, category, creation date
   - Status with visual indicator
   - Duration (if executing/completed)

2. **Description Section**
   - Markdown-rendered description
   - Expandable if long

3. **Context Section** (collapsible)
   - Source proposal summary
   - Implementation plan preview
   - Related artifacts list

4. **QA Progress Section** (if QA enabled)
   - Three-phase indicator with timestamps
   - Test results summary (if available)

5. **History Section** (collapsible)
   - State transition timeline
   - Review decisions

#### 2.5 Right Panel (Embedded Chat)

Reuse existing `ChatPanel` internals but:
- Remove resize handle
- Remove collapse button
- Always visible
- Context set by task state
- Show "Worker Execution" header when executing

---

### Part 3: Implementation Tasks

#### Phase A: Reactive Task Cards (Priority: High)

```json
[
  {
    "id": "A1",
    "task": "Add execution state hook",
    "description": "Create useTaskExecutionState(taskId) hook that returns: isActive, duration, lastActivity, operationsCount",
    "files": ["src/hooks/useTaskExecutionState.ts"]
  },
  {
    "id": "A2",
    "task": "Add status-based animations to TaskCard",
    "description": "Apply CSS animations based on internalStatus (executing pulse, qa pulse, attention pulse)",
    "files": ["src/components/tasks/TaskBoard/TaskCard.tsx", "src/styles/globals.css"]
  },
  {
    "id": "A3",
    "task": "Add activity dots indicator",
    "description": "Show animated dots in top-right corner when task.internalStatus === 'executing'",
    "files": ["src/components/tasks/TaskBoard/TaskCard.tsx"]
  },
  {
    "id": "A4",
    "task": "Add duration badge",
    "description": "Show 'Running Xm Xs' badge when executing, calculated from task.startedAt",
    "files": ["src/components/tasks/TaskBoard/TaskCard.tsx"]
  }
]
```

#### Phase B: Full-Screen Task View (Priority: High)

```json
[
  {
    "id": "B1",
    "task": "Create TaskFullView component shell",
    "description": "Full-screen overlay with header, split panels, footer. Add to uiStore: taskFullViewId",
    "files": ["src/components/tasks/TaskFullView.tsx", "src/stores/uiStore.ts"]
  },
  {
    "id": "B2",
    "task": "Extract TaskDetailPanel from modal",
    "description": "Refactor TaskDetailModal content into reusable TaskDetailPanel for left side",
    "files": ["src/components/tasks/TaskDetailPanel.tsx"]
  },
  {
    "id": "B3",
    "task": "Create TaskChatPanel component",
    "description": "Embedded chat panel for right side, using ChatPanel internals",
    "files": ["src/components/tasks/TaskChatPanel.tsx"]
  },
  {
    "id": "B4",
    "task": "Add resizable split panel",
    "description": "Implement drag-to-resize between left and right panels",
    "files": ["src/components/tasks/TaskFullView.tsx"]
  },
  {
    "id": "B5",
    "task": "Wire up full view opening from Kanban",
    "description": "TaskCard click opens TaskFullView instead of modal. Update TaskBoard.",
    "files": ["src/components/tasks/TaskBoard/TaskCard.tsx", "src/components/tasks/TaskBoard/TaskBoard.tsx"]
  }
]
```

#### Phase C: Context-Aware Chat (Priority: Medium)

```json
[
  {
    "id": "C1",
    "task": "Add context mode detection",
    "description": "Determine chat context based on task.internalStatus",
    "files": ["src/components/tasks/TaskChatPanel.tsx"]
  },
  {
    "id": "C2",
    "task": "Add context switch transition",
    "description": "Animate context changes with fade and indicator",
    "files": ["src/components/tasks/TaskChatPanel.tsx"]
  },
  {
    "id": "C3",
    "task": "Show execution-specific UI",
    "description": "When executing: show 'Worker Execution' header, tool call previews, streaming indicator",
    "files": ["src/components/tasks/TaskChatPanel.tsx"]
  }
]
```

#### Phase D: QA Progress Visualization (Priority: Medium)

```json
[
  {
    "id": "D1",
    "task": "Create QAProgressIndicator component",
    "description": "Three-phase visual: ○○○ → ●○○ → ●●○ → ●●● with labels and timestamps",
    "files": ["src/components/qa/QAProgressIndicator.tsx"]
  },
  {
    "id": "D2",
    "task": "Add QA progress to TaskFullView left panel",
    "description": "Show QAProgressIndicator when task has QA data",
    "files": ["src/components/tasks/TaskDetailPanel.tsx"]
  },
  {
    "id": "D3",
    "task": "Add QA mini-badge to TaskCard",
    "description": "Small indicator showing QA phase on card (Prep/Refine/Test)",
    "files": ["src/components/tasks/TaskBoard/TaskCard.tsx"]
  }
]
```

#### Phase E: Animation Polish (Priority: Low)

```json
[
  {
    "id": "E1",
    "task": "Add full view enter/exit animation",
    "description": "Scale from card position on open, scale back on close (Framer Motion)",
    "files": ["src/components/tasks/TaskFullView.tsx"]
  },
  {
    "id": "E2",
    "task": "Add status change micro-animation",
    "description": "Flash/pulse when task status changes while viewing",
    "files": ["src/components/tasks/TaskDetailPanel.tsx"]
  },
  {
    "id": "E3",
    "task": "Smooth scroll for chat messages",
    "description": "Auto-scroll to new messages with smooth behavior",
    "files": ["src/components/tasks/TaskChatPanel.tsx"]
  }
]
```

#### Phase F: Activity Tab Evaluation (Priority: Low)

```json
[
  {
    "id": "F1",
    "task": "Evaluate Activity tab usage",
    "description": "With chat in TaskFullView, determine if Activity is still needed",
    "decision": "Keep as 'Mission Control' for multi-task monitoring OR remove"
  },
  {
    "id": "F2",
    "task": "Add filtering to Activity (if kept)",
    "description": "Filter by: running tasks only, specific task, message type",
    "files": ["src/components/activity/ActivityView.tsx"]
  }
]
```

---

## Component Specifications

### useTaskExecutionState Hook

```typescript
interface TaskExecutionState {
  isActive: boolean;           // Has recent activity (<10s)
  duration: number | null;     // Seconds since startedAt
  lastActivity: {
    type: string;              // "tool_call", "text", etc.
    content: string;           // Truncated preview
    timestamp: number;         // Unix ms
  } | null;
  operationsCount: number;     // Count of tool_call events
  phase: "idle" | "executing" | "qa" | "review" | "done";
}

function useTaskExecutionState(taskId: string): TaskExecutionState;
```

### TaskFullView Props

```typescript
interface TaskFullViewProps {
  taskId: string;
  onClose: () => void;
}
```

### TaskDetailPanel Props

```typescript
interface TaskDetailPanelProps {
  task: Task;
  showContext?: boolean;      // Expand context section
  showHistory?: boolean;      // Expand history section
  onEdit?: () => void;
  onArchive?: () => void;
}
```

### TaskChatPanel Props

```typescript
interface TaskChatPanelProps {
  taskId: string;
  contextType: "task" | "task_execution";
  autoScroll?: boolean;
}
```

---

## UI Store Additions

```typescript
interface UiState {
  // Existing...

  // New for full view
  taskFullViewId: string | null;
  openTaskFullView: (taskId: string) => void;
  closeTaskFullView: () => void;
}
```

---

## Migration Path

1. **Phase A** can ship independently - improves Kanban immediately
2. **Phase B** replaces modal usage - TaskDetailModal kept for backwards compat temporarily
3. **Phase C+D** enhance the full view experience
4. **Phase E** is polish - can be done incrementally
5. **Phase F** is evaluation - decide after B ships

---

## Design Token Additions

```css
/* Animation tokens */
--animation-executing-pulse: executing-pulse 2s ease-in-out infinite;
--animation-attention-pulse: attention-pulse 1.5s ease-in-out infinite;
--animation-bounce-dot: bounce 1s infinite;

/* Full view tokens */
--task-full-view-margin: var(--space-6);  /* 24px */
--task-full-view-min-panel: 360px;
--task-full-view-divider: 4px;
```

---

## Success Criteria

1. **Kanban Clarity**: User can see at a glance which tasks are executing
2. **Zero Navigation**: Opening a task shows both details AND execution log
3. **Context Awareness**: Chat mode automatically matches task state
4. **Performance**: Animations run at 60fps, no layout thrashing
5. **Accessibility**: All states have screen reader announcements

---

## Open Questions

1. Should TaskDetailModal be deprecated entirely after TaskFullView ships?
2. Should we keep keyboard shortcut (Cmd+K) to toggle global ChatPanel?
3. Should Activity tab become "Mission Control" or be removed?
4. Should we add sound effects for status changes? (Mac native feel)
