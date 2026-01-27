# Chat Service Consolidation Plan

## Goal
Consolidate `OrchestratorService` into the `ExecutionChatService` pattern (background spawn, backend queue, reliable state management) while preserving all tested OrchestratorService behavior.

## Current State

### ExecutionChatService (Target Pattern)
- **Location:** `src-tauri/src/application/execution_chat_service.rs`
- **Pattern:** `spawn_with_persistence()` returns immediately, background tokio task processes stream
- **Queue:** Backend `ExecutionMessageQueue` (in-memory, per-task)
- **Resume:** Stores `claude_session_id`, uses `--resume` for continuations
- **Events:** `execution:*` namespace
- **State:** Auto-transitions task `Executing → PendingReview`

### OrchestratorService (To Consolidate)
- **Location:** `src-tauri/src/application/orchestrator_service.rs`
- **Pattern:** `send_message()` blocks until completion
- **Queue:** Frontend Zustand store (not backend)
- **Resume:** Same `claude_session_id` mechanism
- **Events:** `chat:*` namespace
- **State:** No task transitions (advisory only)
- **Agents:** `orchestrator-ideation`, `chat-task`, `chat-project`

## Architecture Decisions

**Create unified `ChatService`** that:
1. Uses background spawn pattern for ALL contexts
2. Has single backend message queue with context-aware routing
3. **Unified event namespace:** All events use `agent:*` prefix (e.g., `agent:message_created`, `agent:run_completed`)
4. Optionally triggers state transitions based on context (TaskExecution only)

**Migration Strategy:**
- Keep old Tauri commands temporarily, delegate to new service
- Add new unified commands
- Gradual frontend migration to new commands
- Remove old commands after verification

## Implementation Plan

### Phase 1: Create Unified Message Queue
**Files:**
- `src-tauri/src/domain/services/message_queue.rs` (new)
- `src-tauri/src/domain/services/mod.rs` (update)

**Tasks:**
1. Create generic `MessageQueue` that handles all context types
2. Queue keyed by `(ChatContextType, context_id)` instead of just `TaskId`
3. Migrate `ExecutionMessageQueue` logic to new unified queue
4. Keep backwards-compatible methods for execution context

```rust
pub struct MessageQueue {
queues: Arc<Mutex<HashMap<QueueKey, Vec<QueuedMessage>>>>,
}

pub struct QueueKey {
context_type: ChatContextType,
context_id: String,
}
```

### Phase 2: Create Unified ChatService Trait
**Files:**
- `src-tauri/src/application/chat_service.rs` (new)
- `src-tauri/src/application/mod.rs` (update)

**Tasks:**
1. Define unified trait combining both service capabilities
2. Support all context types: Ideation, Task, Project, TaskExecution
3. Background spawn with immediate return for all contexts
4. Context-aware agent selection

```rust
pub trait ChatService: Send + Sync {
async fn send_message(
&self,
context_type: ChatContextType,
context_id: &str,
message: &str,
) -> Result<SendResult, ChatServiceError>;

async fn get_conversation(
&self,
context_type: ChatContextType,
context_id: &str,
) -> Result<Option<ChatConversation>, ChatServiceError>;

async fn list_conversations(
&self,
context_type: ChatContextType,
context_id: &str,
) -> Result<Vec<ChatConversation>, ChatServiceError>;

async fn queue_message(
&self,
context_type: ChatContextType,
context_id: &str,
content: &str,
) -> Result<QueuedMessage, ChatServiceError>;

async fn get_queued_messages(
&self,
context_type: ChatContextType,
context_id: &str,
) -> Result<Vec<QueuedMessage>, ChatServiceError>;

async fn delete_queued_message(
&self,
context_type: ChatContextType,
context_id: &str,
message_id: &str,
) -> Result<bool, ChatServiceError>;
}
```

### Phase 3: Implement Unified Service
**Files:**
- `src-tauri/src/application/chat_service.rs` (continue)

**Tasks:**
1. Implement `ClaudeChatService` struct with all dependencies
2. Port `spawn_with_persistence` logic from ExecutionChatService
3. Port agent selection from OrchestratorService
4. **Unified event namespace:** All contexts emit `agent:*` events
5. Background queue processing for ALL contexts
6. Task state transitions only for TaskExecution context

**Unified Event Names:**
- `agent:run_started` - Agent begins processing
- `agent:chunk` - Streaming text chunk
- `agent:tool_call` - Tool invocation (start/complete/result)
- `agent:message_created` - Message persisted
- `agent:run_completed` - Agent finished successfully
- `agent:error` - Agent failed

**Event Payload includes context_type:**
```rust
pub struct AgentEventPayload {
pub context_type: String,      // "ideation" | "task" | "project" | "task_execution"
pub context_id: String,
pub conversation_id: String,
// ... other fields
}
```

**Key Implementation Details:**
```rust
impl ClaudeChatService {
async fn send_message(&self, context_type, context_id, message) {
// 1. Get/create conversation
// 2. Create AgentRun
// 3. Persist user message
// 4. Emit agent:run_started
// 5. Spawn CLI with appropriate agent
// 6. Return SendResult immediately
// 7. Background task: process stream, persist, process queue
// 8. If TaskExecution: transition task state
}

fn get_agent_name(&self, context_type: &ChatContextType) -> &str {
match context_type {
ChatContextType::Ideation => "orchestrator-ideation",
ChatContextType::Task => "chat-task",
ChatContextType::Project => "chat-project",
ChatContextType::TaskExecution => "worker",
}
}
}
```

### Phase 4: Update Tauri Commands
**Files:**
- `src-tauri/src/commands/unified_chat_commands.rs` (new)
- `src-tauri/src/commands/context_chat_commands.rs` (update - delegate to new service)
- `src-tauri/src/commands/execution_chat_commands.rs` (update - delegate to new service)
- `src-tauri/src/commands/mod.rs` (update)
- `src-tauri/src/lib.rs` (update command registration)

**Migration Strategy:**
1. Create new unified commands
2. Update OLD commands to delegate to new ChatService (backwards compatible)
3. Frontend can gradually migrate to new commands
4. Remove old commands after frontend fully migrated

**New Unified Commands:**
```rust
#[tauri::command]
async fn send_agent_message(context_type: String, context_id: String, content: String) -> Result<SendResult>

#[tauri::command]
async fn queue_agent_message(context_type: String, context_id: String, content: String) -> Result<QueuedMessage>

#[tauri::command]
async fn get_queued_agent_messages(context_type: String, context_id: String) -> Result<Vec<QueuedMessage>>

#[tauri::command]
async fn delete_queued_agent_message(context_type: String, context_id: String, message_id: String) -> Result<bool>

#[tauri::command]
async fn list_agent_conversations(context_type: String, context_id: String) -> Result<Vec<ChatConversation>>

#[tauri::command]
async fn get_agent_conversation(conversation_id: String) -> Result<ConversationWithMessages>
```

**Old Commands (delegate to new service):**
```rust
// context_chat_commands.rs
#[tauri::command]
async fn send_context_message(...) {
// Delegate to chat_service.send_message(...)
}

// execution_chat_commands.rs
#[tauri::command]
async fn queue_execution_message(...) {
// Delegate to chat_service.queue_message(TaskExecution, task_id, ...)
}
```

### Phase 5: Update Frontend API
**Files:**
- `src/api/chat.ts` (update)

**Tasks:**
1. Add new API functions for unified commands (keep old for compatibility)
2. Remove frontend queue for ideation/task/project contexts
3. Keep queue in Zustand for optimistic UI only
4. All queue operations now call backend

**Key Changes:**
```typescript
// New unified API functions
export async function sendAgentMessage(contextType: string, contextId: string, content: string)
export async function queueAgentMessage(contextType: string, contextId: string, content: string)
export async function getQueuedAgentMessages(contextType: string, contextId: string)
export async function deleteQueuedAgentMessage(contextType: string, contextId: string, messageId: string)

// Old functions continue to work (delegate in backend)
export async function sendContextMessage(...) // Still works
export async function queueExecutionMessage(...) // Still works
```

### Phase 6: Update Frontend Event Listeners
**Files:**
- `src/hooks/useChat.ts` (update)
- `src/components/Chat/ChatPanel.tsx` (update)

**Tasks:**
1. Replace `chat:*` and `execution:*` listeners with single `agent:*` listeners
2. Filter events by context_type in payload
3. Remove frontend queue processing logic (backend handles it)
4. Simplify run_completed handler

**Event Listener Changes:**
```typescript
// Before: Two separate listener sets
listen("chat:tool_call", handler)
listen("execution:tool_call", handler)
listen("chat:run_completed", handler)
listen("execution:run_completed", handler)

// After: Single unified listeners with context filtering
listen("agent:tool_call", (event) => {
if (event.payload.context_type === currentContextType) {
// Handle
}
})
listen("agent:run_completed", (event) => {
if (event.payload.context_type === currentContextType) {
// Handle
}
})
```

### Phase 7: Deprecate Old Services
**Files:**
- `src-tauri/src/application/orchestrator_service.rs` (deprecate)
- `src-tauri/src/application/execution_chat_service.rs` (deprecate)
- `src-tauri/src/domain/services/execution_message_queue.rs` (deprecate)

**Tasks:**
1. Mark old services as deprecated
2. Keep temporarily for reference
3. Update all imports to use new ChatService
4. Remove after verification complete

### Phase 8: Update AppState
**Files:**
- `src-tauri/src/state.rs` (update)

**Tasks:**
1. Replace `orchestrator_service` and `execution_chat_service` with single `chat_service`
2. Replace `execution_message_queue` with unified `message_queue`
3. Update all state access patterns

## Verification Checklist

### Ideation Context (Previously OrchestratorService)
- [ ] Can send message in ideation session
- [ ] Messages persist to database
- [ ] Claude session ID captured and stored
- [ ] Resume works (second message uses --resume)
- [ ] `agent:message_created` events emitted with context_type="ideation"
- [ ] `agent:run_completed` events emitted
- [ ] `agent:tool_call` events emitted for tool usage
- [ ] Agent uses `orchestrator-ideation`
- [ ] MCP tools scoped correctly (RALPHX_AGENT_TYPE)
- [ ] Queue works: can queue while agent running
- [ ] Queued messages processed after completion
- [ ] Queued messages use --resume

### Task Context (Previously OrchestratorService)
- [ ] Can send message about task
- [ ] Messages persist with task_id
- [ ] Agent uses `chat-task`
- [ ] Events have context_type="task"
- [ ] All queue behavior same as ideation

### Project Context (Previously OrchestratorService)
- [ ] Can send message about project
- [ ] Agent uses `chat-project`
- [ ] Events have context_type="project"
- [ ] All queue behavior same as ideation

### TaskExecution Context (Previously ExecutionChatService)
- [ ] spawn returns immediately with IDs
- [ ] Background processing works
- [ ] `agent:*` events emitted with context_type="task_execution"
- [ ] Task transitions Executing → PendingReview
- [ ] Queue works (keyed by task_id)
- [ ] Agent uses `worker`

### Frontend Integration
- [ ] ChatPanel receives `agent:*` events correctly
- [ ] Messages display in real-time
- [ ] Tool calls show in UI
- [ ] Queue UI works for all contexts
- [ ] Conversation switching works
- [ ] Context type filtering works (only show relevant events)
- [ ] No regressions in ideation flow
- [ ] No regressions in task chat flow
- [ ] No regressions in project chat flow
- [ ] Old commands still work (backwards compatibility)

## File Changes Summary

### Backend (Rust)
| File | Action |
|------|--------|
| `src-tauri/src/domain/services/message_queue.rs` | CREATE - Unified queue |
| `src-tauri/src/domain/services/mod.rs` | UPDATE - Export message_queue |
| `src-tauri/src/application/chat_service.rs` | CREATE - Unified service |
| `src-tauri/src/application/mod.rs` | UPDATE - Export chat_service |
| `src-tauri/src/commands/unified_chat_commands.rs` | CREATE - New commands |
| `src-tauri/src/commands/context_chat_commands.rs` | UPDATE - Delegate to new service |
| `src-tauri/src/commands/execution_chat_commands.rs` | UPDATE - Delegate to new service |
| `src-tauri/src/commands/mod.rs` | UPDATE - Export new commands |
| `src-tauri/src/lib.rs` | UPDATE - Register new commands |
| `src-tauri/src/state.rs` | UPDATE - Add chat_service, message_queue |

### Frontend (TypeScript)
| File | Action |
|------|--------|
| `src/api/chat.ts` | UPDATE - Add unified API functions |
| `src/hooks/useChat.ts` | UPDATE - Listen to agent:* events |
| `src/components/Chat/ChatPanel.tsx` | UPDATE - Listen to agent:* events |
| `src/stores/chatStore.ts` | UPDATE - Simplify queue (optimistic only) |

### Deprecate Later
| File | Action |
|------|--------|
| `src-tauri/src/application/orchestrator_service.rs` | DEPRECATE after migration |
| `src-tauri/src/application/execution_chat_service.rs` | DEPRECATE after migration |
| `src-tauri/src/domain/services/execution_message_queue.rs` | DEPRECATE after migration |

## Risks & Mitigations

1. **Event namespace change requires frontend updates**
- Mitigation: Update all frontend listeners to `agent:*` namespace
- Mitigation: Include context_type in payload for filtering
- Mitigation: Test all contexts thoroughly

2. **Old commands must continue working during migration**
- Mitigation: Old commands delegate to new service internally
- Mitigation: No breaking changes to existing API signatures

3. **Queue behavior differences between contexts**
- Mitigation: Unified queue with (context_type, context_id) key structure
- Mitigation: Same QueuedMessage type for all contexts

4. **State transition side effects**
- Mitigation: Only TaskExecution triggers transitions (preserve existing behavior)
- Mitigation: Explicit context_type check before transition

5. **Tested OrchestratorService flows break**
- Mitigation: Comprehensive verification checklist above
- Mitigation: Test each context type independently

## Implementation Order

1. Phase 1: Unified Message Queue (foundation)
2. Phase 2-3: Unified ChatService (core logic)
3. Phase 4: Tauri Commands (backend interface)
4. Phase 5-6: Frontend updates (last, after backend stable)
5. Phase 7-8: Cleanup (after verification)

Each phase should be verified before proceeding to next.
