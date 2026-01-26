# RalphX - Phase 15: Context-Aware Chat System

## Overview

This phase implements the context-aware chat system with MCP integration, Claude session management (`--resume`), conversation history, message queueing, and real-time streaming persistence.

**Reference Plan:**
- `specs/plans/context_aware_chat_implementation.md` - Complete implementation plan with architecture diagrams, code examples, and detailed specifications

## Goals

1. Create MCP server (TypeScript proxy) to expose RalphX tools to Claude
2. Implement `--resume` flag support for Claude session persistence
3. Add real-time message streaming with persistence (leave and come back)
4. Implement conversation history selector (multiple conversations per context)
5. Implement message queue system (queue while agent running)
6. Display tool calls in chat UI (collapsible view)

## Dependencies

- Phase 14 must be complete (design implementation finished)
- ChatPanel component exists (from Phase 14)
- Orchestrator service exists (will be refactored)
- SQLite database infrastructure in place

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/context_aware_chat_implementation.md`
2. Understand the architecture (MCP proxy pattern, session flow, event system)
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications and code examples
3. Write tests where appropriate
4. Run `npm run lint && npm run typecheck` and `cargo test`
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/context_aware_chat_implementation.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "database",
    "description": "Create database migration for chat conversations, agent runs, and tool calls",
    "plan_section": "4.1 Database Schema Update",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation.md section '4.1 Database Schema Update'",
      "Create migration file: src-tauri/src/infrastructure/sqlite/migrations.rs (add new migration)",
      "Add chat_conversations table (id, context_type, context_id, claude_session_id, title, message_count, last_message_at, timestamps)",
      "Add agent_runs table (id, conversation_id, status, started_at, completed_at, error_message)",
      "Add conversation_id and tool_calls columns to chat_messages table",
      "Add indexes for conversation lookups",
      "Add trigger for message_count updates",
      "Run cargo test to verify migration applies",
      "Commit: feat(db): add chat conversations, agent runs, and tool calls schema"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Create ChatConversation and AgentRun entities and repositories",
    "plan_section": "Files to Create/Modify - Backend (Rust)",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation.md for entity definitions",
      "Create src-tauri/src/domain/entities/chat_conversation.rs with ChatConversation struct",
      "Create src-tauri/src/domain/entities/agent_run.rs with AgentRun struct and AgentRunStatus enum",
      "Update src-tauri/src/domain/entities/ideation.rs - add tool_calls field to ChatMessage",
      "Create src-tauri/src/domain/repositories/chat_conversation_repo.rs trait",
      "Create src-tauri/src/domain/repositories/agent_run_repo.rs trait",
      "Create SQLite implementations in infrastructure/sqlite/",
      "Update mod.rs files to export new modules",
      "Add repositories to AppState",
      "Write unit tests for repositories",
      "Run cargo test",
      "Commit: feat(domain): add ChatConversation and AgentRun entities with repositories"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add HTTP server to Tauri backend for MCP proxy",
    "plan_section": "2. Add HTTP Server to Tauri Backend",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation.md section '2. Add HTTP Server to Tauri Backend'",
      "Add axum and tower-http dependencies to src-tauri/Cargo.toml",
      "Create src-tauri/src/http_server.rs with axum routes",
      "Implement POST endpoints for MCP tools: /api/create_task_proposal, /api/update_task_proposal, /api/complete_review, etc.",
      "Each endpoint reuses existing service logic (ideation_service, task_repo)",
      "Start HTTP server on app launch in lib.rs (port 3847)",
      "Test endpoints with curl",
      "Run cargo test",
      "Commit: feat(backend): add HTTP server for MCP proxy (port 3847)"
    ],
    "passes": true
  },
  {
    "category": "mcp",
    "description": "Create RalphX MCP Server (TypeScript proxy)",
    "plan_section": "1. Create RalphX MCP Server (Proxy to Tauri Backend)",
    "steps": [
      "FIRST: Use the mcp-builder skill by running: /mcp-builder",
      "Read specs/plans/context_aware_chat_implementation.md section '1. Create RalphX MCP Server'",
      "Create ralphx-mcp-server/ directory",
      "Create package.json with @modelcontextprotocol/sdk dependency",
      "Create tsconfig.json",
      "Create src/index.ts - MCP server entry point (follow mcp-builder guidance)",
      "Create src/tauri-client.ts - HTTP client to call Tauri backend",
      "Create src/tools.ts - Tool definitions (create_task_proposal, update_task, add_task_note, complete_review, etc.)",
      "All tools forward to Tauri backend via HTTP (no business logic in MCP server)",
      "Build and test standalone: npm run build",
      "Commit: feat(mcp): create RalphX MCP server proxy"
    ],
    "passes": true
  },
  {
    "category": "mcp",
    "description": "Implement MCP tool scoping based on agent type",
    "plan_section": "MCP Tool Scoping",
    "steps": [
      "Reference the mcp-builder skill (/mcp-builder) for MCP best practices if needed",
      "Read specs/plans/context_aware_chat_implementation.md section 'MCP Tool Scoping'",
      "Add TOOL_ALLOWLIST constant to ralphx-mcp-server/src/tools.ts or index.ts:",
      "  - orchestrator-ideation: [create_task_proposal, update_task_proposal, delete_task_proposal, add_proposal_dependency]",
      "  - chat-task: [update_task, add_task_note, get_task_details]",
      "  - chat-project: [suggest_task, list_tasks]",
      "  - reviewer: [complete_review] (submit review decision: approved/needs_changes/escalate)",
      "  - worker, supervisor, qa-prep, qa-tester: [] (no MCP tools)",
      "Read RALPHX_AGENT_TYPE from process.env in MCP server",
      "Create getAllowedToolNames() helper function",
      "Update ListToolsRequestSchema handler: filter ALL_TOOLS by allowlist",
      "Update CallToolRequestSchema handler: reject unauthorized calls with clear error message",
      "Test manually: set RALPHX_AGENT_TYPE=worker and verify no tools returned",
      "Test manually: set RALPHX_AGENT_TYPE=orchestrator-ideation and verify only ideation tools returned",
      "Commit: feat(mcp): implement tool scoping based on agent type"
    ],
    "passes": true
  },
  {
    "category": "mcp",
    "description": "Add permission_request tool to MCP server for UI-based permission handling",
    "plan_section": "Permission Bridge System - Permission Handler MCP Tool",
    "steps": [
      "Reference the mcp-builder skill (/mcp-builder) for MCP tool best practices",
      "Read specs/plans/context_aware_chat_implementation/permission_bridge.md",
      "Create ralphx-mcp-server/src/permission-handler.ts:",
      "  - Export permissionRequestTool definition (name: 'permission_request')",
      "  - Export handlePermissionRequest function that:",
      "    1. POSTs to Tauri /api/permission/request",
      "    2. Long-polls /api/permission/await/:request_id (5 min timeout)",
      "    3. Returns allow/deny decision to Claude CLI",
      "Update ralphx-mcp-server/src/index.ts:",
      "  - Import and register permission_request tool",
      "  - Handle in CallToolRequestSchema (NOT scoped by agent type - always available)",
      "Build and verify: npm run build",
      "Commit: feat(mcp): add permission_request tool for UI-based approval"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add PermissionState and permission HTTP endpoints to Tauri backend",
    "plan_section": "Permission Bridge System - Tauri Backend",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation/permission_bridge.md",
      "Create src-tauri/src/application/permission_state.rs:",
      "  - PermissionState struct with pending: Mutex<HashMap<String, watch::Sender<...>>>",
      "  - PermissionDecision struct",
      "Update src-tauri/src/http_server.rs - add permission endpoints:",
      "  - POST /api/permission/request: registers pending request, emits Tauri event, returns request_id",
      "  - GET /api/permission/await/:request_id: long-polls until decision (5 min timeout -> 408)",
      "  - POST /api/permission/resolve: signals waiting request with decision",
      "Initialize PermissionState in AppState (lib.rs)",
      "Write tests for permission state and endpoints",
      "Run cargo test",
      "Commit: feat(backend): add permission state and HTTP endpoints for permission bridge"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add Tauri commands for permission resolution",
    "plan_section": "Permission Bridge System - Tauri Command for Frontend",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation/permission_bridge.md",
      "Create src-tauri/src/commands/permission_commands.rs:",
      "  - resolve_permission_request(request_id, decision, message) command",
      "  - get_pending_permissions() -> Vec<PendingPermissionInfo> command",
      "Register commands in lib.rs invoke_handler",
      "Write unit tests for commands",
      "Run cargo test",
      "Commit: feat(commands): add permission resolution commands"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Update ClaudeCodeClient to use --permission-prompt-tool flag",
    "plan_section": "Permission Bridge System - Update Claude CLI Spawn",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation/permission_bridge.md",
      "Update src-tauri/src/infrastructure/agents/claude/claude_code_client.rs:",
      "  - Add --permission-prompt-tool mcp__ralphx__permission_request to spawn args",
      "  - This enables UI-based permission approval for non-pre-approved tools",
      "Run cargo test",
      "Commit: feat(agents): add --permission-prompt-tool flag for UI-based approval"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create permission types",
    "plan_section": "Permission Bridge System - Files Summary",
    "steps": [
      "Create src/types/permission.ts:",
      "  - PermissionRequest interface (request_id, tool_name, tool_input, context)",
      "  - PermissionDecision type ('allow' | 'deny')",
      "  - Zod schemas for validation",
      "Run npm run typecheck",
      "Commit: feat(types): add permission request types"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create PermissionDialog component",
    "plan_section": "Permission Bridge System - Frontend: Permission Dialog Component",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation/permission_bridge.md",
      "Create src/components/PermissionDialog.tsx:",
      "  - Listen to Tauri event 'permission:request'",
      "  - Queue multiple requests (show first, count remaining)",
      "  - Display tool name and formatted input preview:",
      "    - Bash: show command",
      "    - Write: show file path + content preview",
      "    - Edit: show file path + old/new strings",
      "    - Read: show file path",
      "    - Default: JSON.stringify",
      "  - Allow/Deny buttons",
      "  - Call invoke('resolve_permission_request') on decision",
      "  - Remove from queue after decision",
      "  - Close dialog closes as deny",
      "Use shadcn Dialog component, design system tokens",
      "Create PermissionDialog.test.tsx with functional tests",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(ui): add PermissionDialog for tool approval"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Mount PermissionDialog globally in App",
    "plan_section": "Permission Bridge System - Frontend Integration",
    "steps": [
      "Update src/App.tsx:",
      "  - Import PermissionDialog from '@/components/PermissionDialog'",
      "  - Mount <PermissionDialog /> at root level (always rendered)",
      "Verify dialog appears when permission:request event fires (manual test)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(app): mount PermissionDialog globally"
    ],
    "passes": true
  },
  {
    "category": "plugin",
    "description": "Configure MCP server in plugin and create chat agents",
    "plan_section": "3. Configure MCP Server in Plugin and Agent Definitions",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation.md section '3. Agent Definitions'",
      "Create/update ralphx-plugin/.mcp.json with ralphx MCP server config",
      "Create ralphx-plugin/agents/chat-task.md - task-focused chat agent (documents its allowed MCP tools)",
      "Create ralphx-plugin/agents/chat-project.md - project-focused chat agent (documents its allowed MCP tools)",
      "Verify orchestrator-ideation.md agent exists and documents its allowed MCP tools",
      "Note: MCP tools are scoped per agent via RALPHX_AGENT_TYPE - see MCP Tool Scoping section",
      "Test agent invocation: claude --agent chat-task --plugin-dir ./ralphx-plugin -p 'test'",
      "Commit: feat(plugin): configure MCP server and create chat agents",
      "STOP: Output <promise>COMPLETE</promise> - Next task (orchestrator refactor) is high-complexity, consider switching to Opus"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Refactor orchestrator service for --resume and MCP delegation",
    "plan_section": "4.2 Context Chat Service and Refactoring Plan",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation.md sections '4.2 Context Chat Service' and 'Refactoring Plan'",
      "Refactor src-tauri/src/application/orchestrator_service.rs:",
      "  - Remove execute_tool_call(), handle_create_task_proposal(), etc. (MCP handles tools now)",
      "  - Add claude_session_id capture from stream-json result event",
      "  - Add --resume flag logic: if conversation has claude_session_id, use --resume instead of --agent",
      "  - Add RALPHX_AGENT_TYPE env var when spawning: cmd.env('RALPHX_AGENT_TYPE', agent_name)",
      "  - Keep stream parsing for UI updates (emit Tauri events for chunks and tool calls)",
      "Create agent_run on message send, update status on completion",
      "Parse tool_calls from stream-json and store them (for UI display)",
      "Emit Tauri events: chat:message_created, chat:chunk, chat:tool_call, chat:run_completed",
      "Write tests for session capture and resume logic",
      "Run cargo test",
      "Commit: refactor(orchestrator): delegate tools to MCP, add --resume support, pass agent type for tool scoping",
      "STOP: Output <promise>COMPLETE</promise> - High-complexity task complete, can switch back to Sonnet"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add Tauri commands for context-aware chat",
    "plan_section": "Files to Create/Modify - Backend",
    "steps": [
      "Create src-tauri/src/commands/context_chat_commands.rs with commands:",
      "  - send_context_message(context_type, context_id, content) -> streams response",
      "  - list_conversations(context_type, context_id) -> Vec<ChatConversation>",
      "  - get_conversation(conversation_id) -> ChatConversation with messages",
      "  - create_conversation(context_type, context_id) -> ChatConversation",
      "  - get_agent_run_status(conversation_id) -> Option<AgentRun>",
      "Register commands in lib.rs invoke_handler",
      "Write tests for commands",
      "Run cargo test",
      "Commit: feat(commands): add context-aware chat commands"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Update frontend types and chat API",
    "plan_section": "Conversation History & Switching - Data Model",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation.md for type definitions",
      "Create src/types/chat-conversation.ts with ChatConversation, AgentRun types and Zod schemas",
      "Update src/types/ideation.ts - add toolCalls field to ChatMessage schema",
      "Update src/api/chat.ts with new API functions:",
      "  - sendContextMessage(contextType, contextId, content)",
      "  - listConversations(contextType, contextId)",
      "  - getConversation(conversationId)",
      "  - createConversation(contextType, contextId)",
      "  - getAgentRunStatus(conversationId)",
      "Run npm run typecheck",
      "Commit: feat(types): add chat conversation types and API"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Update chat store with queue and conversation state",
    "plan_section": "Message Queue System - State Model",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation.md for state model",
      "Update src/stores/chatStore.ts:",
      "  - Add activeConversationId: string | null",
      "  - Add queuedMessages: QueuedMessage[]",
      "  - Add isAgentRunning: boolean",
      "  - Add actions: queueMessage, editQueuedMessage, deleteQueuedMessage, processQueue",
      "  - Add setActiveConversation, setAgentRunning",
      "Write tests for store actions",
      "Run npm run typecheck && npm run test",
      "Commit: feat(store): add conversation and queue state to chat store"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create ToolCallIndicator component",
    "plan_section": "Message Queue System - UI Design (tool calls)",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation.md UI mockup for tool calls",
      "Create src/components/Chat/ToolCallIndicator.tsx:",
      "  - Collapsible view (summary by default, expand for details)",
      "  - Show tool name, icon (wrench), and summary",
      "  - Expand to show full arguments and result",
      "  - Style with design system tokens",
      "Create src/components/Chat/ToolCallIndicator.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(chat): add ToolCallIndicator component"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Update ChatMessage to display tool calls",
    "plan_section": "Refactoring Plan - Frontend",
    "steps": [
      "Update src/components/Chat/ChatMessage.tsx:",
      "  - Import ToolCallIndicator",
      "  - Parse message.toolCalls if present (JSON string)",
      "  - Render ToolCallIndicator for each tool call",
      "  - Position tool calls within message bubble appropriately",
      "Update ChatMessage.test.tsx with tool call test cases",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(chat): display tool calls in ChatMessage"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create ConversationSelector component",
    "plan_section": "Conversation History & Switching - UI Design",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation.md for ConversationSelector UI mockup",
      "Create src/components/Chat/ConversationSelector.tsx:",
      "  - Dropdown trigger (history icon button)",
      "  - List of conversations for current context",
      "  - Show: title (or first message excerpt), date, message count",
      "  - Active conversation indicator (filled dot)",
      "  - 'New Conversation' option at top",
      "  - Click to switch conversations",
      "Use shadcn DropdownMenu component",
      "Create ConversationSelector.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(chat): add ConversationSelector component"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create QueuedMessage components",
    "plan_section": "Message Queue System - UI Design",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation.md for queue UI mockup",
      "Create src/components/Chat/QueuedMessage.tsx:",
      "  - Display queued message content",
      "  - Edit button (pencil icon) -> inline edit mode",
      "  - Delete button (X icon) -> remove from queue",
      "  - Pending/queued visual style (muted, send icon)",
      "Create src/components/Chat/QueuedMessageList.tsx:",
      "  - Header: 'QUEUED MESSAGES (sent when agent finishes)'",
      "  - List of QueuedMessage components",
      "  - Only show if queue not empty",
      "Create tests for both components",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(chat): add QueuedMessage and QueuedMessageList components"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Update ChatInput for queue mode and keyboard navigation",
    "plan_section": "Message Queue System - Queue Behavior",
    "steps": [
      "Update src/components/Chat/ChatInput.tsx:",
      "  - If isAgentRunning: show '(will be queued)' placeholder",
      "  - On send while running: call queueMessage instead of sendMessage",
      "  - Up arrow in empty input: edit last queued message",
      "  - Handle edit mode: Enter saves, Escape cancels",
      "  - Show hint: 'Press ↑ to edit last queued message' when queue exists",
      "Update ChatInput.test.tsx with queue mode tests",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(chat): add queue mode and keyboard navigation to ChatInput"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Update ChatPanel with conversation selector, queue UI, and event handling",
    "plan_section": "Agent Run & Streaming Persistence - Chat Open Flow",
    "steps": [
      "Read specs/plans/context_aware_chat_implementation.md for full ChatPanel requirements",
      "Update src/components/Chat/ChatPanel.tsx:",
      "  - Add history icon button in header (opens ConversationSelector)",
      "  - Load messages from active conversation on mount",
      "  - Check agent_run status on mount -> set isAgentRunning",
      "  - Subscribe to Tauri events: chat:chunk, chat:tool_call, chat:run_completed",
      "  - On chat:run_completed: process queue (send first queued message)",
      "  - Show QueuedMessageList above input when queue not empty",
      "  - Hide panel in ideation view if no session exists",
      "  - Show 'Agent is responding...' indicator when running",
      "Update ChatPanel.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(chat): integrate conversation selector, queue, and events in ChatPanel"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Update useChat hook for context-aware messaging",
    "plan_section": "Files to Create/Modify - Frontend",
    "steps": [
      "Update src/hooks/useChat.ts:",
      "  - Use new chat API (sendContextMessage, etc.)",
      "  - Handle conversation switching",
      "  - Manage agent run status",
      "  - Process queue on run completion",
      "  - Subscribe to Tauri events for real-time updates",
      "Create comprehensive tests for the hook",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(hooks): update useChat for context-aware messaging"
    ],
    "passes": false
  },
  {
    "category": "documentation",
    "description": "Update CLAUDE.md and activity log",
    "steps": [
      "Update CLAUDE.md if needed with new chat architecture notes",
      "Update src/CLAUDE.md with new chat components and hooks",
      "Update src-tauri/CLAUDE.md with new services and commands",
      "Update logs/activity.md with Phase 15 completion summary",
      "Commit: docs: update documentation for context-aware chat"
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
| **MCP Server as TypeScript proxy** | MCP SDK more mature in TypeScript; all business logic stays in Rust |
| **`--resume` instead of history rebuild** | Claude manages context; no prompt bloat; simpler |
| **Agent runs table** | Track running/completed status for leave-and-come-back |
| **Message queue in frontend state** | Simple; queued messages are ephemeral until sent |
| **Tauri events for real-time** | Standard Tauri pattern; frontend subscribes to backend events |
| **MCP tool scoping via RALPHX_AGENT_TYPE** | Hard enforcement: each agent only sees tools appropriate for its role; prevents misuse (e.g., worker creating proposals) |
| **Reviewer uses complete_review MCP tool** | Structured review submission (approved/needs_changes/escalate) via MCP instead of parsing agent output text |
| **Permission Bridge via MCP tool** | Enables UI-based approval for non-pre-approved tools; MCP tool long-polls Tauri backend for user decision |

---

## Verification Checklist

After completing all tasks:

### Backend
- [ ] HTTP server running on port 3847
- [ ] MCP server can call all endpoints
- [ ] `--resume` flag used for follow-up messages
- [ ] Session ID captured and stored
- [ ] Messages persisted in real-time (not just on completion)
- [ ] Agent runs tracked (running/completed/failed)
- [ ] `RALPHX_AGENT_TYPE` env var passed when spawning Claude CLI

### MCP Tool Scoping
- [ ] orchestrator-ideation: sees only ideation tools (create/update/delete proposal, add dependency)
- [ ] chat-task: sees only task tools (update_task, add_task_note, get_task_details)
- [ ] chat-project: sees only project tools (suggest_task, list_tasks)
- [ ] reviewer: sees only review tools (complete_review)
- [ ] worker/supervisor/qa-prep/qa-tester: see NO MCP tools
- [ ] Unauthorized tool calls rejected with clear error message

### Frontend
- [ ] Conversation selector shows history
- [ ] Can switch between conversations
- [ ] Can start new conversation
- [ ] Queue UI appears when agent running
- [ ] Can edit/delete queued messages
- [ ] Up arrow edits last queued message
- [ ] Tool calls displayed (collapsible)
- [ ] Chat hidden in ideation without session

### Integration
- [ ] MCP tools execute successfully
- [ ] Proposals created via MCP appear in UI
- [ ] Leave and come back works
- [ ] Auto-send queued messages on completion

### Permission Bridge
- [ ] permission_request MCP tool registered and callable
- [ ] Permission HTTP endpoints working (/request, /await, /resolve)
- [ ] PermissionDialog appears when permission:request event fires
- [ ] Allow decision continues agent execution
- [ ] Deny decision sends rejection message to agent
- [ ] Timeout (5 min) treated as deny
- [ ] Multiple queued permission requests handled correctly
