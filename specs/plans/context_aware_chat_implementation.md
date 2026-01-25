# Context-Aware Chat Implementation Plan

## Problem
1. The ChatPanel (Cmd+K) saves user messages but never gets AI responses
2. ChatPanel shows in Ideation view even without a session (should be hidden)
3. Custom tools (create_task_proposal, etc.) are not properly exposed to Claude

## Solution
1. Create an **MCP Server** to expose RalphX tools to Claude
2. Use `--agent` flag with plugin agents
3. Create context-aware chat with full tool support

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Frontend (React)                             │
│  ChatPanel → useChat → chatApi.sendContextAwareMessage()            │
└─────────────────────────┬───────────────────────────────────────────┘
                          │ Tauri IPC
                          ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Rust Backend (Tauri)                              │
│  send_context_aware_message command                                  │
│    1. Save user message to SQLite                                   │
│    2. Spawn Claude CLI with --agent flag                            │
│       + Pass RALPHX_AGENT_TYPE env var for MCP tool scoping         │
│    3. Parse streaming response                                       │
│    4. Save assistant message                                         │
└─────────────────────────┬───────────────────────────────────────────┘
                          │ Spawns process with env: RALPHX_AGENT_TYPE
                          ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Claude CLI                                      │
│  claude --agent orchestrator-ideation                               │
│         --plugin-dir ./ralphx-plugin                                │
│         --output-format stream-json                                 │
│         -p "User message..."                                        │
│                                                                      │
│  Environment: RALPHX_AGENT_TYPE=orchestrator-ideation               │
└─────────────────────────┬───────────────────────────────────────────┘
                          │ MCP Protocol (inherits env vars)
                          ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    RalphX MCP Server                                 │
│                                                                      │
│  Reads RALPHX_AGENT_TYPE → filters available tools                  │
│                                                                      │
│  Tool scoping per agent type:                                       │
│  - orchestrator-ideation: ideation tools only                       │
│  - chat-task: task tools only                                       │
│  - chat-project: project tools only                                 │
│  - worker/reviewer/supervisor: NO MCP tools                         │
│                                                                      │
│  Connects to Tauri backend via HTTP (proxy pattern)                 │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Session ID Distinction

**Important:** There are TWO different session IDs in play:

| ID | Source | Purpose | Storage |
|----|--------|---------|---------|
| **RalphX Session ID** | Our backend | Ideation session, task context, project context | `ideation_sessions.id`, `tasks.id`, `projects.id` |
| **Claude Session ID** | Claude CLI | Resume conversations with `--resume` flag | `chat_conversations.claude_session_id` |

**How they relate:**
```
┌──────────────────────────────────────────────────────────────────┐
│ RalphX Ideation Session (id: "ideation-abc123")                  │
│                                                                   │
│   ┌─────────────────────────────────────────────────────────┐    │
│   │ Chat Conversation (claude_session_id: "550e8400-...")   │    │
│   │                                                          │    │
│   │   Message 1: User → "I want dark mode"                  │    │
│   │   Message 2: Assistant → "Great idea!"                  │    │
│   │   Message 3: User → "Add toggle button"                 │    │
│   │   Message 4: Assistant → "I'll update the proposal"     │    │
│   └─────────────────────────────────────────────────────────┘    │
│                                                                   │
│   Task Proposals: [proposal-1, proposal-2, ...]                  │
└──────────────────────────────────────────────────────────────────┘
```

**Workflow:**
1. User creates RalphX ideation session → we generate `ideation-abc123`
2. User sends first chat message → we spawn Claude CLI → Claude returns `session_id: 550e8400-...`
3. We store `claude_session_id` linked to our conversation record
4. User sends follow-up → we use `--resume 550e8400-...` → Claude remembers everything

---

## Implementation Steps

### 1. Create RalphX MCP Server (Proxy to Tauri Backend)

**Location:** `ralphx-mcp-server/`

The MCP server is a **thin proxy** that forwards tool calls to the Tauri backend via HTTP. This keeps all business logic centralized in Rust.

**Architecture:**
```
Claude CLI
    ↓ (spawns MCP server)
RalphX MCP Server (TypeScript)
    ↓ (HTTP calls to localhost)
Tauri Backend (Rust) ← HTTP server plugin
    ↓
SQLite Database
```

**Technology:** Node.js with `@modelcontextprotocol/sdk`

```
ralphx-mcp-server/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts           # MCP server entry point
│   ├── tauri-client.ts    # HTTP client to Tauri backend
│   └── tools.ts           # Tool definitions and handlers
└── build/                 # Compiled JS
```

**MCP Server Setup (proxy pattern with tool scoping):**
```typescript
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";

// HTTP client to call Tauri backend
const TAURI_API_URL = process.env.TAURI_API_URL || "http://localhost:3847";

// Agent type from environment (set by Rust backend when spawning)
const AGENT_TYPE = process.env.RALPHX_AGENT_TYPE || "";

// Tool scoping: which tools each agent type can access
const TOOL_ALLOWLIST: Record<string, string[]> = {
  "orchestrator-ideation": [
    "create_task_proposal",
    "update_task_proposal",
    "delete_task_proposal",
    "add_proposal_dependency",
  ],
  "chat-task": [
    "update_task",
    "add_task_note",
    "get_task_details",
  ],
  "chat-project": [
    "suggest_task",
    "list_tasks",
  ],
  // Review agent - needs to submit review decisions
  "reviewer": [
    "complete_review",
  ],
  // These agents have NO MCP tools - they use filesystem tools only
  "worker": [],
  "supervisor": [],
  "qa-prep": [],
  "qa-tester": [],
};

// Get allowed tools for current agent type
function getAllowedToolNames(): string[] {
  return TOOL_ALLOWLIST[AGENT_TYPE] || [];
}

async function callTauri(command: string, args: Record<string, unknown>) {
  const response = await fetch(`${TAURI_API_URL}/api/${command}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(args),
  });
  if (!response.ok) {
    throw new Error(`Tauri API error: ${response.statusText}`);
  }
  return response.json();
}

// All tool definitions (full list)
const ALL_TOOLS = [
  {
    name: "create_task_proposal",
    description: "Create a new task proposal in the ideation session",
    inputSchema: {
      type: "object",
      properties: {
        session_id: { type: "string" },
        title: { type: "string" },
        description: { type: "string" },
        category: { type: "string", enum: ["feature", "fix", "setup", "testing", "refactor", "docs"] },
        priority: { type: "string", enum: ["critical", "high", "medium", "low"] },
        steps: { type: "array", items: { type: "string" } },
        acceptance_criteria: { type: "array", items: { type: "string" } },
      },
      required: ["session_id", "title", "category"],
    },
  },
  // ... more tools (see full list below)
];

const server = new Server({
  name: "ralphx",
  version: "1.0.0",
}, {
  capabilities: { tools: {} },
});

// Tool definitions - FILTERED by agent type
server.setRequestHandler(ListToolsRequestSchema, async () => {
  const allowedNames = getAllowedToolNames();
  const filteredTools = ALL_TOOLS.filter(tool => allowedNames.includes(tool.name));
  return { tools: filteredTools };
});

// Forward tool calls to Tauri - with authorization check
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  // Double-check authorization (defense in depth)
  const allowedNames = getAllowedToolNames();
  if (!allowedNames.includes(name)) {
    throw new Error(
      `Tool "${name}" is not available for agent type "${AGENT_TYPE}". ` +
      `Allowed tools: ${allowedNames.join(", ") || "none"}`
    );
  }

  const result = await callTauri(name, args);
  return { content: [{ type: "text", text: JSON.stringify(result) }] };
});

const transport = new StdioServerTransport();
await server.connect(transport);
```

### 2. Add HTTP Server to Tauri Backend

**File:** `src-tauri/Cargo.toml` - Add HTTP server dependency

```toml
[dependencies]
axum = "0.7"
tower-http = { version = "0.5", features = ["cors"] }
```

**File:** `src-tauri/src/http_server.rs` (NEW)

Expose existing Tauri commands via HTTP for MCP server to call:

```rust
use axum::{
    routing::post,
    Router, Json,
    extract::State,
};
use std::sync::Arc;
use crate::application::AppState;

pub async fn start_http_server(state: Arc<AppState>) {
    let app = Router::new()
        // Ideation tools
        .route("/api/create_task_proposal", post(create_task_proposal))
        .route("/api/update_task_proposal", post(update_task_proposal))
        .route("/api/delete_task_proposal", post(delete_task_proposal))
        .route("/api/add_proposal_dependency", post(add_proposal_dependency))
        // Task tools
        .route("/api/update_task", post(update_task))
        .route("/api/add_task_note", post(add_task_note))
        .route("/api/get_task_details", post(get_task_details))
        // Project tools
        .route("/api/suggest_task", post(suggest_task))
        .route("/api/list_tasks", post(list_tasks))
        // Review tools
        .route("/api/complete_review", post(complete_review))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3847").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Each handler reuses existing repository/service logic
async fn create_task_proposal(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateProposalInput>,
) -> Json<TaskProposalResponse> {
    // Reuse existing ideation service
    let proposal = state.ideation_service.create_proposal(input).await.unwrap();
    Json(TaskProposalResponse::from(proposal))
}
// ... similar for other handlers
```

**File:** `src-tauri/src/lib.rs` - Start HTTP server on app launch

```rust
// In setup, spawn HTTP server
tokio::spawn(async move {
    http_server::start_http_server(state.clone()).await;
});
```

### 3. Configure MCP Server in Plugin

**File:** `ralphx-plugin/.mcp.json`

```json
{
  "mcpServers": {
    "ralphx": {
      "command": "node",
      "args": ["${CLAUDE_PLUGIN_ROOT}/../ralphx-mcp-server/build/index.js"],
      "env": {
        "TAURI_API_URL": "http://127.0.0.1:3847"
      }
    }
  }
}
```

### 3. Agent Definitions (MCP tools scoped via RALPHX_AGENT_TYPE)

**Important:** While MCP servers configured at the plugin level (`.mcp.json`) are technically available to all agents, our MCP server implements **tool scoping** based on the `RALPHX_AGENT_TYPE` environment variable. Each agent only sees tools appropriate for its role.

**How tool scoping works:**
1. Rust backend passes `RALPHX_AGENT_TYPE=<agent-name>` when spawning Claude CLI
2. Claude CLI inherits env vars and passes them to MCP server
3. MCP server reads `RALPHX_AGENT_TYPE` and filters available tools
4. Agent only sees tools in its allowlist (hard enforcement)

**Supported agent frontmatter fields:**
- `name` (required)
- `description` (required)
- `tools` - Claude Code tools (Read, Grep, Glob, Bash, Edit, Write)
- `disallowedTools`
- `model` - sonnet, opus, haiku, or inherit
- `permissionMode`
- `skills` - skills to preload
- `hooks` - lifecycle hooks

**File:** `ralphx-plugin/agents/orchestrator-ideation.md` (already exists - no changes needed)

**File:** `ralphx-plugin/agents/chat-task.md` (NEW)

```yaml
---
name: chat-task
description: Context-aware assistant for task-related chat. Use when user is chatting about a specific task.
tools: Read, Grep, Glob
model: sonnet
---

You are a task assistant for RalphX. You help users with a specific task.

The task context (ID, title, description, status) will be provided in the prompt.

## Guidelines
- Stay focused on the current task
- Suggest improvements or next steps
- Help clarify requirements
- Use MCP tools when the user wants to modify the task
```

**File:** `ralphx-plugin/agents/chat-project.md` (NEW)

```yaml
---
name: chat-project
description: General project assistant. Use for project-level questions and task suggestions.
tools: Read, Grep, Glob
model: sonnet
---

You are a project assistant for RalphX.

The project context will be provided in the prompt.

## Guidelines
- Help answer questions about the project
- Suggest tasks when the user has ideas
- Use Glob/Grep/Read to explore the codebase
- Use MCP tools when appropriate
```

### 4. Backend: Session Management with --resume Flag

**Key Insight:** Claude CLI manages its own conversation state. We don't need to rebuild conversation history in prompts - instead:

1. **First message:** Spawn Claude, capture `session_id` from response, store it
2. **Follow-up messages:** Use `--resume <session_id>` flag - Claude remembers context

**What we store in our database:**
- User messages (for display in UI)
- Assistant responses (for display in UI)
- Tool calls (parsed from stream-json output)
- **Claude's session_id** (to enable `--resume` for follow-ups)

**What we DON'T do:**
- We don't rebuild conversation history in the prompt
- Claude manages its own context via session persistence

---

#### 4.1 Database Schema Update

**File:** `src-tauri/migrations/XXXX_chat_conversations_and_runs.sql`

```sql
-- ============================================================================
-- Chat Conversations Table
-- Links a context (ideation session, task, project) to Claude sessions
-- ============================================================================

CREATE TABLE IF NOT EXISTS chat_conversations (
    id TEXT PRIMARY KEY,
    context_type TEXT NOT NULL,  -- 'ideation', 'task', 'project'
    context_id TEXT NOT NULL,    -- session_id, task_id, or project_id
    claude_session_id TEXT,      -- Claude CLI session UUID for --resume
    title TEXT,                  -- Auto-generated or user-set title
    message_count INTEGER NOT NULL DEFAULT 0,
    last_message_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Multiple conversations per context allowed
CREATE INDEX idx_chat_conversations_context ON chat_conversations(context_type, context_id);
CREATE INDEX idx_chat_conversations_claude_session ON chat_conversations(claude_session_id);

-- ============================================================================
-- Agent Runs Table
-- Tracks active/completed agent runs for streaming persistence
-- ============================================================================

CREATE TABLE IF NOT EXISTS agent_runs (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES chat_conversations(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'running',  -- 'running', 'completed', 'failed', 'cancelled'
    started_at TEXT NOT NULL,
    completed_at TEXT,
    error_message TEXT
);

CREATE INDEX idx_agent_runs_conversation ON agent_runs(conversation_id);
CREATE INDEX idx_agent_runs_status ON agent_runs(status);

-- ============================================================================
-- Modify chat_messages Table
-- Add conversation reference and tool_calls
-- ============================================================================

-- Add new columns to chat_messages
ALTER TABLE chat_messages ADD COLUMN conversation_id TEXT REFERENCES chat_conversations(id) ON DELETE CASCADE;
ALTER TABLE chat_messages ADD COLUMN tool_calls TEXT;  -- JSON array of tool calls

-- Index for conversation lookup
CREATE INDEX IF NOT EXISTS idx_chat_messages_conversation ON chat_messages(conversation_id);

-- ============================================================================
-- Update message_count trigger
-- ============================================================================

CREATE TRIGGER IF NOT EXISTS update_conversation_message_count
AFTER INSERT ON chat_messages
FOR EACH ROW
WHEN NEW.conversation_id IS NOT NULL
BEGIN
    UPDATE chat_conversations
    SET message_count = message_count + 1,
        last_message_at = NEW.created_at,
        updated_at = datetime('now')
    WHERE id = NEW.conversation_id;
END;
```

**Key relationships:**
```
┌─────────────────┐       ┌─────────────────────┐       ┌──────────────────┐
│ Context         │ 1   N │ chat_conversations  │ 1   N │ chat_messages    │
│ (ideation/task/ ├───────┤                     ├───────┤                  │
│  project)       │       │ claude_session_id   │       │ tool_calls (JSON)│
└─────────────────┘       │ title               │       └──────────────────┘
                          └─────────┬───────────┘
                                    │ 1
                                    │
                                    │ N
                          ┌─────────┴───────────┐
                          │ agent_runs          │
                          │ status: running/    │
                          │         completed   │
                          └─────────────────────┘
```

---

#### 4.2 Context Chat Service

**File:** `src-tauri/src/application/context_chat_service.rs`

```rust
use tokio::process::Command;
use serde::Deserialize;

/// Response from Claude CLI with --output-format json
#[derive(Deserialize)]
struct ClaudeJsonResponse {
    result: String,
    session_id: String,  // Claude's session UUID
    // usage, etc. - we can store these too
}

/// Stream event from --output-format stream-json
#[derive(Deserialize)]
#[serde(tag = "type")]
enum StreamEvent {
    #[serde(rename = "assistant")]
    Assistant { message: AssistantMessage },
    #[serde(rename = "result")]
    Result { result: String, session_id: String },
    // ... tool_use, tool_result, etc.
}

impl ContextAwareChatService {
    pub async fn send_message(
        &self,
        context: ChatContextType,
        content: &str,
    ) -> Result<ChatResponse, ChatError> {
        // 1. Store user message immediately (for UI display)
        let user_msg = self.create_user_message(&context, content);
        self.chat_message_repo.create(user_msg.clone()).await?;

        // 2. Check if we have an existing Claude session for this context
        let existing_claude_session = self.get_claude_session_id(&context).await?;

        // 3. Determine which agent to use
        let agent_name = match &context {
            ChatContextType::Ideation { .. } => "orchestrator-ideation",
            ChatContextType::Task { .. } => "chat-task",
            ChatContextType::Project { .. } => "chat-project",
        };

        // 4. Build the command
        let mut cmd = Command::new(&self.cli_path);

        // Common args
        cmd.args([
            "--plugin-dir", "./ralphx-plugin",
            "--output-format", "stream-json",
        ]);

        // Pass agent type for MCP tool scoping
        // The MCP server reads this to filter which tools are available
        cmd.env("RALPHX_AGENT_TYPE", agent_name);

        if let Some(claude_session_id) = &existing_claude_session {
            // FOLLOW-UP: Resume existing Claude session
            // Claude remembers the full conversation - just send new message
            cmd.args([
                "--resume", claude_session_id,
                "-p", content,  // Just the new message, no context needed
            ]);
        } else {
            // FIRST MESSAGE: Start new session with agent and initial context
            let initial_prompt = self.build_initial_prompt(&context, content).await?;
            cmd.args([
                "--agent", agent_name,
                "-p", &initial_prompt,
            ]);
        }

        cmd.current_dir(&self.working_directory)
           .stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::piped());

        // 5. Process streaming response
        let result = self.process_stream(&mut cmd).await?;

        // 6. If this was a new session, store Claude's session_id
        if existing_claude_session.is_none() {
            self.store_claude_session_id(&context, &result.claude_session_id).await?;
        }

        // 7. Store assistant message (for UI display)
        let assistant_msg = self.create_assistant_message(&context, &result.response_text);
        self.chat_message_repo.create(assistant_msg.clone()).await?;

        // 8. Store tool calls (for UI display and audit)
        for tool_call in &result.tool_calls {
            self.tool_call_repo.create(tool_call.clone()).await?;
        }

        Ok(ChatResponse {
            user_message: user_msg,
            assistant_message: assistant_msg,
            tool_calls: result.tool_calls,
        })
    }

    /// Build initial prompt with context (only for first message)
    /// Follow-up messages just use --resume with the raw user message
    async fn build_initial_prompt(
        &self,
        context: &ChatContextType,
        user_message: &str,
    ) -> Result<String, ChatError> {
        match context {
            ChatContextType::Ideation { session_id } => {
                // Provide session context for MCP tools
                Ok(format!(
                    "RalphX Ideation Session ID: {}\n\n\
                     User's message: {}",
                    session_id.as_str(), user_message
                ))
            }
            ChatContextType::Task { task_id } => {
                // Provide task details
                let task = self.task_repo.get_by_id(task_id).await?
                    .ok_or(ChatError::TaskNotFound)?;
                Ok(format!(
                    "Task Context:\n\
                     - ID: {}\n\
                     - Title: {}\n\
                     - Description: {}\n\
                     - Status: {}\n\n\
                     User's message: {}",
                    task.id.as_str(),
                    task.title,
                    task.description.unwrap_or_default(),
                    task.internal_status,
                    user_message
                ))
            }
            ChatContextType::Project { project_id } => {
                Ok(format!(
                    "Project ID: {}\n\n\
                     User's message: {}",
                    project_id.as_str(), user_message
                ))
            }
        }
    }

    /// Process streaming JSON output from Claude CLI
    async fn process_stream(&self, cmd: &mut Command) -> Result<StreamResult, ChatError> {
        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().ok_or(ChatError::NoStdout)?;
        let reader = tokio::io::BufReader::new(stdout);
        let mut lines = reader.lines();

        let mut response_text = String::new();
        let mut tool_calls = Vec::new();
        let mut claude_session_id = String::new();

        while let Some(line) = lines.next_line().await? {
            if let Ok(event) = serde_json::from_str::<StreamEvent>(&line) {
                match event {
                    StreamEvent::Assistant { message } => {
                        // Accumulate text content
                        for block in message.content {
                            if let ContentBlock::Text { text } = block {
                                response_text.push_str(&text);
                            }
                        }
                    }
                    StreamEvent::Result { result, session_id } => {
                        // Final result includes session_id
                        claude_session_id = session_id;
                        if response_text.is_empty() {
                            response_text = result;
                        }
                    }
                    // Handle tool_use, tool_result events for tool_calls vec
                    // ...
                }
            }
        }

        let status = child.wait().await?;
        if !status.success() {
            return Err(ChatError::ClaudeCliError(status.code()));
        }

        Ok(StreamResult {
            response_text,
            tool_calls,
            claude_session_id,
        })
    }

    /// Get stored Claude session ID for this context (if exists)
    async fn get_claude_session_id(&self, context: &ChatContextType) -> Result<Option<String>, ChatError> {
        let conversation = self.get_or_create_conversation(context).await?;
        Ok(conversation.claude_session_id)
    }

    /// Store Claude's session ID for future --resume calls
    async fn store_claude_session_id(
        &self,
        context: &ChatContextType,
        claude_session_id: &str,
    ) -> Result<(), ChatError> {
        let conversation = self.get_or_create_conversation(context).await?;
        self.conversation_repo.update_claude_session_id(
            conversation.id,
            claude_session_id,
        ).await?;
        Ok(())
    }
}
```

### 5. MCP Server Tool Implementations (Proxy Pattern)

The MCP server is a thin proxy - all business logic stays in Rust:

**File:** `ralphx-mcp-server/src/tools.ts`

```typescript
// All tools just forward to Tauri backend via HTTP
// No database logic here - reuses Rust services

const TAURI_API_URL = process.env.TAURI_API_URL || "http://127.0.0.1:3847";

async function callTauri(endpoint: string, args: unknown): Promise<unknown> {
  const response = await fetch(`${TAURI_API_URL}/api/${endpoint}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(args),
  });
  if (!response.ok) {
    const error = await response.text();
    throw new Error(`Tauri API error: ${error}`);
  }
  return response.json();
}

// Tool definitions with schemas
export const tools = [
  {
    name: "create_task_proposal",
    description: "Create a new task proposal in the ideation session",
    inputSchema: {
      type: "object",
      properties: {
        session_id: { type: "string", description: "Ideation session ID" },
        title: { type: "string", description: "Task title" },
        description: { type: "string", description: "Task description" },
        category: { type: "string", enum: ["feature", "fix", "setup", "testing", "refactor", "docs"] },
        priority: { type: "string", enum: ["critical", "high", "medium", "low"] },
        steps: { type: "array", items: { type: "string" }, description: "Implementation steps" },
        acceptance_criteria: { type: "array", items: { type: "string" } },
      },
      required: ["session_id", "title", "category"],
    },
  },
  {
    name: "update_task",
    description: "Update task properties (title, description, priority)",
    inputSchema: {
      type: "object",
      properties: {
        task_id: { type: "string", description: "Task ID" },
        title: { type: "string" },
        description: { type: "string" },
        priority: { type: "number" },
      },
      required: ["task_id"],
    },
  },
  {
    name: "add_task_note",
    description: "Add a note to a task",
    inputSchema: {
      type: "object",
      properties: {
        task_id: { type: "string" },
        note: { type: "string", description: "Note content" },
      },
      required: ["task_id", "note"],
    },
  },
  // ... other tools follow same pattern
];

// Handler forwards to Tauri
export async function handleToolCall(name: string, args: unknown) {
  const result = await callTauri(name, args);
  return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
}
```

### 6. Frontend Changes (Same as Before)

**Files to modify:**
- `src/api/chat.ts` - Add `sendContextAwareMessage`
- `src/hooks/useChat.ts` - Use new API
- `src/components/Chat/ChatPanel.tsx` - Hide in ideation without session

---

## Files to Create/Modify

### New Files

**Backend (Rust):**
| File | Purpose |
|------|---------|
| `src-tauri/src/http_server.rs` | HTTP server exposing commands to MCP proxy |
| `src-tauri/migrations/XXXX_chat_conversations_and_runs.sql` | Add conversations, agent_runs, tool_calls |
| `src-tauri/src/domain/entities/chat_conversation.rs` | ChatConversation entity |
| `src-tauri/src/domain/entities/agent_run.rs` | AgentRun entity |
| `src-tauri/src/domain/repositories/chat_conversation_repo.rs` | Conversation repository trait |
| `src-tauri/src/domain/repositories/agent_run_repo.rs` | Agent run repository trait |
| `src-tauri/src/infrastructure/sqlite/sqlite_chat_conversation_repo.rs` | SQLite implementation |
| `src-tauri/src/infrastructure/sqlite/sqlite_agent_run_repo.rs` | SQLite implementation |
| `src-tauri/src/commands/context_chat_commands.rs` | New Tauri commands for chat |

**MCP Server (TypeScript proxy):**
| File | Purpose |
|------|---------|
| `ralphx-mcp-server/package.json` | MCP server dependencies |
| `ralphx-mcp-server/tsconfig.json` | TypeScript config |
| `ralphx-mcp-server/src/index.ts` | MCP server entry point with tool scoping logic |
| `ralphx-mcp-server/src/tauri-client.ts` | HTTP client to Tauri backend |
| `ralphx-mcp-server/src/tools.ts` | Tool definitions, TOOL_ALLOWLIST for agent-based filtering |
| `ralphx-mcp-server/src/permission-handler.ts` | Permission bridge MCP tool (long-polls Tauri for user decisions) |

**Permission Bridge (Backend):**
| File | Purpose |
|------|---------|
| `src-tauri/src/application/permission_state.rs` | Shared state for pending permission requests |
| `src-tauri/src/commands/permission_commands.rs` | Tauri commands: resolve_permission_request, get_pending_permissions |

**Permission Bridge (Frontend):**
| File | Purpose |
|------|---------|
| `src/components/PermissionDialog.tsx` | Modal dialog for permission approval/denial |
| `src/types/permission.ts` | TypeScript types for permission events |

**Agent Definitions:**
| File | Purpose |
|------|---------|
| `ralphx-plugin/agents/chat-task.md` | Task-focused chat agent |
| `ralphx-plugin/agents/chat-project.md` | Project-focused chat agent |

**Frontend (React):**
| File | Purpose |
|------|---------|
| `src/components/Chat/ConversationSelector.tsx` | History dropdown for switching conversations |
| `src/components/Chat/QueuedMessage.tsx` | Individual queued message with edit/delete |
| `src/components/Chat/QueuedMessageList.tsx` | List of queued messages below input |
| `src/components/Chat/ToolCallIndicator.tsx` | Collapsible tool call display |
| `src/types/chat-conversation.ts` | ChatConversation, AgentRun types |

### Modified Files

**Backend (Rust):**
| File | Change |
|------|--------|
| `src-tauri/Cargo.toml` | Add axum, tower-http dependencies |
| `src-tauri/src/lib.rs` | Spawn HTTP server, register new commands, emit events, init PermissionState |
| `src-tauri/src/application/mod.rs` | Export new services including permission_state |
| `src-tauri/src/application/orchestrator_service.rs` | **Major refactor**: remove tool execution, add session capture, add `--resume`, pass `RALPHX_AGENT_TYPE` env var |
| `src-tauri/src/infrastructure/agents/claude/claude_code_client.rs` | Pass `RALPHX_AGENT_TYPE` env var, add `--permission-prompt-tool` flag |
| `src-tauri/src/domain/entities/mod.rs` | Export new entities |
| `src-tauri/src/domain/entities/ideation.rs` | Add `tool_calls` field to ChatMessage |
| `src-tauri/src/domain/repositories/mod.rs` | Export new repository traits |
| `src-tauri/src/commands/mod.rs` | Export new commands including permission_commands |
| `src-tauri/src/http_server.rs` | Add permission endpoints: /request, /await/:id, /resolve |

**Plugin:**
| File | Change |
|------|--------|
| `ralphx-plugin/.mcp.json` | Add ralphx MCP server config |

**Frontend (React):**
| File | Change |
|------|--------|
| `src/api/chat.ts` | Add conversation CRUD, context-aware send |
| `src/hooks/useChat.ts` | Use new API, handle events, manage queue |
| `src/stores/chatStore.ts` | Add queuedMessages, activeConversationId, isAgentRunning |
| `src/types/ideation.ts` | Add toolCalls to ChatMessage schema |
| `src/components/Chat/ChatPanel.tsx` | Add history button, queue UI, tool calls, hide logic |
| `src/components/Chat/ChatMessage.tsx` | Add tool call display |
| `src/components/Chat/ChatInput.tsx` | Handle queue mode, up-arrow for edit |
| `src/App.tsx` | Mount PermissionDialog globally |

**Note:** No changes needed to existing agent `.md` files for MCP - they automatically inherit access once `.mcp.json` is configured.

---

## MCP Server Tools Summary

| Tool | Context | Description |
|------|---------|-------------|
| `create_task_proposal` | Ideation | Create new proposal |
| `update_task_proposal` | Ideation | Modify proposal |
| `delete_task_proposal` | Ideation | Remove proposal |
| `add_proposal_dependency` | Ideation | Link proposals |
| `update_task` | Task | Modify task properties |
| `add_task_note` | Task | Add note to task |
| `get_task_details` | Task | Get full task info |
| `suggest_task` | Project | Create task suggestion |
| `list_tasks` | Project | List project tasks |
| `complete_review` | Review | Submit review decision (approved/needs_changes/escalate) |
| `permission_request` | **All** | Handle permission prompts (not scoped - available to all agents) |

**Note:** No `search_codebase` tool needed - agents have direct access to Bash, Grep, Glob, Read tools.

**Note:** The `permission_request` tool is special - it's called by Claude CLI via `--permission-prompt-tool` flag, not by agents directly. It's always available regardless of `RALPHX_AGENT_TYPE`.

---

## MCP Tool Scoping

### Why Scoping is Needed

Different agents have different responsibilities:
- **orchestrator-ideation**: Facilitates brainstorming, creates task proposals
- **chat-task**: Helps with a specific task, can add notes and update properties
- **chat-project**: General project questions, can suggest new tasks
- **worker**: Executes tasks - should NOT create proposals or modify tasks directly

Without scoping, an orchestrator-ideation agent could see `update_task` and try to use it even though that's not its job. Worse, a worker agent could see `create_task_proposal` and create proposals when it should only be executing code.

### Implementation Approach

**Hard enforcement via RALPHX_AGENT_TYPE environment variable:**

1. **Rust backend** passes agent type when spawning Claude CLI:
   ```rust
   cmd.env("RALPHX_AGENT_TYPE", agent_name);
   ```

2. **Claude CLI** inherits environment variables and passes them to MCP server

3. **MCP server** reads `RALPHX_AGENT_TYPE` and filters tools:
   ```typescript
   const AGENT_TYPE = process.env.RALPHX_AGENT_TYPE || "";
   const allowedNames = TOOL_ALLOWLIST[AGENT_TYPE] || [];
   ```

4. **Double enforcement**:
   - `ListToolsRequestSchema`: Only returns tools in allowlist (agent doesn't even see others)
   - `CallToolRequestSchema`: Rejects calls to unauthorized tools (defense in depth)

### Tool Allowlist

| Agent Type | Allowed MCP Tools |
|------------|-------------------|
| `orchestrator-ideation` | `create_task_proposal`, `update_task_proposal`, `delete_task_proposal`, `add_proposal_dependency` |
| `chat-task` | `update_task`, `add_task_note`, `get_task_details` |
| `chat-project` | `suggest_task`, `list_tasks` |
| `worker` | *(none)* |
| `reviewer` | *(none)* |
| `supervisor` | *(none)* |
| `qa-prep` | *(none)* |
| `qa-tester` | *(none)* |

Agents not in this list (e.g., custom agents) get no MCP tools by default.

### Benefits

- **Security**: Agents can't call tools outside their role
- **Clarity**: Agent system prompts match actual available tools
- **Maintainability**: Single source of truth for tool permissions (MCP server)
- **Debugging**: Clear error messages when unauthorized tool is attempted

---

---

## Session Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        FIRST MESSAGE IN CONVERSATION                     │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  User: "I want to add dark mode"                                        │
│                            │                                             │
│                            ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ Backend: No claude_session_id stored for this conversation      │    │
│  │                                                                  │    │
│  │ Spawn: claude --agent orchestrator-ideation \                   │    │
│  │              --plugin-dir ./ralphx-plugin \                     │    │
│  │              --output-format stream-json \                      │    │
│  │              -p "Session ID: abc123\n\nUser: I want dark mode"  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                            │                                             │
│                            ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ Claude CLI Response (stream-json):                               │    │
│  │ {"type":"result","result":"Great idea!...","session_id":"uuid"}  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                            │                                             │
│                            ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ Backend stores:                                                  │    │
│  │ - User message: "I want to add dark mode"                       │    │
│  │ - Assistant response: "Great idea!..."                          │    │
│  │ - claude_session_id: "uuid" (for future --resume)               │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                        FOLLOW-UP MESSAGE                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  User: "Can you also add a toggle button?"                              │
│                            │                                             │
│                            ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ Backend: Found claude_session_id = "uuid" for this conversation │    │
│  │                                                                  │    │
│  │ Spawn: claude --resume "uuid" \                                 │    │
│  │              --plugin-dir ./ralphx-plugin \                     │    │
│  │              --output-format stream-json \                      │    │
│  │              -p "Can you also add a toggle button?"             │    │
│  │                                                                  │    │
│  │ NOTE: No --agent flag needed! Claude resumes with full context  │    │
│  │       including the agent system prompt from first message.     │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                            │                                             │
│                            ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ Claude already knows:                                           │    │
│  │ - It's orchestrator-ideation agent                              │    │
│  │ - Session ID is abc123                                          │    │
│  │ - User previously asked about dark mode                         │    │
│  │ - It responded with "Great idea!"                               │    │
│  │ - All MCP tools are available                                   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                            │                                             │
│                            ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ Backend stores:                                                  │    │
│  │ - User message: "Can you also add a toggle button?"             │    │
│  │ - Assistant response: "I'll add that to the proposal..."       │    │
│  │ - Tool calls: create_task_proposal (if any)                     │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

**Key Benefits:**
- No prompt bloat from rebuilding conversation history
- Claude manages its own context (tool calls, reasoning, etc.)
- We still store everything for our UI display
- Simple `--resume` flag handles all the complexity

---

## Verification

1. **MCP Server works standalone:**
   ```bash
   cd ralphx-mcp-server && npm run build && npm start
   # Test with MCP inspector
   ```

2. **Claude session capture and resume:**
   ```bash
   # First message - capture session_id
   response=$(claude --agent orchestrator-ideation \
          --plugin-dir ./ralphx-plugin \
          --output-format json \
          -p "Session ID: abc123

    User's message: I want to add dark mode")

      # Extract session_id
      session_id=$(echo "$response" | jq -r '.session_id')
      echo "Captured session: $session_id"

      # Follow-up - use --resume (Claude remembers everything)
      claude --resume "$session_id" \
              --plugin-dir ./ralphx-plugin \
              --output-format json \
              -p "Can you also add a toggle button?"
      # Claude knows the context, agent, and previous conversation
   ```

3. **Context-aware chat:**
   - Ideation: Create session → chat → proposals created via MCP
   - Task: Select task → chat → task modified via MCP
   - Project: General chat → task suggestions via MCP

4. **UI behavior:**
   - Ideation without session → chat hidden
   - Messages persist across refreshes (stored in our DB)
   - Claude session persists for resume (via stored claude_session_id)
   - Context switching shows different history (different conversations)

---

---

## Agent Run & Streaming Persistence

When agent is running, we need to persist messages in real-time so user can leave and come back.

### Agent Run Tracking

```sql
CREATE TABLE agent_runs (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES chat_conversations(id),
    status TEXT NOT NULL DEFAULT 'running',  -- 'running', 'completed', 'failed', 'cancelled'
    started_at TEXT NOT NULL,
    completed_at TEXT,
    error_message TEXT
);
```

### Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     MESSAGE SEND FLOW                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  User clicks Send                                                        │
│        │                                                                 │
│        ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ 1. Persist user message to DB immediately                        │    │
│  │ 2. Create agent_run record: status = "running"                   │    │
│  │ 3. Emit event: "chat:message_created"                           │    │
│  │ 4. Spawn Claude CLI process                                      │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│        │                                                                 │
│        ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ Stream Processing Loop:                                          │    │
│  │                                                                   │    │
│  │   For each chunk:                                                │    │
│  │   - Accumulate text into assistant message                       │    │
│  │   - Update DB (upsert assistant message)                         │    │
│  │   - Emit event: "chat:chunk" { text, message_id }               │    │
│  │                                                                   │    │
│  │   For each tool_call:                                            │    │
│  │   - Persist tool call to message.tool_calls                      │    │
│  │   - Emit event: "chat:tool_call" { tool_name, args, result }    │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│        │                                                                 │
│        ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ On completion:                                                    │    │
│  │ - Capture claude_session_id from response                        │    │
│  │ - Update conversation.claude_session_id                          │    │
│  │ - Update agent_run: status = "completed"                         │    │
│  │ - Emit event: "chat:run_completed" { conversation_id }          │    │
│  │ - Check message queue → send next if any                         │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                     CHAT OPEN FLOW                                       │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  User opens ChatPanel                                                    │
│        │                                                                 │
│        ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ 1. Get active conversation for context (or create new)           │    │
│  │ 2. Load all messages from DB                                     │    │
│  │ 3. Load queued messages from local state                         │    │
│  │ 4. Check agent_run status                                        │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│        │                                                                 │
│        ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ If agent_run.status == "running":                                │    │
│  │ - Show "Agent is responding..." indicator                        │    │
│  │ - Subscribe to Tauri events for this conversation                │    │
│  │ - New messages added via events until "chat:run_completed"       │    │
│  │ - Input shows "Queue message" mode (can type, but queues)        │    │
│  │                                                                   │    │
│  │ If agent_run.status != "running":                                │    │
│  │ - Normal input mode                                              │    │
│  │ - If queued messages exist, show them below input                │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Conversation History & Switching

### UI Design

```
┌──────────────────────────────────────────────────────────────────┐
│  ChatPanel Header                                                 │
├──────────────────────────────────────────────────────────────────┤
│  [Icon] Task ▼    [History 📜]  [─] [×]                         │
│                        │                                          │
│                        ▼ (click opens dropdown)                   │
│                   ┌────────────────────────────────┐             │
│                   │ + New Conversation             │             │
│                   ├────────────────────────────────┤             │
│                   │ ● Today 2:30 PM                │ ← active    │
│                   │   "Dark mode implementation"   │             │
│                   │   12 messages                  │             │
│                   ├────────────────────────────────┤             │
│                   │ ○ Yesterday 4:15 PM            │             │
│                   │   "API refactoring discussion" │             │
│                   │   8 messages                   │             │
│                   ├────────────────────────────────┤             │
│                   │ ○ Jan 20, 11:00 AM             │             │
│                   │   "Initial setup help"         │             │
│                   │   3 messages                   │             │
│                   └────────────────────────────────┘             │
└──────────────────────────────────────────────────────────────────┘
```

### Conversation Title Generation

- Auto-generate from first user message (first 50 chars)
- Or from Claude's summary of the conversation
- User can rename via right-click → "Rename conversation"

### Data Model

```typescript
interface ChatConversation {
  id: string;
  contextType: 'ideation' | 'task' | 'project';
  contextId: string;  // session_id, task_id, or project_id
  claudeSessionId: string | null;  // For --resume
  title: string | null;  // Auto-generated or user-set
  messageCount: number;
  lastMessageAt: string;
  createdAt: string;
}
```

---

## Message Queue System

### UI Design

```
┌──────────────────────────────────────────────────────────────────┐
│  Messages Area                                                    │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  [User] Can you add dark mode?                         2:30 PM   │
│                                                                   │
│  [Bot] I'll create a proposal for that...             2:30 PM   │
│        ┌─────────────────────────────────────┐                   │
│        │ 🔧 create_task_proposal              │ ← collapsible    │
│        │    title: "Dark mode support"       │                   │
│        └─────────────────────────────────────┘                   │
│        I've created a proposal for dark mode.                    │
│                                                                   │
│  [User] Also add a toggle button please              2:31 PM    │
│                                                                   │
│  [Bot] ••• typing...                                             │
│                                                                   │
├──────────────────────────────────────────────────────────────────┤
│  QUEUED MESSAGES (sent when agent finishes)                      │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ 📤 Make sure it persists the user preference    [✏️] [×]  │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ 📤 And add a keyboard shortcut Cmd+Shift+D     [✏️] [×]  │  │
│  └────────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────────┤
│  [Type a message... (will be queued)]              [↑ Send]     │
│                                                                   │
│  ↑ Press ↑ to edit last queued message                          │
└──────────────────────────────────────────────────────────────────┘
```

### Queue Behavior

1. **While agent is running:**
   - User types message and presses Enter/Send
   - Message added to queue (not sent yet)
   - Visual indicator: pending/queued state
   - Input clears, ready for next message

2. **When agent completes:**
   - Auto-send first queued message
   - Start new agent run
   - Continue until queue empty

3. **Keyboard shortcuts:**
   - `↑` (up arrow) in empty input → edit last queued message
   - `Enter` → save edit
   - `Escape` → cancel edit

4. **Inline actions:**
   - Click message → edit inline
   - `×` button → delete from queue

### State Model

```typescript
interface QueuedMessage {
  id: string;  // Local ID for tracking
  content: string;
  createdAt: string;
  isEditing: boolean;
}

interface ChatStore {
  // ... existing state
  queuedMessages: QueuedMessage[];
  isAgentRunning: boolean;

  // Actions
  queueMessage: (content: string) => void;
  editQueuedMessage: (id: string, content: string) => void;
  deleteQueuedMessage: (id: string) => void;
  processQueue: () => Promise<void>;  // Called when agent completes
}
```

---

## Existing Code Analysis

### What Exists (Old Approach)

**`orchestrator_service.rs`** - Current implementation:
- Rebuilds conversation history from stored messages (`build_conversation_history()`)
- Parses streaming JSON from Claude CLI
- **Intercepts tool calls and executes them locally** (`execute_tool_call()`)
- Has hardcoded tool handlers: `handle_create_task_proposal()`, `handle_update_task_proposal()`, `handle_delete_task_proposal()`
- Stores user/orchestrator messages via `ChatMessageRepository`

**`ideation_service.rs`** - Clean service layer:
- Manages sessions, proposals, messages via repositories
- Methods: `create_proposal()`, `add_user_message()`, `add_orchestrator_message()`
- Does NOT invoke Claude CLI (that's `orchestrator_service`)

**Frontend Chat Components:**
- `ChatPanel.tsx` - Message list with input, uses `useChat` hook
- `ChatMessage.tsx` - Displays role (user/orchestrator/system) + content
- **Does NOT display tool calls** - only text messages

### Problems with Current Architecture

| Problem | Impact |
|---------|--------|
| Tool calls are intercepted | User never sees what tools Claude called |
| No `--resume` support | History rebuilt in prompt every time (tokens wasted) |
| No `claude_session_id` stored | Can't resume Claude sessions |
| Tool handling duplicated | Rust parses tool calls instead of MCP doing it |
| No tool call storage | Can't display tool history in UI |

---

## Refactoring Plan

### What Changes

| Current | New |
|---------|-----|
| `orchestrator_service.rs` intercepts tool calls | MCP server handles tool execution |
| History rebuilt in prompt | `--resume` with `claude_session_id` |
| `ChatMessage` stores only text | Store text + tool calls |
| Frontend shows text only | Frontend shows text + tool call indicators |

### Files to Refactor

**Backend (Rust):**

1. **`orchestrator_service.rs`** → Refactor significantly
   - Remove `execute_tool_call()`, `handle_create_task_proposal()`, etc.
   - Add `claude_session_id` capture from response
   - Add `--resume` flag for follow-up messages
   - Keep stream parsing for UI updates (text chunks, tool indicators)
   - **Let MCP handle actual tool execution**

2. **`ChatMessage` entity** → Add tool_calls field
   ```rust
   pub struct ChatMessage {
       pub id: ChatMessageId,
       pub session_id: Option<IdeationSessionId>,
       pub role: MessageRole,
       pub content: String,
       pub tool_calls: Option<String>,  // NEW: JSON array of tool calls
       pub created_at: DateTime<Utc>,
   }
   ```

3. **New: `chat_conversations` table** → Store Claude session mapping
   ```sql
   CREATE TABLE chat_conversations (
       id TEXT PRIMARY KEY,
       context_type TEXT NOT NULL,  -- 'ideation', 'task', 'project'
       context_id TEXT NOT NULL,    -- session_id, task_id, or project_id
       claude_session_id TEXT,      -- Claude CLI session for --resume
       created_at TEXT NOT NULL,
       updated_at TEXT NOT NULL,
       UNIQUE(context_type, context_id)
   );
   ```

**Frontend (React):**

1. **`ChatMessage.tsx`** → Add tool call display
   ```tsx
   // Show tool calls if present
   {message.toolCalls && (
     <ToolCallIndicator calls={JSON.parse(message.toolCalls)} />
   )}
   ```

2. **`types/ideation.ts`** → Update ChatMessage type
   ```typescript
   export const ChatMessageSchema = z.object({
     id: z.string(),
     sessionId: z.string().nullable(),
     role: MessageRoleSchema,
     content: z.string(),
     toolCalls: z.string().nullable(),  // NEW
     createdAt: z.string(),
   });
   ```

---

## Permission Bridge System

When agents attempt to use tools that aren't pre-approved via `--allowedTools`, we need a mechanism to:
1. Pause Claude CLI execution
2. Present the permission request to the user in the UI
3. Capture the user's approve/reject decision
4. Resume Claude CLI with that decision

### Why This Is Needed

Claude CLI in `-p` mode is non-interactive. The built-in permission mechanisms are:
- `--allowedTools`: Pre-approve tools at spawn time (compile-time only)
- `--permission-prompt-tool`: Specify an MCP tool to handle permission prompts synchronously
- Hooks (`PermissionRequest`): Shell commands that run synchronously

None of these support **asynchronous UI-based approval**. We solve this by using `--permission-prompt-tool` with an MCP tool that long-polls our Tauri backend.

---

### Permission Bridge Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     PERMISSION BRIDGE FLOW                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. Claude CLI encounters tool needing permission                           │
│     (tool not in --allowedTools)                                            │
│           │                                                                  │
│           ▼                                                                  │
│  2. Claude CLI calls MCP tool: mcp__ralphx__permission_request              │
│     Args: { tool_name, tool_input, context }                                │
│           │                                                                  │
│           ▼                                                                  │
│  3. MCP Server receives permission_request call                             │
│     → POST to Tauri: /api/permission/request                                │
│     → Tauri stores pending request in memory                                │
│     → Tauri emits event: "permission:request"                               │
│     → MCP tool BLOCKS (long-poll /api/permission/await/:id)                 │
│           │                                                                  │
│           ▼                                                                  │
│  4. Frontend receives Tauri event                                           │
│     → Shows PermissionDialog with tool details                              │
│     → User clicks Allow / Deny                                              │
│           │                                                                  │
│           ▼                                                                  │
│  5. Frontend calls: invoke("resolve_permission_request", { id, decision })  │
│     → Tauri signals waiting long-poll request                               │
│     → MCP tool receives response, returns to Claude CLI                     │
│           │                                                                  │
│           ▼                                                                  │
│  6. Claude CLI continues or aborts based on decision                        │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

### Permission Handler MCP Tool

**File:** `ralphx-mcp-server/src/permission-handler.ts`

```typescript
import { TAURI_API_URL } from "./tauri-client.js";

// Tool definition for permission handling
export const permissionRequestTool = {
  name: "permission_request",
  description: "Internal tool for handling permission prompts from Claude CLI",
  inputSchema: {
    type: "object",
    properties: {
      tool_name: {
        type: "string",
        description: "Name of the tool requesting permission"
      },
      tool_input: {
        type: "object",
        description: "Input arguments for the tool"
      },
      context: {
        type: "string",
        description: "Additional context about why the tool is being called"
      },
    },
    required: ["tool_name", "tool_input"],
  },
};

interface PermissionDecision {
  decision: "allow" | "deny";
  message?: string;
}

/**
 * Handle a permission request by forwarding to Tauri backend
 * and waiting for user decision via long-poll.
 *
 * Timeout: 5 minutes (user may be away from keyboard)
 */
export async function handlePermissionRequest(args: {
  tool_name: string;
  tool_input: Record<string, unknown>;
  context?: string;
}): Promise<{ content: Array<{ type: "text"; text: string }> }> {
  // 1. Register permission request with Tauri backend
  const registerResponse = await fetch(`${TAURI_API_URL}/api/permission/request`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      tool_name: args.tool_name,
      tool_input: args.tool_input,
      context: args.context,
    }),
  });

  if (!registerResponse.ok) {
    throw new Error(`Failed to register permission request: ${registerResponse.statusText}`);
  }

  const { request_id } = await registerResponse.json() as { request_id: string };

  // 2. Long-poll for user decision (5 minute timeout)
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), 5 * 60 * 1000);

  try {
    const decisionResponse = await fetch(
      `${TAURI_API_URL}/api/permission/await/${request_id}`,
      {
        method: "GET",
        signal: controller.signal,
      }
    );

    clearTimeout(timeoutId);

    if (!decisionResponse.ok) {
      if (decisionResponse.status === 408) {
        // Timeout - treat as deny
        return {
          content: [{
            type: "text",
            text: JSON.stringify({
              allowed: false,
              reason: "Permission request timed out waiting for user response",
            }),
          }],
        };
      }
      throw new Error(`Permission decision error: ${decisionResponse.statusText}`);
    }

    const decision = await decisionResponse.json() as PermissionDecision;

    return {
      content: [{
        type: "text",
        text: JSON.stringify({
          allowed: decision.decision === "allow",
          reason: decision.message || (decision.decision === "allow"
            ? "User approved the tool call"
            : "User denied the tool call"),
        }),
      }],
    };
  } catch (error) {
    clearTimeout(timeoutId);
    if (error instanceof Error && error.name === "AbortError") {
      return {
        content: [{
          type: "text",
          text: JSON.stringify({
            allowed: false,
            reason: "Permission request timed out",
          }),
        }],
      };
    }
    throw error;
  }
}
```

**Update MCP Server index.ts:**

```typescript
import { permissionRequestTool, handlePermissionRequest } from "./permission-handler.js";

// Add to tool list (always available, not scoped by agent type)
const PERMISSION_TOOLS = [permissionRequestTool];

// In CallToolRequestSchema handler:
if (name === "permission_request") {
  return handlePermissionRequest(args as Parameters<typeof handlePermissionRequest>[0]);
}
```

---

### Tauri Backend: Permission Endpoints

**File:** `src-tauri/src/http_server.rs` (additions)

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;

/// Pending permission request waiting for user decision
struct PendingPermission {
    request_id: String,
    tool_name: String,
    tool_input: serde_json::Value,
    context: Option<String>,
    response_tx: oneshot::Sender<PermissionDecision>,
}

#[derive(Clone, Serialize)]
struct PermissionDecision {
    decision: String,  // "allow" or "deny"
    message: Option<String>,
}

/// Shared state for pending permissions
pub struct PermissionState {
    pending: Mutex<HashMap<String, PendingPermission>>,
}

impl PermissionState {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }
}

// ============================================================================
// HTTP Endpoints
// ============================================================================

#[derive(Deserialize)]
struct PermissionRequestInput {
    tool_name: String,
    tool_input: serde_json::Value,
    context: Option<String>,
}

#[derive(Serialize)]
struct PermissionRequestResponse {
    request_id: String,
}

/// POST /api/permission/request
///
/// Called by MCP server when Claude CLI needs permission for a tool.
/// Registers the request, emits Tauri event, returns request_id.
async fn request_permission(
    State(state): State<Arc<AppState>>,
    Json(input): Json<PermissionRequestInput>,
) -> Json<PermissionRequestResponse> {
    let request_id = Uuid::new_v4().to_string();
    let (tx, _rx) = oneshot::channel();  // rx stored in pending map

    // Store pending request
    {
        let mut pending = state.permission_state.pending.lock().await;
        pending.insert(request_id.clone(), PendingPermission {
            request_id: request_id.clone(),
            tool_name: input.tool_name.clone(),
            tool_input: input.tool_input.clone(),
            context: input.context.clone(),
            response_tx: tx,
        });
    }

    // Emit Tauri event to frontend
    let _ = state.app_handle.emit("permission:request", serde_json::json!({
        "request_id": request_id,
        "tool_name": input.tool_name,
        "tool_input": input.tool_input,
        "context": input.context,
    }));

    Json(PermissionRequestResponse { request_id })
}

/// GET /api/permission/await/:request_id
///
/// Long-poll endpoint. MCP server calls this and blocks until user decides.
/// Returns 408 on timeout (5 minutes).
async fn await_permission(
    State(state): State<Arc<AppState>>,
    Path(request_id): Path<String>,
) -> Result<Json<PermissionDecision>, StatusCode> {
    // Extract the receiver from pending map
    let rx = {
        let mut pending = state.permission_state.pending.lock().await;
        if let Some(p) = pending.remove(&request_id) {
            // Re-insert without the sender (we took it)
            // Actually, we need a different approach - use a channel per request
            // that we can await on
            Some(p.response_tx)  // This won't work as-is
        } else {
            None
        }
    };

    // Better approach: use a broadcast or watch channel
    // For now, simplified polling approach:
    let timeout = tokio::time::Duration::from_secs(300);
    let start = tokio::time::Instant::now();

    loop {
        // Check if decision has been made
        {
            let pending = state.permission_state.pending.lock().await;
            if !pending.contains_key(&request_id) {
                // Request was resolved - check decisions map
                if let Some(decision) = state.permission_decisions.lock().await.remove(&request_id) {
                    return Ok(Json(decision));
                }
            }
        }

        if start.elapsed() > timeout {
            // Clean up and return timeout
            state.permission_state.pending.lock().await.remove(&request_id);
            return Err(StatusCode::REQUEST_TIMEOUT);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

#[derive(Deserialize)]
struct ResolvePermissionInput {
    request_id: String,
    decision: String,  // "allow" or "deny"
    message: Option<String>,
}

/// POST /api/permission/resolve
///
/// Called by frontend when user makes a decision.
async fn resolve_permission(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ResolvePermissionInput>,
) -> StatusCode {
    // Store decision for the await endpoint to pick up
    state.permission_decisions.lock().await.insert(
        input.request_id.clone(),
        PermissionDecision {
            decision: input.decision,
            message: input.message,
        },
    );

    // Remove from pending
    state.permission_state.pending.lock().await.remove(&input.request_id);

    StatusCode::OK
}

// Add routes to router:
// .route("/api/permission/request", post(request_permission))
// .route("/api/permission/await/:request_id", get(await_permission))
// .route("/api/permission/resolve", post(resolve_permission))
```

**Alternative: Cleaner implementation with tokio::sync::watch**

```rust
use tokio::sync::watch;

pub struct PermissionState {
    pending: Mutex<HashMap<String, watch::Sender<Option<PermissionDecision>>>>,
}

async fn request_permission(...) -> Json<PermissionRequestResponse> {
    let request_id = Uuid::new_v4().to_string();
    let (tx, _rx) = watch::channel(None);

    state.permission_state.pending.lock().await
        .insert(request_id.clone(), tx);

    // Emit event...

    Json(PermissionRequestResponse { request_id })
}

async fn await_permission(...) -> Result<Json<PermissionDecision>, StatusCode> {
    let mut rx = {
        let pending = state.permission_state.pending.lock().await;
        pending.get(&request_id)
            .map(|tx| tx.subscribe())
            .ok_or(StatusCode::NOT_FOUND)?
    };

    let timeout = tokio::time::Duration::from_secs(300);

    match tokio::time::timeout(timeout, rx.wait_for(|v| v.is_some())).await {
        Ok(Ok(_)) => {
            let decision = rx.borrow().clone().unwrap();
            state.permission_state.pending.lock().await.remove(&request_id);
            Ok(Json(decision))
        }
        _ => {
            state.permission_state.pending.lock().await.remove(&request_id);
            Err(StatusCode::REQUEST_TIMEOUT)
        }
    }
}

async fn resolve_permission(...) -> StatusCode {
    let pending = state.permission_state.pending.lock().await;
    if let Some(tx) = pending.get(&input.request_id) {
        let _ = tx.send(Some(PermissionDecision {
            decision: input.decision,
            message: input.message,
        }));
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}
```

---

### Tauri Command for Frontend

**File:** `src-tauri/src/commands/permission_commands.rs`

```rust
use tauri::State;
use crate::AppState;

#[derive(serde::Deserialize)]
pub struct ResolvePermissionArgs {
    request_id: String,
    decision: String,
    message: Option<String>,
}

#[tauri::command]
pub async fn resolve_permission_request(
    state: State<'_, AppState>,
    args: ResolvePermissionArgs,
) -> Result<(), String> {
    let pending = state.permission_state.pending.lock().await;

    if let Some(tx) = pending.get(&args.request_id) {
        tx.send(Some(PermissionDecision {
            decision: args.decision,
            message: args.message,
        })).map_err(|_| "Failed to send decision")?;
        Ok(())
    } else {
        Err("Permission request not found".to_string())
    }
}

#[tauri::command]
pub async fn get_pending_permissions(
    state: State<'_, AppState>,
) -> Vec<PendingPermissionInfo> {
    let pending = state.permission_state.pending.lock().await;
    pending.values()
        .map(|p| PendingPermissionInfo {
            request_id: p.request_id.clone(),
            tool_name: p.tool_name.clone(),
            tool_input: p.tool_input.clone(),
            context: p.context.clone(),
        })
        .collect()
}
```

---

### Frontend: Permission Dialog Component

**File:** `src/components/PermissionDialog.tsx`

```tsx
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogDescription,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { AlertTriangle, Shield, Terminal } from "lucide-react";
import { cn } from "@/lib/utils";

interface PermissionRequest {
  request_id: string;
  tool_name: string;
  tool_input: Record<string, unknown>;
  context?: string;
}

export function PermissionDialog() {
  const [requests, setRequests] = useState<PermissionRequest[]>([]);
  const currentRequest = requests[0];

  useEffect(() => {
    const unlisten = listen<PermissionRequest>("permission:request", (event) => {
      setRequests((prev) => [...prev, event.payload]);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleDecision = async (decision: "allow" | "deny") => {
    if (!currentRequest) return;

    try {
      await invoke("resolve_permission_request", {
        args: {
          request_id: currentRequest.request_id,
          decision,
          message: decision === "deny" ? "User denied permission" : undefined,
        },
      });
    } catch (error) {
      console.error("Failed to resolve permission:", error);
    }

    // Remove from queue
    setRequests((prev) => prev.slice(1));
  };

  if (!currentRequest) return null;

  const toolInputPreview = formatToolInput(currentRequest.tool_name, currentRequest.tool_input);

  return (
    <Dialog open onOpenChange={() => handleDecision("deny")}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <div className="flex items-center gap-2">
            <div className="p-2 rounded-full bg-warning/10">
              <AlertTriangle className="h-5 w-5 text-warning" />
            </div>
            <DialogTitle>Permission Required</DialogTitle>
          </div>
          <DialogDescription>
            An agent is requesting permission to use a tool
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* Tool name */}
          <div className="flex items-center gap-2 text-sm">
            <Terminal className="h-4 w-4 text-muted-foreground" />
            <span className="font-medium">{currentRequest.tool_name}</span>
          </div>

          {/* Tool input preview */}
          <div className="rounded-md bg-muted p-3 font-mono text-sm overflow-x-auto">
            <pre className="whitespace-pre-wrap break-all">
              {toolInputPreview}
            </pre>
          </div>

          {/* Context if provided */}
          {currentRequest.context && (
            <p className="text-sm text-muted-foreground">
              {currentRequest.context}
            </p>
          )}

          {/* Queue indicator */}
          {requests.length > 1 && (
            <p className="text-xs text-muted-foreground">
              +{requests.length - 1} more permission request(s) waiting
            </p>
          )}
        </div>

        <DialogFooter className="gap-2 sm:gap-0">
          <Button
            variant="outline"
            onClick={() => handleDecision("deny")}
          >
            Deny
          </Button>
          <Button
            onClick={() => handleDecision("allow")}
            className="bg-primary"
          >
            <Shield className="h-4 w-4 mr-2" />
            Allow
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function formatToolInput(toolName: string, input: Record<string, unknown>): string {
  // Special formatting for common tools
  switch (toolName) {
    case "Bash":
      return input.command as string || JSON.stringify(input, null, 2);
    case "Write":
      return `Write to: ${input.file_path}\n\n${(input.content as string)?.slice(0, 200)}${
        (input.content as string)?.length > 200 ? "..." : ""
      }`;
    case "Edit":
      return `Edit: ${input.file_path}\n- "${input.old_string}"\n+ "${input.new_string}"`;
    case "Read":
      return `Read: ${input.file_path}`;
    default:
      return JSON.stringify(input, null, 2);
  }
}
```

**File:** `src/components/PermissionDialog.module.css` (optional styling)

---

### Update Claude CLI Spawn to Use Permission Handler

**File:** `src-tauri/src/infrastructure/agents/claude/claude_code_client.rs`

```rust
impl ClaudeCodeClient {
    pub async fn spawn_agent(&self, config: AgentSpawnConfig) -> Result<AgentHandle, AgentError> {
        let mut cmd = Command::new(&self.cli_path);

        cmd.args([
            "--plugin-dir", "./ralphx-plugin",
            "--output-format", "stream-json",
        ]);

        // Add permission prompt tool for UI-based approval
        // The MCP tool name format: mcp__<server>__<tool>
        cmd.args([
            "--permission-prompt-tool",
            "mcp__ralphx__permission_request"
        ]);

        // Pass agent type for MCP tool scoping
        cmd.env("RALPHX_AGENT_TYPE", &config.agent);

        // ... rest of spawn logic
    }
}
```

---

### Frontend Integration

**File:** `src/App.tsx` (or root layout)

```tsx
import { PermissionDialog } from "@/components/PermissionDialog";

function App() {
  return (
    <>
      {/* ... existing app content */}

      {/* Global permission dialog - always mounted */}
      <PermissionDialog />
    </>
  );
}
```

---

### Files Summary for Permission Bridge

**New Files:**

| File | Purpose |
|------|---------|
| `ralphx-mcp-server/src/permission-handler.ts` | MCP tool that handles permission prompts |
| `src-tauri/src/application/permission_state.rs` | Shared state for pending permissions |
| `src-tauri/src/commands/permission_commands.rs` | Tauri commands for permission resolution |
| `src/components/PermissionDialog.tsx` | UI for permission approval/denial |
| `src/types/permission.ts` | TypeScript types for permission events |

**Modified Files:**

| File | Change |
|------|--------|
| `ralphx-mcp-server/src/index.ts` | Register permission_request tool |
| `src-tauri/src/http_server.rs` | Add permission endpoints |
| `src-tauri/src/lib.rs` | Initialize PermissionState, register commands |
| `src-tauri/src/infrastructure/agents/claude/claude_code_client.rs` | Add `--permission-prompt-tool` flag |
| `src/App.tsx` | Mount PermissionDialog globally |

---

## Implementation Order (Updated)

### Phase 1: Database & Entity Updates
1. Create migration: `add_claude_session_and_tool_calls.sql`
   - Add `chat_conversations` table
   - Add `tool_calls` column to `chat_messages`
2. Update `ChatMessage` entity in Rust
3. Update `ChatMessageSchema` in frontend types

### Phase 2: MCP Server
1. Create `ralphx-mcp-server` with basic structure
2. Implement ideation tools (create_task_proposal, etc.)
3. Add HTTP server to Tauri backend for MCP proxy pattern
4. Test standalone with MCP inspector

### Phase 3: Refactor Orchestrator Service
1. Remove tool execution methods (MCP handles this now)
2. Add `claude_session_id` capture from stream-json `result` event
3. Add `--resume` flag logic for follow-up messages
4. Parse tool calls from stream-json and store them (for UI display)
5. Create `ChatConversationRepository` for session mapping

### Phase 4: Agent Configuration
1. Update `ralphx-plugin/.mcp.json` with MCP server config
2. Update `orchestrator-ideation.md` agent (inherits MCP tools)
3. Create `chat-task.md` and `chat-project.md` agents
4. Test with `claude --agent` + `--resume` commands

### Phase 5: Frontend Updates
1. Update `chat.ts` API to send messages via new backend
2. Update `useChat.ts` hook
3. Add `ToolCallIndicator` component to display tool calls
4. Update `ChatPanel.tsx` - hide without session, show tool calls
5. Fix ideation view behavior

### Phase 6: Conversation History & Switching
1. Add `ConversationSelector` component (similar to project selector)
2. Add history icon button in `ChatPanel` header
3. When clicked, show dropdown of conversations for current context
4. Display conversation title/date, message count, last activity
5. Clicking a conversation switches to it (loads messages, changes active conversation)
6. "New Conversation" option at top of list

### Phase 7: Message Queue System
1. Add `queuedMessages` state to chat store
2. Allow user to "send" while agent is running → adds to queue
3. Display queued messages below the input (styled differently - pending state)
4. When agent run completes, auto-send first queued message
5. Repeat until queue is empty
6. Add keyboard navigation:
   - Up arrow in empty input → edit last queued message
   - Enter while editing → save changes
   - Escape → cancel edit
7. Add inline edit/delete for queued messages:
   - Click on queued message → edit mode
   - X button → delete from queue

### Phase 8: Permission Bridge System
1. Add `permission-handler.ts` to MCP server with `permission_request` tool
2. Register tool in MCP server index (not scoped by agent type - always available)
3. Add `PermissionState` to Tauri backend with pending permissions map
4. Add HTTP endpoints: `/api/permission/request`, `/api/permission/await/:id`, `/api/permission/resolve`
5. Add Tauri commands: `resolve_permission_request`, `get_pending_permissions`
6. Update `ClaudeCodeClient` to pass `--permission-prompt-tool mcp__ralphx__permission_request`
7. Create `PermissionDialog.tsx` component with tool preview formatting
8. Mount `PermissionDialog` globally in App root

### Phase 9: Functional Testing

**Note:** Focus on unit and integration tests that can be run by an autonomous agent. No end-to-end browser testing required.

**Backend (Rust) - `cargo test`:**
1. `permission_state_tests.rs`:
   - Test adding pending permission request
   - Test resolving permission with allow decision
   - Test resolving permission with deny decision
   - Test timeout behavior (mock time)
   - Test concurrent permission requests
   - Test request not found error
2. `chat_conversation_repo_tests.rs`:
   - Test create conversation with context
   - Test update claude_session_id
   - Test get conversation by context
   - Test multiple conversations per context
3. `context_chat_service_tests.rs`:
   - Test first message creates conversation
   - Test follow-up message uses --resume
   - Test claude_session_id capture from response
   - Test tool call extraction from stream-json
4. `http_server_tests.rs`:
   - Test permission request endpoint returns request_id
   - Test permission resolve endpoint signals waiter
   - Test permission await endpoint timeout

**MCP Server (TypeScript) - `npm test`:**
1. `permission-handler.test.ts`:
   - Test permission_request tool schema validation
   - Test HTTP call to Tauri backend (mock fetch)
   - Test long-poll timeout handling
   - Test allow decision response format
   - Test deny decision response format
2. `tools.test.ts`:
   - Test tool scoping by RALPHX_AGENT_TYPE
   - Test unauthorized tool call rejection
   - Test tool list filtering

**Frontend (React) - `npm run test`:**
1. `PermissionDialog.test.tsx`:
   - Test dialog renders on permission:request event
   - Test tool input formatting for Bash commands
   - Test tool input formatting for Write/Edit/Read
   - Test Allow button calls resolve with "allow"
   - Test Deny button calls resolve with "deny"
   - Test multiple queued requests show count
   - Test dialog closes after decision
2. `chatStore.test.ts`:
   - Test queueMessage adds to queue
   - Test editQueuedMessage updates content
   - Test deleteQueuedMessage removes from queue
   - Test processQueue sends first message
3. `ConversationSelector.test.tsx`:
   - Test renders conversation list
   - Test clicking conversation switches active
   - Test "New Conversation" option
