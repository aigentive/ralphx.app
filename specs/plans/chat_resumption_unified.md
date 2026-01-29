# Plan: Unify All Chats on Background Processing with Resumption

## Problem Summary

Two issues discovered:

1. **Missing execution_state in unified commands** - `create_chat_service()` doesn't pass `execution_state`, so TaskExecution/Review via `send_agent_message` can't do proper state transitions (logs warning at line 205)

2. **No chat resumption on startup** - When app crashes, only task-based agents resume via `StartupJobRunner`. Ideation, Task, and Project chats are abandoned.

## Current Architecture

All 5 chat types (ideation, task, project, task_execution, review) use the same `ChatService.send_message()` path:

```
Frontend: sendAgentMessage(contextType, contextId, content)
    ↓
Backend: send_agent_message → create_chat_service() → ClaudeChatService
    ↓
ChatService.send_message() → tokio::spawn(spawn_send_message_background)
    ↓
Background: stream processing → transitions → queue processing
```

**The background processing and queue work correctly for all context types.**

The issue is:
- `create_chat_service()` doesn't call `.with_execution_state()`
- No startup runner for interrupted conversations

---

## Implementation Plan

### Step 1: Wire execution_state to Unified Commands

**File:** `src-tauri/src/commands/unified_chat_commands.rs`

Modify `create_chat_service()` to accept and pass execution_state:

```rust
fn create_chat_service(
    state: &AppState,
    app_handle: tauri::AppHandle,
    execution_state: &Arc<ExecutionState>,  // ADD THIS
) -> ClaudeChatService<tauri::Wry> {
    ClaudeChatService::new(
        // ... existing repos ...
    )
    .with_app_handle(app_handle)
    .with_execution_state(Arc::clone(execution_state))  // ADD THIS
}
```

Update all callers to pass execution_state from Tauri state:

```rust
#[tauri::command]
pub async fn send_agent_message(
    input: SendAgentMessageInput,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,  // ADD THIS
    app: tauri::AppHandle,
) -> Result<SendAgentMessageResponse, String> {
    // ...
    let service = create_chat_service(&state, app, &execution_state);  // MODIFY
    // ...
}
```

**Commands to update:**
- `send_agent_message`
- `queue_agent_message`
- `get_queued_agent_messages`
- `delete_queued_agent_message`
- `stop_agent`
- `is_agent_running`

### Step 2: Add Repository Method for Interrupted Conversations

**File:** `src-tauri/src/domain/repositories/agent_run_repository.rs`

Add trait method:
```rust
/// Get conversations that were interrupted during app shutdown
async fn get_interrupted_conversations(&self) -> AppResult<Vec<InterruptedConversation>>;
```

**File:** `src-tauri/src/domain/entities/agent_run.rs`

Add struct:
```rust
pub struct InterruptedConversation {
    pub conversation: ChatConversation,
    pub last_run: AgentRun,
}
```

### Step 3: Implement SQLite Query

**File:** `src-tauri/src/infrastructure/sqlite/sqlite_agent_run_repo.rs`

```sql
SELECT c.*, ar.*
FROM chat_conversations c
INNER JOIN agent_runs ar ON c.id = ar.conversation_id
WHERE c.claude_session_id IS NOT NULL
  AND ar.status = 'cancelled'
  AND ar.error_message = 'Orphaned on app restart'
  AND ar.id = (
    SELECT ar2.id FROM agent_runs ar2
    WHERE ar2.conversation_id = c.id
    ORDER BY ar2.started_at DESC LIMIT 1
  )
```

### Step 4: Create ChatResumptionRunner

**New file:** `src-tauri/src/application/chat_resumption.rs`

Follow `StartupJobRunner` pattern:

```rust
pub struct ChatResumptionRunner<R: Runtime = tauri::Wry> {
    agent_run_repo: Arc<dyn AgentRunRepository>,
    conversation_repo: Arc<dyn ChatConversationRepository>,
    task_repo: Arc<dyn TaskRepository>,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    message_queue: Arc<MessageQueue>,
    running_agent_registry: Arc<RunningAgentRegistry>,
    execution_state: Arc<ExecutionState>,
    app_handle: Option<AppHandle<R>>,
}

impl<R: Runtime> ChatResumptionRunner<R> {
    pub async fn run(&self) {
        // 1. Skip if paused
        if self.execution_state.is_paused() { return; }

        // 2. Get interrupted conversations
        let interrupted = self.agent_run_repo.get_interrupted_conversations().await?;

        // 3. Sort by priority
        let sorted = self.prioritize_resumptions(interrupted);

        // 4. Resume each (skip if handled by task resumption)
        for (conversation, _run) in sorted {
            if self.is_handled_by_task_resumption(&conversation).await {
                continue;
            }

            // Create ChatService and send resume message
            let chat_service = self.create_chat_service();
            let _ = chat_service.send_message(
                conversation.context_type,
                &conversation.context_id,
                "Continue where you left off.",
            ).await;
        }
    }

    fn prioritize_resumptions(&self, convs: Vec<InterruptedConversation>)
        -> Vec<InterruptedConversation> {
        // Priority: TaskExecution > Review > Task > Ideation > Project
    }

    async fn is_handled_by_task_resumption(&self, conv: &ChatConversation) -> bool {
        // TaskExecution/Review with task in AGENT_ACTIVE_STATUSES → skip
    }
}
```

### Step 5: Integrate into Startup Flow

**File:** `src-tauri/src/lib.rs` (after line 184)

```rust
// After StartupJobRunner
runner.run().await;

// Resume interrupted chat conversations (NEW)
let chat_resumption = ChatResumptionRunner::new(
    startup_agent_run_repo.clone(),
    startup_conversation_repo.clone(),
    startup_task_repo.clone(),
    startup_chat_message_repo.clone(),
    startup_project_repo.clone(),
    startup_ideation_session_repo.clone(),
    startup_message_queue.clone(),
    startup_running_agent_registry.clone(),
    Arc::clone(&startup_execution_state),
    Some(startup_app_handle.clone()),
);
chat_resumption.run().await;
```

### Step 6: Add mod.rs Export

**File:** `src-tauri/src/application/mod.rs`

```rust
mod chat_resumption;
pub use chat_resumption::ChatResumptionRunner;
```

---

## Priority Order for Resumption

| Priority | Context Type | Rationale |
|----------|--------------|-----------|
| 1 | TaskExecution | Active task work |
| 2 | Review | Affects task completion |
| 3 | Task | User likely waiting |
| 4 | Ideation | Active brainstorming |
| 5 | Project | General discussion |

---

## Deduplication Logic

**Skip resumption if:**
1. `TaskExecution` or `Review` AND task is in `AGENT_ACTIVE_STATUSES`
   - `StartupJobRunner` handles these via entry actions
2. Conversation has no `claude_session_id` (can't use --resume)

---

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/commands/unified_chat_commands.rs` | Add execution_state parameter to commands |
| `src-tauri/src/domain/repositories/agent_run_repository.rs` | Add trait method |
| `src-tauri/src/domain/entities/agent_run.rs` | Add InterruptedConversation struct |
| `src-tauri/src/infrastructure/sqlite/sqlite_agent_run_repo.rs` | Implement SQL query |
| `src-tauri/src/application/mod.rs` | Export ChatResumptionRunner |
| `src-tauri/src/application/chat_resumption.rs` | **NEW** - Main logic |
| `src-tauri/src/lib.rs` | Wire into startup flow |

---

## Verification

### Unit Tests

1. **execution_state wiring test**
   - Send TaskExecution message via unified API
   - Verify state transition happens (not just warning logged)

2. **interrupted conversations test**
   - Create conversation with claude_session_id
   - Mark agent_run cancelled with orphan message
   - Call get_interrupted_conversations()
   - Verify returned

3. **priority ordering test**
   - Multiple context types interrupted
   - Verify TaskExecution first

4. **deduplication test**
   - TaskExecution conversation + task in Executing state
   - Verify is_handled_by_task_resumption returns true

### Integration Test

1. Start ideation chat, send message
2. Kill app while agent running
3. Restart app
4. Verify chat resumes automatically (agent respawned with --resume)

### Manual Verification

```bash
# 1. Start app, create ideation session, send message
# 2. While agent running, force quit app
# 3. Restart app
# 4. Check logs for "Resuming interrupted conversation..."
# 5. Verify ideation chat shows continued response
```
