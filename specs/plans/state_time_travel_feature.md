# State Time Travel Feature

## Overview

Add the ability to view historical states of a task - see exactly what happened during execution, review, escalation, etc. Users can "rewind" to any past state and see the task details + conversation as they were at that point.

## Design

```
┌─────────────────────────────────────────────────────────────────┐
│  [Ready] ─── [Executing] ─── [Reviewing] ─── [Escalated] ─── ●  │
│                                   ▲                             │
│                              clicking                           │
├─────────────────────────────────────────────────────────────────┤
│  ⚠️ Viewing historical state: Reviewing (Jan 28, 2:30pm)        │
│                                          [Return to Current]   │
├────────────────────────────────┬────────────────────────────────┤
│  Task Details                  │  Chat (Read-only)              │
│  (ReviewingTaskDetail view)    │  (Messages from Reviewing)     │
│                                │                                │
│  • AI review in progress...    │  AI: I've reviewed the code    │
│  • Found 2 issues              │  and found the following...    │
│                                │                                │
└────────────────────────────────┴────────────────────────────────┘
```

**Key behaviors:**
- Timeline shows all states the task has been through
- Click any past state → enters "history mode"
- Task detail renders using view registry for that state
- Chat filters to show only messages from that state
- Banner indicates viewing history, button to return
- Current state (rightmost) always exits history mode

## Data Flow

**Already available:**
- `task_state_history` - transitions with timestamps
- `activity_events.internal_status` - messages tagged by state
- View registry - state → component mapping

**No new backend work needed** - just smarter frontend querying.

## Implementation Tasks

### Task 1: Create StateTimelineNav Component
**Dependencies:** Task 5, Task 6
**Atomic Commit:** `feat(tasks): add StateTimelineNav component for history navigation`
**Files:** `src/components/tasks/StateTimelineNav.tsx`

New horizontal timeline navigation component:
- Props: `taskId`, `currentStatus`, `onStateSelect`, `selectedState?`
- Fetch state history via `useTaskStateHistory` (extend if needed)
- Render clickable state badges in chronological order
- Highlight selected state vs current state
- Show timestamps on hover

### Task 2: Add History Mode State to TaskDetailOverlay (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(tasks): add history mode state and banner to TaskDetailOverlay`
**Files:** `src/components/tasks/TaskDetailOverlay.tsx`

Add state management for history mode:
```typescript
const [historyState, setHistoryState] = useState<{
  status: InternalStatus;
  timestamp: string;
} | null>(null);

const isHistoryMode = historyState !== null;
const viewStatus = historyState?.status ?? task.internalStatus;
```

- Insert `StateTimelineNav` below header
- Pass `historyState` to child components
- Add history mode banner with "Return to Current" button

### Task 3: Extend TaskDetailPanel for History Mode
**Dependencies:** Task 2
**Atomic Commit:** `feat(tasks): add viewAsStatus prop to TaskDetailPanel for history mode`
**Files:** `src/components/tasks/TaskDetailPanel.tsx`

Add prop for viewing historical state:
```typescript
interface Props {
  // existing...
  viewAsStatus?: InternalStatus; // Override for history mode
}
```

- When `viewAsStatus` provided, use it for view registry lookup
- Pass to child detail views so they know it's historical
- Detail views render read-only (no action buttons)

### Task 4: Add Historical Message Filtering to TaskChatPanel
**Dependencies:** Task 2
**Atomic Commit:** `feat(tasks): add historical message filtering to TaskChatPanel`
**Files:** `src/components/tasks/TaskChatPanel.tsx`, `src/hooks/useTaskChat.ts`

Filter messages by historical state:
```typescript
interface Props {
  // existing...
  historicalStatus?: InternalStatus; // Filter to this state only
}
```

- When `historicalStatus` provided:
  - Filter activity_events by `internal_status`
  - Disable message input (read-only)
  - Show "Historical view" indicator

### Task 5: Extend useTaskStateHistory Hook (BLOCKING)
**Dependencies:** Task 6
**Atomic Commit:** `feat(hooks): add useTaskStateTransitions hook for history timeline`
**Files:** `src/hooks/useReviews.ts` or new `src/hooks/useTaskStateTransitions.ts`

Current hook returns review notes. Need full state transitions:
```typescript
interface StateTransition {
  id: string;
  fromStatus: InternalStatus | null;
  toStatus: InternalStatus;
  timestamp: string;
  reason?: string;
}
```

- New query: `get_task_state_transitions` (or extend existing)
- Returns ordered list of all status changes
- Used by StateTimelineNav to render the timeline

### Task 6: Backend - Add State Transitions Query (if needed) (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(backend): add get_task_state_transitions command`
**Files:** `src-tauri/src/commands/task_commands.rs`

Check if `task_state_history` query exists. If not:
```rust
#[tauri::command]
pub async fn get_task_state_transitions(task_id: String) -> Result<Vec<StateTransition>>
```

- Query `task_state_history` table
- Return chronological list of transitions
- Include timestamps for timeline rendering

## File Summary

| File | Change |
|------|--------|
| `src/components/tasks/StateTimelineNav.tsx` | NEW - Timeline navigation |
| `src/components/tasks/TaskDetailOverlay.tsx` | Add history mode state + banner |
| `src/components/tasks/TaskDetailPanel.tsx` | Add `viewAsStatus` prop |
| `src/components/tasks/TaskChatPanel.tsx` | Add `historicalStatus` filtering |
| `src/hooks/useTaskChat.ts` | Support historical message filtering |
| `src/hooks/useTaskStateTransitions.ts` | NEW - Fetch state transitions |
| `src-tauri/src/commands/task_commands.rs` | Maybe add transitions query |

## Verification

1. Open a completed task (e.g., Approved status)
2. Timeline shows: Ready → Executing → Reviewing → Approved
3. Click "Executing" in timeline
4. Banner appears: "Viewing historical state: Executing"
5. Task detail shows ExecutionTaskDetail view
6. Chat shows only messages from executing phase
7. Click "Return to Current" → back to Approved view
8. Click current state (Approved) → also exits history mode

## Execution Order

Based on dependency analysis, execute tasks in this order:

```
Task 6 (backend)
    ↓
Task 5 (hook)
    ↓
Task 1 (StateTimelineNav)
    ↓
Task 2 (TaskDetailOverlay)
    ↓
┌───┴───┐
Task 3  Task 4  (can run in parallel)
```

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
