# Task Execution Chat Implementation Plan

## Problem

When a task is executing (worker agent running), users have limited visibility and no interaction:

1. **Activity Stream** shows live output but only stores in memory (100 max, lost on refresh)
2. User **cannot see past execution** - no persistence
3. User **cannot interact** with a running worker (only pause/stop the whole execution)
4. Multiple tasks can run in parallel, but Activity Stream shows everything mixed together

## Goals

1. **Persist worker output** - Store thinking, tool calls, results in database
2. **View execution as chat** - User can open ChatPanel for a task and see its execution conversation
3. **Queue messages to worker** - User can send messages that get injected when worker finishes current response
4. **Complement Activity Stream** - Activity Stream remains the unified view; ChatPanel is the task-specific view

## What This Does NOT Change

The existing execution layer stays the same:

| Component | Behavior (unchanged) |
|-----------|---------------------|
| `ExecutionState` | Manages pause/resume/stop, running counts |
| Task scheduler | Picks up Ready tasks when capacity available |
| `TransitionHandler` | Spawns worker via `agent_spawner.spawn("worker", task_id)` on Executing entry |
| Activity Stream | Shows real-time unified view of all running tasks |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         EXISTING LAYER (unchanged)                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Task enters Executing → TransitionHandler.on_enter(Executing)              │
│                                   │                                          │
│                                   ▼                                          │
│                     agent_spawner.spawn("worker", task_id)                  │
│                                   │                                          │
│                                   ▼                                          │
│                     Claude CLI spawned with --agent worker                  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ Phase 15B adds capture here
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         NEW LAYER (Phase 15B)                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. Capture claude_session_id from worker spawn                             │
│  2. Create chat_conversation for this task execution                        │
│  3. Stream worker output → persist to chat_messages (+ emit to Activity)    │
│  4. User can view in ChatPanel (task context)                               │
│  5. User can queue messages → sent via --resume when worker responds        │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Relationship: Activity Stream vs ChatPanel

| Aspect | Activity Stream | ChatPanel (Task Context) |
|--------|-----------------|--------------------------|
| **Scope** | All running tasks | Single task |
| **Storage** | Memory only (ring buffer) | Database (persistent) |
| **After refresh** | Lost | Preserved |
| **User interaction** | View only | Can queue messages |
| **Purpose** | Monitor everything at a glance | Deep dive into one task |

**They complement each other:**
- Activity Stream = "What's happening right now across all tasks?"
- ChatPanel = "Show me the full conversation for this specific task"

---

## Data Model

### Reusing Phase 15A Infrastructure

Phase 15B uses the same tables from Phase 15A:

```sql
-- chat_conversations: Links task execution to Claude session
-- context_type = 'task_execution' (new type for executions)
-- context_id = task_id

-- agent_runs: Tracks running/completed status
-- Same as Phase 15A

-- chat_messages: Stores worker output
-- role = 'assistant' for worker output
-- role = 'user' for injected messages
-- tool_calls = JSON of tool calls made
```

### New Context Type

```typescript
type ChatContextType =
  | 'ideation'        // Phase 15A: Ideation sessions
  | 'task'            // Phase 15A: Task chat (questions about a task)
  | 'project'         // Phase 15A: Project-level chat
  | 'task_execution'; // Phase 15B: Worker execution for a task
```

---

## Flow Diagrams

### Worker Spawn (Modified)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     WORKER SPAWN FLOW (Phase 15B)                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  TransitionHandler: task enters Executing                                   │
│        │                                                                     │
│        ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │ 1. Create chat_conversation:                                     │        │
│  │    - context_type: 'task_execution'                             │        │
│  │    - context_id: task_id                                        │        │
│  │    - claude_session_id: null (will capture from spawn)          │        │
│  │                                                                  │        │
│  │ 2. Create agent_run:                                            │        │
│  │    - conversation_id: from above                                │        │
│  │    - status: 'running'                                          │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│        │                                                                     │
│        ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │ 3. Spawn Claude CLI:                                            │        │
│  │    claude --agent worker \                                      │        │
│  │           --plugin-dir ./ralphx-plugin \                        │        │
│  │           --output-format stream-json \                         │        │
│  │           -p "Task: {task_details}"                             │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│        │                                                                     │
│        ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │ 4. Stream Processing:                                           │        │
│  │    - Parse stream-json events                                   │        │
│  │    - Persist to chat_messages (thinking, tool_calls, text)      │        │
│  │    - Emit Tauri events for Activity Stream (existing behavior)  │        │
│  │    - Emit Tauri events for ChatPanel (new: chat:chunk, etc.)    │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│        │                                                                     │
│        ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │ 5. On completion:                                               │        │
│  │    - Capture claude_session_id from result event                │        │
│  │    - Update conversation.claude_session_id                      │        │
│  │    - Update agent_run.status = 'completed'                      │        │
│  │    - Check for queued messages → send via --resume              │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### User Views Task Execution

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     USER VIEWS TASK EXECUTION                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  User selects task in Kanban (task is Executing)                            │
│        │                                                                     │
│        ▼                                                                     │
│  User opens ChatPanel (or it auto-opens when task selected)                 │
│        │                                                                     │
│        ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │ ChatPanel detects: task.internal_status == 'Executing'          │        │
│  │                                                                  │        │
│  │ → Switches to 'task_execution' context mode                     │        │
│  │ → Loads conversation for this task execution                    │        │
│  │ → Subscribes to live stream events                              │        │
│  │ → Shows "Worker is executing..." indicator                      │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│        │                                                                     │
│        ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │ Chat UI shows:                                                   │        │
│  │                                                                  │        │
│  │  [Worker] Reading the task requirements...          10:30:01    │        │
│  │           ┌─────────────────────────────────────┐               │        │
│  │           │ 🔧 Read                              │               │        │
│  │           │    path: src/components/...         │               │        │
│  │           └─────────────────────────────────────┘               │        │
│  │           I see the existing implementation...                  │        │
│  │                                                                  │        │
│  │  [Worker] Writing tests first as per TDD...      10:30:15      │        │
│  │           ┌─────────────────────────────────────┐               │        │
│  │           │ 🔧 Write                             │               │        │
│  │           │    path: src/components/...test.tsx │               │        │
│  │           └─────────────────────────────────────┘               │        │
│  │                                                                  │        │
│  │  ••• Worker is executing...                                     │        │
│  │                                                                  │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### User Queues Message to Worker

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     USER QUEUES MESSAGE TO WORKER                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  User is viewing task execution in ChatPanel                                │
│  Worker is still running (agent_run.status == 'running')                    │
│        │                                                                     │
│        ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │ User types: "Also add error handling for edge cases"            │        │
│  │ User presses Enter                                              │        │
│  │                                                                  │        │
│  │ Since worker is running → message is QUEUED (not sent yet)      │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│        │                                                                     │
│        ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │ Chat UI shows:                                                   │        │
│  │                                                                  │        │
│  │  [Worker] ... still working ...                                 │        │
│  │                                                                  │        │
│  │  ┌────────────────────────────────────────────────────────────┐ │        │
│  │  │ QUEUED (will be sent when worker responds)                 │ │        │
│  │  │ 📤 Also add error handling for edge cases      [✏️] [×]   │ │        │
│  │  └────────────────────────────────────────────────────────────┘ │        │
│  │                                                                  │        │
│  │  [Type a message... (will be queued)]                           │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│        │                                                                     │
│        │ Worker finishes current response                                   │
│        ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │ Backend:                                                         │        │
│  │ 1. Worker response complete                                     │        │
│  │ 2. Check queue → found message                                  │        │
│  │ 3. Send via --resume <claude_session_id>:                       │        │
│  │    claude --resume <session_id> \                               │        │
│  │           --plugin-dir ./ralphx-plugin \                        │        │
│  │           --output-format stream-json \                         │        │
│  │           -p "Also add error handling for edge cases"           │        │
│  │ 4. Continue streaming...                                        │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Changes

### Backend (Rust)

**1. Modify `AgentSpawner` trait/implementation**

Current:
```rust
// Just spawns and returns handle
async fn spawn(&self, agent: &str, task_id: &str) -> AgentHandle;
```

New (or new wrapper):
```rust
// Spawns, creates conversation, captures session, persists output
async fn spawn_with_persistence(
    &self,
    agent: &str,
    task_id: &str,
    context_type: ChatContextType,
) -> AgentHandle;
```

**2. Modify stream processing in `ClaudeCodeClient`**

Add persistence layer that:
- Creates `chat_conversation` before spawn
- Creates `agent_run` with status "running"
- Persists each chunk/tool_call to `chat_messages`
- Captures `claude_session_id` from result event
- Updates `agent_run` status on completion
- Checks and processes message queue

**3. Add queue management**

```rust
pub struct ExecutionMessageQueue {
    // Queued messages per task_id
    queues: HashMap<TaskId, Vec<QueuedMessage>>,
}

impl ExecutionMessageQueue {
    pub fn queue(&mut self, task_id: &TaskId, message: String);
    pub fn pop(&mut self, task_id: &TaskId) -> Option<QueuedMessage>;
    pub fn get_queued(&self, task_id: &TaskId) -> Vec<QueuedMessage>;
}
```

### Frontend (React)

**1. ChatPanel context detection**

```typescript
// When task is selected and task.internalStatus === 'Executing'
// → Use 'task_execution' context instead of 'task' context
// → Show execution conversation
// → Enable queue mode for input
```

**2. Reuse Phase 15A components**

- `ToolCallIndicator` - Already built for Phase 15A
- `QueuedMessage` - Already built for Phase 15A
- `QueuedMessageList` - Already built for Phase 15A
- `ConversationSelector` - Could show execution history for task

**3. New: Execution-specific UI elements**

- "Worker is executing..." indicator (different from "Agent is responding...")
- Progress hints (if available from worker output)
- Link to Activity Stream for unified view

---

## Files to Create/Modify

### New Files

| File | Purpose |
|------|---------|
| `src-tauri/src/application/execution_chat_service.rs` | Service for execution chat (spawn with persistence, queue) |
| `src-tauri/src/domain/services/execution_message_queue.rs` | Queue management for execution messages |

### Modified Files

| File | Change |
|------|--------|
| `src-tauri/src/infrastructure/agents/claude/client.rs` | Add persistence to stream processing |
| `src-tauri/src/domain/state_machine/transition_handler.rs` | Use new spawn_with_persistence in on_enter(Executing) |
| `src-tauri/src/domain/state_machine/services.rs` | Update AgentSpawner trait if needed |
| `src/components/Chat/ChatPanel.tsx` | Detect execution context, show execution UI, switch between executions |
| `src/stores/chatStore.ts` | Handle 'task_execution' context type |
| `src/types/chat-conversation.ts` | Add 'task_execution' to ChatContextType |

---

## Resolved Design Decisions

### 1. Multiple executions per task

**Decision: A) Create a new conversation per execution attempt**

Each execution attempt gets its own `chat_conversation` record. This:
- Preserves clean history of each attempt
- Allows fresh Claude context window for each retry
- Enables switching between past executions in UI

### 2. Queue persistence

**Decision: A) In memory only**

Queued messages are held in memory and lost on app restart. Rationale:
- Simpler implementation
- Queued messages are short-lived (sent when worker responds)
- If app restarts, user can re-type their message
- Avoids stale queued messages from days-old sessions

### 3. Execution history in ChatPanel

**Decision: B) User can switch between past executions**

The `ConversationSelector` component (from Phase 15A) shows all executions for the selected task:
- Current/running execution highlighted
- Past executions shown with timestamps and outcome
- Clicking switches the view

### 4. Worker agent awareness of previous failures

**Decision: A) Just the task details (fresh start)**

Each retry gets a completely fresh context with only the task details. Rationale:
- Simpler implementation
- Fresh context allows the agent to rethink the problem
- Avoids context bloat from previous execution logs
- User can manually add context via queued messages if needed
- Past executions are still viewable in UI via ConversationSelector

---

## Implementation Order

1. **Database** - Add 'task_execution' context type support (reuses Phase 15A schema)
2. **Backend service** - Create `execution_chat_service.rs` with spawn_with_persistence
3. **Stream persistence** - Modify Claude client to persist execution output
4. **TransitionHandler** - Use new spawn method in on_enter(Executing)
5. **Frontend context detection** - ChatPanel detects execution mode
6. **Execution selector** - Add past execution switching via ConversationSelector
7. **Queue UI** - Reuse Phase 15A queue components for execution context
8. **Testing** - End-to-end test: viewing, message injection, execution history

---

## Dependencies

**Phase 15A is a prerequisite for this work.**

| Resource | Path | Purpose |
|----------|------|---------|
| Phase 15A Plan | [`specs/plans/context_aware_chat_implementation.md`](../plans/context_aware_chat_implementation.md) | Full implementation details |
| Phase 15A PRD | [`specs/phases/prd_phase_15_context_aware_chat.md`](../phases/prd_phase_15_context_aware_chat.md) | Task checklist |

**What Phase 15A provides:**
- Database schema (`chat_conversations`, `agent_runs`, `chat_messages` tables)
- `--resume` pattern for Claude session management
- `--output-format stream-json` parsing
- Message queue system (frontend and backend)
- Chat UI components (`ToolCallIndicator`, `QueuedMessage`, `QueuedMessageList`, `ConversationSelector`)
- Context-aware chat service foundation

Phase 15B extends this infrastructure with the `'task_execution'` context type.
