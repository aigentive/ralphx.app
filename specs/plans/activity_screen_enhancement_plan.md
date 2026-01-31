# Activity Screen Enhancement Plan

## Overview

Enhance the activity screen with persistent storage, server-side pagination with infinite scroll, filtering capabilities, and capture missing event types (thinking blocks, tool results). Also improve the "agent is executing" widget UX across all chat types.

## Requirements Summary

1. **Persist activity events** to database with associated context (task/session, status, role)
2. **Server-side pagination** with infinite scroll for browsing historical events
3. **Filtering** by event type, status, associated entity, role
4. **Capture missing events**: thinking blocks and tool results (currently not emitted)
5. **Improve "agent is executing" widget** - show in all chats, reduce redundancy

---

## Part 1: Capture Missing Event Types (BLOCKING)

**Dependencies:** None
**Atomic Commit:** `feat(streaming): capture thinking blocks and tool results as activity events`

### Problem
The backend `StreamProcessor` does NOT parse or emit:
- **Thinking blocks** - No `Thinking` variant in `StreamEvent` enum
- **Tool results to activity** - `ToolResultReceived` emits to `AGENT_TOOL_CALL` only, not `AGENT_MESSAGE`

### Files to Modify

**Backend:**
- `src-tauri/src/infrastructure/agents/claude/stream_processor.rs` - Add `Thinking` variant to `StreamEvent`, parse thinking blocks from stream
- `src-tauri/src/application/chat_service/chat_service_streaming.rs` - Emit thinking and tool_result as `AGENT_MESSAGE` events

### Implementation

1. Add to `StreamEvent` enum:
```rust
/// Thinking block from Claude's reasoning
Thinking(String),
```

2. In `StreamProcessor::process_message()`, parse `content_block` with `type: "thinking"` and emit `StreamEvent::Thinking`

3. In `chat_service_streaming.rs`, add handlers:
```rust
StreamEvent::Thinking(text) => {
    if context_type == ChatContextType::TaskExecution {
        handle.emit(events::AGENT_MESSAGE, json!({
            "taskId": context_id_str,
            "type": "thinking",
            "content": text,
            "timestamp": chrono::Utc::now().timestamp_millis(),
        }));
    }
}

StreamEvent::ToolResultReceived { tool_use_id, result } => {
    // Existing AGENT_TOOL_CALL emission...

    // ADD: Activity stream event
    if context_type == ChatContextType::TaskExecution {
        handle.emit(events::AGENT_MESSAGE, json!({
            "taskId": context_id_str,
            "type": "tool_result",
            "content": serde_json::to_string(&result).unwrap_or_default(),
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "metadata": { "tool_use_id": tool_use_id },
        }));
    }
}
```

---

## Part 2: Database Persistence (BLOCKING)

**Dependencies:** Part 1
**Atomic Commits:** Multiple tasks below

### Task 2.1: Create activity_events entity (BLOCKING)
**Dependencies:** None (additive)
**Atomic Commit:** `feat(domain): add ActivityEvent entity`

**Files:**
- `src-tauri/src/domain/entities/activity_event.rs` - Entity definition
- `src-tauri/src/domain/entities/mod.rs` - Export entity

### Task 2.2: Create repository trait
**Dependencies:** Task 2.1
**Atomic Commit:** `feat(domain): add ActivityEventRepository trait`

**Files:**
- `src-tauri/src/domain/repositories/activity_event_repo.rs` - Repository trait
- `src-tauri/src/domain/repositories/mod.rs` - Export trait

### Task 2.3: Add database migration (BLOCKING)
**Dependencies:** Task 2.1
**Atomic Commit:** `feat(migrations): add activity_events table`

**Files:**
- `src-tauri/src/infrastructure/sqlite/migrations/v3_add_activity_events.rs` - Migration
- `src-tauri/src/infrastructure/sqlite/migrations/mod.rs` - Register migration

### New Table: `activity_events`

```sql
CREATE TABLE IF NOT EXISTS activity_events (
    id TEXT PRIMARY KEY,
    -- Context (polymorphic)
    task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    ideation_session_id TEXT REFERENCES ideation_sessions(id) ON DELETE CASCADE,
    -- State snapshot
    internal_status TEXT,  -- Status when event occurred
    -- Event data
    event_type TEXT NOT NULL,  -- thinking, tool_call, tool_result, text, error
    role TEXT NOT NULL DEFAULT 'agent',  -- agent, system, user
    content TEXT NOT NULL,
    metadata TEXT,  -- JSON
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),

    CHECK ((task_id IS NOT NULL) != (ideation_session_id IS NOT NULL))
);

-- Indexes
CREATE INDEX idx_activity_events_task_id ON activity_events(task_id);
CREATE INDEX idx_activity_events_session_id ON activity_events(ideation_session_id);
CREATE INDEX idx_activity_events_type ON activity_events(event_type);
CREATE INDEX idx_activity_events_created_at ON activity_events(created_at DESC);
```

### Task 2.4: Implement SQLite repository
**Dependencies:** Task 2.2, Task 2.3
**Atomic Commit:** `feat(sqlite): implement ActivityEventRepository`

**Files:**
- `src-tauri/src/infrastructure/sqlite/sqlite_activity_event_repo.rs` - SQLite implementation
- `src-tauri/src/infrastructure/sqlite/mod.rs` - Export repository

### Task 2.5: Wire repository to app state
**Dependencies:** Task 2.4
**Atomic Commit:** `feat(app): wire ActivityEventRepository to app state`

**Files:**
- `src-tauri/src/application/app_state.rs` - Wire repository

### Task 2.6: Add Tauri commands for pagination/filtering (BLOCKING)
**Dependencies:** Task 2.5
**Atomic Commit:** `feat(commands): add activity event pagination commands`

**Files:**
- `src-tauri/src/commands/activity_commands.rs` - Tauri commands
- `src-tauri/src/commands/mod.rs` - Export commands
- `src-tauri/src/lib.rs` - Register commands

### Task 2.7: Persist events when emitting
**Dependencies:** Task 2.5, Part 1
**Atomic Commit:** `feat(streaming): persist activity events to database`

**Files:**
- `src-tauri/src/application/chat_service/chat_service_streaming.rs` - Persist events when emitting

### Pagination Strategy

Cursor-based using `(created_at, id)` tuple:
- Request `limit + 1` to detect `has_more`
- Cursor format: `"2026-01-31T10:30:45+00:00:uuid"`
- Default limit: 50, max: 100

---

## Part 3: Frontend Pagination

**Dependencies:** Part 2 (Task 2.6)
**Atomic Commits:** Multiple tasks below

### Task 3.1: Add API wrapper with Zod schemas (BLOCKING)
**Dependencies:** Task 2.6
**Atomic Commit:** `feat(api): add activity events API wrapper`

**Files:**
- `src/api/activity-events.ts` - API wrapper with Zod schemas

### Task 3.2: Add TanStack Query infinite hook (BLOCKING)
**Dependencies:** Task 3.1
**Atomic Commit:** `feat(hooks): add useActivityEvents infinite query hook`

**Files:**
- `src/hooks/useActivityEvents.ts` - TanStack Query infinite query hook

### Task 3.3: Enhance ActivityView with infinite scroll
**Dependencies:** Task 3.2
**Atomic Commit:** `feat(activity): add infinite scroll and historical mode`

**Files:**
- `src/components/activity/ActivityView.tsx` - Add infinite scroll, historical mode, status filter

### Implementation

1. **Dual-mode operation**:
   - Real-time: Current in-memory events from Zustand (existing)
   - Historical: Load from database with infinite scroll (new)

2. **Infinite scroll**:
```tsx
import { useInView } from 'react-intersection-observer';

const { ref: loadMoreRef, inView } = useInView();
const { data, fetchNextPage, hasNextPage, isFetchingNextPage } = useTaskActivityEvents(taskId, filter);

useEffect(() => {
    if (inView && hasNextPage && !isFetchingNextPage) {
        fetchNextPage();
    }
}, [inView, hasNextPage, isFetchingNextPage]);

// Render sentinel at end of list
{hasNextPage && <div ref={loadMoreRef} className="h-4" />}
```

3. **Filtering UI**: Add status filter dropdown alongside existing type filters

4. **Context-aware filtering**: Read `taskId` from `uiStore.activityFilter` when navigating from chat

---

## Part 4: Unified Status + Activity Widget

**Dependencies:** Part 3 (for full functionality, but can be done in parallel for UI)
**Atomic Commits:** Multiple tasks below

### Problem
- `WorkerExecutingIndicator` only shows in execution mode
- Redundant with header status badge ("Agent responding...")
- Activity link not accessible from other chat types

### Solution: Unified `StatusActivityBadge`

Replace both current Badge AND WorkerExecutingIndicator with single component.

### Task 4.1: Add activityFilter to uiStore (BLOCKING)
**Dependencies:** None (additive)
**Atomic Commit:** `feat(store): add activityFilter state to uiStore`

**Files:**
- `src/stores/uiStore.ts` - Add `activityFilter` state

### Task 4.2: Create StatusActivityBadge component (BLOCKING)
**Dependencies:** Task 4.1
**Atomic Commit:** `feat(chat): create StatusActivityBadge component`

**Files:**
- `src/components/Chat/StatusActivityBadge.tsx` - New unified component

### Task 4.3: Integrate StatusActivityBadge in chat panels
**Dependencies:** Task 4.2
**Atomic Commit:** `feat(chat): replace Badge + WorkerExecutingIndicator with StatusActivityBadge`

**Files:**
- `src/components/Chat/IntegratedChatPanel.tsx` - Replace Badge + remove WorkerExecutingIndicator
- `src/components/Chat/ChatPanel.tsx` - Same changes
- `src/components/Chat/ChatMessageList.tsx` - Remove `isExecutionMode` prop and indicator rendering

### Task 4.4: Wire ActivityView to read filter from store
**Dependencies:** Task 4.1, Task 3.3
**Atomic Commit:** `feat(activity): read filter from uiStore for context-aware navigation`

**Files:**
- `src/App.tsx` - Pass `activityFilter.taskId` to ActivityView

### Behavior by State

| State | Display | Click Action |
|-------|---------|--------------|
| Idle, no activity | Hidden | - |
| Idle, has activity | Muted Activity icon | Navigate to Activity (filtered) |
| Agent active | Badge: "Responding..." + spinner + Activity icon | Navigate to Activity (filtered) |

### Component Structure

```tsx
interface StatusActivityBadgeProps {
    isAgentActive: boolean;      // isSending || isAgentRunning || isExecutionMode
    agentType: "worker" | "reviewer" | "agent" | "idle";
    contextType: ChatContext["view"];
    contextId: string | null;    // taskId or sessionId
}

// Click navigates to Activity with context filter set
const handleClick = () => {
    setActivityFilter({ taskId: contextId });
    setCurrentView("activity");
};
```

---

## Implementation Phases Summary

### Phase 1: Capture Missing Events (Part 1)
**Dependencies:** None
1. Add `Thinking` variant to `StreamEvent`
2. Parse thinking blocks in `StreamProcessor`
3. Emit thinking and tool_result as `AGENT_MESSAGE` events
4. Test with live CLI execution

### Phase 2: Database Persistence (Part 2)
**Dependencies:** Phase 1
1. Create `activity_events` entity and repository (Tasks 2.1, 2.2)
2. Add migration v3 (Task 2.3)
3. Implement SQLite repository and wire to app state (Tasks 2.4, 2.5)
4. Add Tauri commands for pagination (Task 2.6)
5. Persist events in `chat_service_streaming.rs` (Task 2.7)

### Phase 3: Frontend Pagination (Part 3)
**Dependencies:** Phase 2
1. Add API wrapper and TanStack Query hook (Tasks 3.1, 3.2)
2. Enhance ActivityView with infinite scroll (Task 3.3)
3. Add status filter UI
4. Merge real-time + historical events

### Phase 4: Unified Status Widget (Part 4)
**Dependencies:** Can start in parallel after Part 1; full functionality after Part 3
1. Add `activityFilter` to uiStore (Task 4.1)
2. Create `StatusActivityBadge` component (Task 4.2)
3. Replace Badge + WorkerExecutingIndicator in chat panels (Task 4.3)
4. Wire ActivityView to read filter from store (Task 4.4)

---

## Verification

1. **Events captured**: Start worker execution, verify thinking blocks and tool results appear in activity
2. **Persistence**: Refresh app, verify historical events load
3. **Pagination**: Generate 100+ events, verify infinite scroll loads more
4. **Filtering**: Filter by status/type, verify correct results
5. **Widget**: In ideation/task/project chat, verify activity link accessible when agent running
6. **Context filter**: Click activity from task chat, verify events filtered to that task

---

## Critical Files Summary

**Backend (event capture):**
- `src-tauri/src/infrastructure/agents/claude/stream_processor.rs:163` - StreamEvent enum
- `src-tauri/src/application/chat_service/chat_service_streaming.rs:61-210` - Event emission

**Backend (persistence):**
- `src-tauri/src/infrastructure/sqlite/migrations/mod.rs` - Register migration
- `src-tauri/src/application/app_state.rs` - Wire repository

**Frontend (activity):**
- `src/components/activity/ActivityView.tsx` - Main view
- `src/stores/activityStore.ts` - Current in-memory store

**Frontend (chat widget):**
- `src/components/Chat/IntegratedChatPanel.tsx:304-310` - Current Badge
- `src/components/Chat/IntegratedChatPanel.components.tsx:85-118` - WorkerExecutingIndicator

---

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
