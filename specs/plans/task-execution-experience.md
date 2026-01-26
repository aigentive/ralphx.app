# RalphX Task Execution Experience

## Overview

This unified plan combines **Task Steps** (deterministic progress tracking) with **Execution UX** (reactive cards + full-screen task view with integrated chat). Together, these features create a cohesive execution experience where users can see real progress on the Kanban board and dive deep into any task with a single click.

**Supersedes:**
- `specs/plans/task-steps-progress-tracking.md` (merged)
- `specs/plans/task-execution-ux-redesign.md` (merged)

---

## Problem Statement

The current execution experience is fragmented:
1. **No progress visibility** - Task cards show only a status badge, no indication of how far along work is
2. **Fragmented views** - Users must navigate Kanban → Activity → Chat to understand execution
3. **No structured steps** - Agents work without trackable checkpoints
4. **Disconnected chat** - Execution output is in a separate panel, not integrated with task details

## Solution

A two-part solution:

### Part 1: Task Steps (Data Layer)
- Dedicated `task_steps` table with status tracking
- MCP tools for worker agent to update progress
- Import steps from proposals automatically

### Part 2: Execution UX (Presentation Layer)
- Reactive TaskCards with progress indicators and animations
- Full-screen TaskFullView with split layout (details + chat)
- Context-aware chat that switches based on task state

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           KANBAN BOARD                                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐                │
│  │ Backlog  │  │  Ready   │  │Executing │  │ Done     │                │
│  │          │  │          │  │ ●●●      │  │          │                │
│  │ [card]   │  │ [card]   │  │ [card]   │  │ [card]   │                │
│  │ [card]   │  │ [card]   │  │  3/7     │  │ [card]   │   ← Progress   │
│  │          │  │          │  │  2m 15s  │  │          │     dots +     │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘     duration   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                              Click on card
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  ← Back                    Task Title                    [Edit] [×]     │
├────────────────────────────────────┬────────────────────────────────────┤
│                                    │                                     │
│  TASK DETAILS (Left)               │  EXECUTION CHAT (Right)            │
│  ─────────────────                 │  ──────────────────                │
│                                    │                                     │
│  Status: Executing ●●●             │  [Worker] Starting step 3...       │
│  Progress: 3/7 steps               │  [Tool] Read src/auth.ts           │
│                                    │  [Tool] Edit src/auth.ts:42        │
│  Steps:                            │  [Tool] Run tests                  │
│  ✓ 1. Set up auth module           │  ...                               │
│  ✓ 2. Add login endpoint           │                                     │
│  ● 3. Add OAuth providers  ←active │                                     │
│  ○ 4. Add session management       │                                     │
│  ○ 5. Write tests                  │  ┌─────────────────────────────┐  │
│  ○ 6. Update docs                  │  │ Message worker...           │  │
│  ○ 7. Review security              │  └─────────────────────────────┘  │
│                                    │                                     │
└────────────────────────────────────┴────────────────────────────────────┘
```

---

## Data Model: TaskStep

### Entity

```rust
pub struct TaskStep {
    pub id: TaskStepId,
    pub task_id: TaskId,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStepStatus,      // pending, in_progress, completed, skipped, failed, cancelled
    pub sort_order: i32,
    pub depends_on: Option<TaskStepId>,
    pub created_by: String,          // "user", "agent", "proposal", "system"
    pub completion_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub enum TaskStepStatus {
    Pending,      // Waiting to be worked on
    InProgress,   // Currently being executed
    Completed,    // Finished successfully
    Skipped,      // Not applicable or deferred
    Failed,       // Needs attention
    Cancelled,    // Task was cancelled
}
```

### Database Schema

```sql
CREATE TABLE task_steps (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    sort_order INTEGER NOT NULL DEFAULT 0,
    depends_on TEXT REFERENCES task_steps(id) ON DELETE SET NULL,
    created_by TEXT NOT NULL DEFAULT 'user',
    completion_note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT
);

CREATE INDEX idx_task_steps_task_id ON task_steps(task_id);
CREATE INDEX idx_task_steps_task_order ON task_steps(task_id, sort_order);
```

### Progress Summary

```rust
pub struct StepProgressSummary {
    pub task_id: String,
    pub total: u32,
    pub completed: u32,
    pub in_progress: u32,
    pub pending: u32,
    pub skipped: u32,
    pub failed: u32,
    pub current_step: Option<TaskStep>,
    pub next_step: Option<TaskStep>,
    pub percent_complete: f32,  // (completed + skipped) / total * 100
}
```

---

## MCP Tools for Worker Agent

| Tool | Description | Parameters |
|------|-------------|------------|
| `get_task_steps` | Fetch steps for current task | `task_id` |
| `start_step` | Mark step as in-progress | `step_id` |
| `complete_step` | Mark step as completed | `step_id`, `note?` |
| `skip_step` | Mark step as skipped | `step_id`, `reason` |
| `fail_step` | Mark step as failed | `step_id`, `error` |
| `add_step` | Add new step during execution | `task_id`, `title`, `description?`, `after_step_id?` |
| `get_step_progress` | Get progress summary | `task_id` |

### Worker Agent Instructions

```markdown
## Step Progress Tracking

When executing a task, you MUST track progress using steps:

1. **At start**, call `get_task_steps(task_id)` to see the plan
2. **Before each step**, call `start_step(step_id)`
3. **After each step**, call `complete_step(step_id, note?)`
4. **If step not needed**, call `skip_step(step_id, reason)`
5. **If step fails**, call `fail_step(step_id, error)`

If no steps exist, create them as you plan your work using `add_step`.
Break down the task into 3-8 discrete, verifiable steps.
```

---

## UI Components

### Reactive TaskCard

**Execution state visuals:**

| Status | Border | Animation | Indicator |
|--------|--------|-----------|-----------|
| `executing` | 2px orange | Pulsing glow | Progress dots + duration |
| `qa_*` | 2px orange | Pulsing | QA phase badge |
| `pending_review` | 2px amber | Static | Eye icon |
| `revision_needed` | 2px amber | Attention pulse | Alert icon |

**Progress indicator on card:**
```
●●●○○○○  3/7  2m 15s
```

### TaskFullView (Split Layout)

Full-screen overlay replacing TaskDetailModal for executing tasks:
- **Left panel (50%)**: Task details, steps list, context, history
- **Right panel (50%)**: Context-aware chat (execution/review/discussion)
- **Resizable** via drag handle
- **Keyboard shortcut**: Escape to close

### StepList Component

```tsx
<StepList taskId={taskId} editable={!isExecuting}>
  ✓ 1. Set up auth module
  ✓ 2. Add login endpoint
  ● 3. Add OAuth providers  ← spinning indicator
  ○ 4. Add session management
  ○ 5. Write tests
</StepList>
```

### StepProgressBar (Compact)

For TaskCard footer:
```tsx
<StepProgressBar taskId={taskId} compact />
// Renders: ●●●○○○○ 3/7
```

---

## Implementation Order

The implementation is organized in **dependency order** - each part builds on the previous:

### Layer 1: Backend Data (Steps)
Tasks 1-6: Entity, migration, repository, AppState integration

### Layer 2: Backend API (Steps)
Tasks 7-10: Tauri commands for CRUD and status updates

### Layer 3: Backend MCP (Steps)
Tasks 11-13: HTTP endpoints and worker tool scoping

### Layer 4: Worker Integration
Tasks 14-15: Agent prompt updates and context injection

### Layer 5: Frontend Data (Steps)
Tasks 16-22: Types, API bindings, hooks, events

### Layer 6: Frontend Components (Steps)
Tasks 23-27: StepProgressBar, StepList, StepItem

### Layer 7: Reactive TaskCards
Tasks 28-32: Execution state visuals, animations, progress display

### Layer 8: TaskFullView
Tasks 33-38: Split layout component with embedded chat

### Layer 9: Integration
Tasks 39-42: Proposal import, form editors, Activity tab evaluation

---

## Events

### Step Events (Tauri)

```typescript
"step:created"   { step: TaskStep }
"step:updated"   { step_id, task_id, status, previous_status }
"step:deleted"   { step_id, task_id }
"steps:reordered" { task_id, step_ids[] }
```

### Frontend Event Handling

```typescript
// Invalidate queries on step events
useStepEvents() → listen for step:* → invalidate stepKeys.byTask(taskId)
```

---

## Design Tokens

```css
/* Step status colors */
--step-pending: var(--text-muted);
--step-in-progress: var(--accent-primary);
--step-completed: var(--status-success);
--step-skipped: var(--text-muted);
--step-failed: var(--status-error);

/* Execution animations */
--animation-executing-pulse: executing-pulse 2s ease-in-out infinite;
--animation-attention-pulse: attention-pulse 1.5s ease-in-out infinite;

/* TaskFullView layout */
--task-full-view-margin: var(--space-6);  /* 24px */
--task-full-view-min-panel: 360px;
```

---

## Success Criteria

1. **Deterministic Progress**: TaskCard shows "Step 3/7" during execution
2. **Real-time Updates**: Steps update as worker progresses
3. **Agent Compliance**: Worker agent uses step tools consistently
4. **Unified View**: Opening a task shows details AND execution chat together
5. **Proposal Integration**: Steps flow from proposal → task automatically
6. **Visual Clarity**: At-a-glance understanding of what's running and how far along

---

## Key Decisions

| Decision | Rationale |
|----------|-----------|
| Separate `task_steps` table | Proper relational model, queryable, auditable |
| Steps have states, not just done/not-done | Richer progress info (skipped, failed) |
| Worker MUST call step tools | Enforced by prompt, provides reliable progress |
| Steps created from proposal.steps | Continuity from ideation to execution |
| Full-screen view for executing tasks | Chat needs space, modal was too cramped |
| 50/50 split default | Both panels equally important |
| Context-aware chat | Right panel shows relevant conversation |

---

## Open Questions

1. Should steps be immutable once task is executing? (Probably yes for completed)
2. Should we track which tool calls belong to which step? (Defer - complex)
3. Should Activity tab be removed after TaskFullView ships? (Evaluate after)
4. Should there be sound effects for step completion? (Nice-to-have)
