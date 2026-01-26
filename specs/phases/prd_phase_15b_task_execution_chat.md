# RalphX - Phase 15B: Task Execution Chat

## Overview

This phase extends the context-aware chat system (Phase 15A) to persist and display worker execution output. Users can view task execution as a chat conversation and queue messages to inject when the worker responds.

**Reference Plan:**
- `specs/plans/task_execution_chat.md` - Complete implementation plan with architecture diagrams and code examples

## Goals

1. Persist worker output to database (not just Activity Stream memory)
2. View task execution as a chat conversation in ChatPanel
3. Queue messages to inject when worker finishes current response
4. Support switching between past execution attempts for a task

## Dependencies

- **Phase 15A must be complete** (context-aware chat implementation)
- Reuses Phase 15A infrastructure:
  - Database schema (`chat_conversations`, `agent_runs`, `chat_messages`)
  - `--resume` pattern for Claude session management
  - Stream parsing and persistence
  - Chat UI components (`ToolCallIndicator`, `QueuedMessage`, `ConversationSelector`)

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/task_execution_chat.md`
2. Also reference `specs/plans/context_aware_chat_implementation.md` for shared infrastructure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write tests where appropriate
4. Run `npm run lint && npm run typecheck` and `cargo test`
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the implementation plan** at `specs/plans/task_execution_chat.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "backend",
    "description": "Add 'task_execution' context type to database and types",
    "plan_section": "Data Model - New Context Type",
    "steps": [
      "Read specs/plans/task_execution_chat.md section 'Data Model'",
      "Update src-tauri/src/domain/entities/chat_conversation.rs:",
      "  - Add TaskExecution variant to ChatContextType enum",
      "Update src/types/chat-conversation.ts:",
      "  - Add 'task_execution' to ChatContextType union",
      "  - Update Zod schema",
      "Run cargo test && npm run typecheck",
      "Commit: feat(types): add task_execution context type"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Create ExecutionMessageQueue for in-memory queue management",
    "plan_section": "Implementation Changes - Add queue management",
    "steps": [
      "Read specs/plans/task_execution_chat.md section 'Add queue management'",
      "Create src-tauri/src/domain/services/execution_message_queue.rs:",
      "  - ExecutionMessageQueue struct with HashMap<TaskId, Vec<QueuedMessage>>",
      "  - queue(task_id, message) method",
      "  - pop(task_id) -> Option<QueuedMessage> method",
      "  - get_queued(task_id) -> Vec<QueuedMessage> method",
      "  - clear(task_id) method",
      "Update mod.rs to export new module",
      "Write unit tests for queue operations",
      "Run cargo test",
      "Commit: feat(domain): add ExecutionMessageQueue for worker message queueing",
      "STOP: Output <promise>COMPLETE</promise> - Next tasks (3-5) are high-complexity cross-cutting changes, consider switching to Opus"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Create ExecutionChatService for spawn with persistence",
    "plan_section": "Implementation Changes - Backend",
    "steps": [
      "Read specs/plans/task_execution_chat.md section 'Implementation Changes - Backend'",
      "Create src-tauri/src/application/execution_chat_service.rs:",
      "  - spawn_with_persistence(agent, task_id) method:",
      "    1. Create chat_conversation (context_type: 'task_execution', context_id: task_id)",
      "    2. Create agent_run (status: 'running')",
      "    3. Spawn Claude CLI with --agent worker",
      "    4. Return handle + conversation_id",
      "  - persist_stream_event(conversation_id, event) method",
      "  - complete_execution(conversation_id, claude_session_id) method",
      "  - Inject ExecutionMessageQueue dependency",
      "Update mod.rs to export new service",
      "Add service to AppState",
      "Write unit tests",
      "Run cargo test",
      "Commit: feat(application): add ExecutionChatService for persistent worker execution"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Modify ClaudeCodeClient to support execution persistence",
    "plan_section": "Implementation Changes - Modify stream processing",
    "steps": [
      "Read specs/plans/task_execution_chat.md section 'Modify stream processing'",
      "Update src-tauri/src/infrastructure/agents/claude/claude_code_client.rs:",
      "  - Add optional persistence callback to spawn method",
      "  - When persistence enabled:",
      "    - Call persist_stream_event for each chunk/tool_call",
      "    - Capture claude_session_id from result event",
      "    - Call completion callback with session_id",
      "  - Continue emitting Activity Stream events (existing behavior)",
      "  - Add new Tauri events: execution:chunk, execution:tool_call, execution:completed",
      "Write tests for persistence flow",
      "Run cargo test",
      "Commit: feat(agents): add persistence support to ClaudeCodeClient stream processing"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Update TransitionHandler to use spawn_with_persistence",
    "plan_section": "Worker Spawn Flow (Phase 15B)",
    "steps": [
      "Read specs/plans/task_execution_chat.md section 'Worker Spawn Flow'",
      "Update src-tauri/src/domain/state_machine/transition_handler.rs:",
      "  - In on_enter(Executing), use ExecutionChatService.spawn_with_persistence",
      "  - Pass task_id and context_type: 'task_execution'",
      "  - Store conversation_id in TaskContext if needed",
      "Note: --permission-prompt-tool flag is already configured in ClaudeCodeClient (Phase 15A)",
      "  - Permission requests during worker execution will trigger PermissionDialog",
      "Ensure backward compatibility (execution still works if service not available)",
      "Run cargo test",
      "Commit: feat(state-machine): use spawn_with_persistence for worker execution",
      "STOP: Output <promise>COMPLETE</promise> - High-complexity tasks complete, can switch back to Sonnet"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Add Tauri commands for execution chat",
    "plan_section": "Files to Create/Modify",
    "steps": [
      "Create src-tauri/src/commands/execution_chat_commands.rs with commands:",
      "  - get_execution_conversation(task_id) -> Option<ChatConversation>",
      "  - list_task_executions(task_id) -> Vec<ChatConversation> (all execution attempts)",
      "  - queue_execution_message(task_id, content) -> QueuedMessage",
      "  - get_queued_execution_messages(task_id) -> Vec<QueuedMessage>",
      "  - delete_queued_execution_message(task_id, message_id)",
      "Register commands in lib.rs invoke_handler",
      "Write tests for commands",
      "Run cargo test",
      "Commit: feat(commands): add execution chat commands"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Implement queue processing on worker completion",
    "plan_section": "User Queues Message to Worker",
    "steps": [
      "Read specs/plans/task_execution_chat.md section 'User Queues Message to Worker'",
      "Update ExecutionChatService completion flow:",
      "  - On worker completion, check ExecutionMessageQueue.pop(task_id)",
      "  - If message exists:",
      "    1. Persist user message to chat_messages",
      "    2. Send via --resume <claude_session_id>",
      "    3. Continue streaming and persisting",
      "  - Repeat until queue empty",
      "  - Emit Tauri event when queue item sent: execution:queue_sent",
      "Write tests for queue processing",
      "Run cargo test",
      "Commit: feat(execution): process message queue on worker completion"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Update chat store for task_execution context",
    "plan_section": "Frontend (React)",
    "steps": [
      "Update src/stores/chatStore.ts:",
      "  - Handle 'task_execution' context type",
      "  - Add executionQueuedMessages state (separate from chat queue)",
      "  - Add queueExecutionMessage action",
      "  - Add deleteExecutionQueuedMessage action",
      "Write tests for new actions",
      "Run npm run typecheck && npm run test",
      "Commit: feat(store): add task_execution context support to chat store"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Update ChatPanel for execution context detection",
    "plan_section": "ChatPanel context detection",
    "steps": [
      "Read specs/plans/task_execution_chat.md section 'User Views Task Execution'",
      "Update src/components/Chat/ChatPanel.tsx:",
      "  - Detect when task.internalStatus === 'executing'",
      "  - Switch to 'task_execution' context mode automatically",
      "  - Show 'Worker is executing...' indicator (different styling)",
      "  - Subscribe to Tauri events: execution:chunk, execution:tool_call, execution:completed",
      "  - Load execution conversation on context switch",
      "  - Enable queue mode when worker is running",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat): detect and display task execution context"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add execution history switching via ConversationSelector",
    "plan_section": "Execution history in ChatPanel",
    "steps": [
      "Update src/components/Chat/ConversationSelector.tsx:",
      "  - When context is 'task_execution', show execution attempts",
      "  - Display: 'Execution #N - <timestamp>' with status (running/completed/failed)",
      "  - Current/running execution highlighted",
      "  - Past executions show outcome (success/failure indicator)",
      "  - Click to switch between past executions",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat): add execution history switching in ConversationSelector"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add execution-specific UI elements",
    "plan_section": "New: Execution-specific UI elements",
    "steps": [
      "Create or update execution-specific UI in ChatPanel:",
      "  - 'Worker is executing...' indicator with pulsing animation",
      "  - Different header styling when in execution mode",
      "  - Optional: Link to Activity Stream for unified view",
      "  - Input placeholder: 'Message worker... (will be sent when current response completes)'",
      "Reuse Phase 15A components for queue display",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat): add execution-specific UI elements"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Update chat API for execution operations",
    "plan_section": "Files to Create/Modify",
    "steps": [
      "Update src/api/chat.ts with new functions:",
      "  - getExecutionConversation(taskId)",
      "  - listTaskExecutions(taskId)",
      "  - queueExecutionMessage(taskId, content)",
      "  - getQueuedExecutionMessages(taskId)",
      "  - deleteQueuedExecutionMessage(taskId, messageId)",
      "Run npm run typecheck",
      "Commit: feat(api): add execution chat API functions"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Wire up execution chat with Activity Stream",
    "plan_section": "Relationship: Activity Stream vs ChatPanel",
    "steps": [
      "Ensure both systems receive worker output:",
      "  - Activity Stream: receives existing events (unchanged)",
      "  - ChatPanel: receives new execution:* events",
      "Verify in code that both event paths work",
      "Both should show same content, just scoped differently",
      "Run cargo test && npm run typecheck",
      "Commit: feat: wire execution chat alongside Activity Stream"
    ],
    "passes": false
  },
  {
    "category": "documentation",
    "description": "Update CLAUDE.md files for Phase 15B",
    "steps": [
      "Update src/CLAUDE.md with:",
      "  - New execution chat context type",
      "  - ExecutionChatService usage",
      "Update src-tauri/CLAUDE.md with:",
      "  - ExecutionChatService",
      "  - ExecutionMessageQueue",
      "  - New commands",
      "Update logs/activity.md with Phase 15B completion summary",
      "Commit: docs: update documentation for task execution chat"
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
| **New conversation per execution attempt** | Preserves clean history; allows switching between attempts |
| **Queue in memory only** | Simpler; queued messages are short-lived; avoids stale messages |
| **User can switch between executions** | ConversationSelector shows execution history for the task |
| **Fresh start on retry (no previous context)** | Simpler; fresh context allows rethinking; user can add context via messages |
| **Complements Activity Stream** | Activity Stream = unified view; ChatPanel = single task deep dive |

---

## Verification Checklist

After completing all tasks:

### Backend
- [ ] 'task_execution' context type works in database
- [ ] ExecutionMessageQueue manages per-task queues
- [ ] ExecutionChatService spawns with persistence
- [ ] Worker output persisted to chat_messages
- [ ] claude_session_id captured on completion
- [ ] Queue processed on worker completion
- [ ] --resume used for queued messages

### Frontend
- [ ] ChatPanel detects 'executing' status and switches to execution mode
- [ ] Worker output displayed as chat messages
- [ ] Tool calls displayed (reusing Phase 15A component)
- [ ] Queue UI works in execution context
- [ ] ConversationSelector shows past execution attempts
- [ ] Can switch between past executions

### Integration
- [ ] Activity Stream still works (unchanged)
- [ ] Both Activity Stream and ChatPanel show same worker output
- [ ] Execution history persists across app restarts
- [ ] Queue messages sent when worker finishes response

### Permission Handling (inherited from Phase 15A)
- [ ] PermissionDialog appears when worker attempts non-approved tool use
- [ ] Allow decision lets worker continue execution
- [ ] Deny decision sends rejection and worker adapts
- [ ] Permission requests work during task execution (not just chat context)
