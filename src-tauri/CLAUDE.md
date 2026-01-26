# src-tauri Backend CLAUDE.md

This file provides guidance for working with the RalphX Tauri/Rust backend.

---

## Tech Stack

| Technology | Version | Purpose |
|------------|---------|---------|
| **Rust** | 2021 edition | Primary language |
| **Tauri** | 2.0 | Desktop app framework, IPC bridge |
| **SQLite** | rusqlite 0.32 | Persistent storage |
| **statig** | 0.3 | State machine library (async) |
| **tokio** | 1.x | Async runtime |
| **serde** | 1.x | Serialization (JSON) |
| **chrono** | 0.4 | Date/time handling |
| **thiserror** | 1.x | Error types |
| **async-trait** | 0.1 | Async trait support |
| **tracing** | 0.1 | Structured logging |
| **uuid** | 1.x | ID generation (v4) |

---

## Directory Structure

```
src-tauri/
├── Cargo.toml              # Dependencies and crate config
├── tauri.conf.json         # Tauri app configuration
├── build.rs                # Build script
├── ralphx.db               # SQLite database (dev)
│
├── src/
│   ├── main.rs             # Entry point (calls lib.rs run())
│   ├── lib.rs              # App setup, Tauri command registration
│   ├── error.rs            # AppError enum, AppResult type alias
│   │
│   ├── domain/             # Business logic (no infrastructure deps)
│   │   ├── entities/       # Core data types (Task, Project, etc.)
│   │   ├── repositories/   # Repository traits (interfaces)
│   │   ├── state_machine/  # Task lifecycle state machine
│   │   ├── agents/         # Agent abstraction (AgenticClient trait)
│   │   ├── supervisor/     # Supervisor events and patterns
│   │   ├── qa/             # QA settings and criteria
│   │   ├── review/         # Review configuration
│   │   ├── services/       # Domain services
│   │   └── tools/          # Tool definitions for agents
│   │
│   ├── application/        # Application services and state
│   │   ├── app_state.rs    # AppState (DI container)
│   │   ├── qa_service.rs   # QA orchestration
│   │   ├── review_service.rs
│   │   ├── supervisor_service.rs
│   │   ├── ideation_service.rs
│   │   ├── dependency_service.rs
│   │   ├── priority_service.rs
│   │   ├── apply_service.rs
│   │   ├── orchestrator_service.rs  # Context-aware chat with --resume support
│   │   ├── permission_state.rs      # Permission bridge for UI-based tool approval
│   │   └── http_server.rs           # HTTP server (port 3847) for MCP proxy
│   │
│   ├── commands/           # Tauri commands (thin IPC layer)
│   │   ├── task_commands.rs
│   │   ├── project_commands.rs
│   │   ├── ideation_commands.rs
│   │   ├── context_chat_commands.rs  # Context-aware chat commands (Phase 15)
│   │   ├── permission_commands.rs    # Permission resolution commands
│   │   ├── workflow_commands.rs
│   │   ├── artifact_commands.rs
│   │   ├── research_commands.rs
│   │   ├── methodology_commands.rs
│   │   └── ...
│   │
│   ├── infrastructure/     # External implementations
│   │   ├── sqlite/         # SQLite repositories
│   │   │   ├── connection.rs
│   │   │   ├── migrations.rs
│   │   │   └── sqlite_*.rs
│   │   ├── memory/         # In-memory repos (testing)
│   │   │   └── memory_*.rs
│   │   ├── agents/         # Agent client implementations
│   │   │   ├── claude/     # Claude Code CLI client
│   │   │   └── mock/       # Mock client for tests
│   │   └── supervisor/     # Event bus implementation
│   │
│   └── testing/            # Test utilities and helpers
│
└── tests/                  # Integration tests
    ├── state_machine_flows.rs
    ├── repository_swapping.rs
    ├── agentic_client_flows.rs
    ├── supervisor_integration.rs
    ├── qa_system_flows.rs
    ├── review_flows.rs
    └── execution_control_flows.rs
```

---

## Architecture Patterns

### Clean Architecture / Hexagonal Architecture

The codebase follows clean architecture principles:

```
┌─────────────────────────────────────────────────────────────┐
│                     Commands (Tauri IPC)                    │
├─────────────────────────────────────────────────────────────┤
│                   Application Services                       │
├─────────────────────────────────────────────────────────────┤
│                      Domain Layer                            │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────┐ │
│  │  Entities   │  │ Repository  │  │   State Machine      │ │
│  │  (Task,     │  │  Traits     │  │   (TaskStateMachine) │ │
│  │   Project)  │  │ (TaskRepo,  │  │                      │ │
│  │             │  │  ProjectR.) │  │                      │ │
│  └─────────────┘  └─────────────┘  └──────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                   Infrastructure Layer                       │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────┐ │
│  │   SQLite    │  │   Memory    │  │   Claude Code CLI    │ │
│  │   Repos     │  │   Repos     │  │   Client             │ │
│  └─────────────┘  └─────────────┘  └──────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

**Key principle:** Domain layer has NO dependencies on infrastructure.

### Repository Pattern

Repository traits are defined in `domain/repositories/` and implemented in:
- `infrastructure/sqlite/` - Production (SQLite)
- `infrastructure/memory/` - Testing (in-memory)

Example:
```rust
// domain/repositories/task_repository.rs
#[async_trait]
pub trait TaskRepository: Send + Sync {
    async fn create(&self, task: Task) -> AppResult<Task>;
    async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>>;
    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<Task>>;
    // ...
}

// infrastructure/sqlite/sqlite_task_repo.rs
impl TaskRepository for SqliteTaskRepository { ... }

// infrastructure/memory/memory_task_repo.rs
impl TaskRepository for MemoryTaskRepository { ... }
```

### Newtype Pattern (Type-Safe IDs)

All entity IDs use the newtype pattern to prevent accidental mixing:

```rust
// domain/entities/types.rs
pub struct TaskId(pub String);
pub struct ProjectId(pub String);
pub struct IdeationSessionId(pub String);
// ... etc

impl TaskId {
    pub fn new() -> Self { Self(uuid::Uuid::new_v4().to_string()) }
    pub fn from_string(s: String) -> Self { Self(s) }
    pub fn as_str(&self) -> &str { &self.0 }
}
```

This ensures compile-time safety - you cannot pass a `TaskId` where a `ProjectId` is expected.

### Dependency Injection via AppState

`AppState` is the DI container, holding all repository trait objects:

```rust
// application/app_state.rs
pub struct AppState {
    pub task_repo: Arc<dyn TaskRepository>,
    pub project_repo: Arc<dyn ProjectRepository>,
    pub agent_client: Arc<dyn AgenticClient>,
    // ... 16+ repositories
}

impl AppState {
    pub fn new_production() -> AppResult<Self> { ... }  // SQLite repos
    pub fn new_test() -> Self { ... }                   // Memory repos
    pub fn with_repos(...) -> Self { ... }              // Custom repos
}
```

### State Machine (statig-inspired)

Task lifecycle uses a 14-state machine defined in `domain/state_machine/`:

```
Backlog → Ready → Executing → ExecutionDone → QaRefining → QaTesting →
QaPassed → PendingReview → Approved

With failure paths:
- Executing → Failed / Blocked
- QaTesting → QaFailed → RevisionNeeded → Executing
- PendingReview → RevisionNeeded → Executing
```

Key files:
- `machine.rs` - State handlers and dispatch
- `events.rs` - TaskEvent enum
- `types.rs` - State data (FailedData, QaFailedData)
- `context.rs` - TaskContext (shared data)
- `persistence.rs` - SQLite state persistence

---

## Key Entities

| Entity | File | Description |
|--------|------|-------------|
| `Task` | `entities/task.rs` | Work item with status, priority, timestamps |
| `Project` | `entities/project.rs` | Project container with path and git mode |
| `InternalStatus` | `entities/status.rs` | 14-state enum with transition rules |
| `TaskQA` | `entities/task_qa.rs` | QA test criteria and results |
| `Review` | `entities/review.rs` | Code review records |
| `IdeationSession` | `entities/ideation.rs` | Chat-based ideation session |
| `TaskProposal` | `entities/ideation.rs` | Proposed task from ideation |
| `ChatMessage` | `entities/ideation.rs` | Chat messages (with tool_calls field) |
| `ChatConversation` | `entities/chat_conversation.rs` | Chat conversation with Claude session tracking (Phase 15) |
| `AgentRun` | `entities/agent_run.rs` | Agent execution status tracking (Phase 15) |
| `WorkflowSchema` | `entities/workflow.rs` | Kanban column configuration |
| `Artifact` | `entities/artifact.rs` | Generated artifacts (PRD, etc.) |
| `ResearchProcess` | `entities/research.rs` | Research task tracking |
| `MethodologyExtension` | `entities/methodology.rs` | BMAD/GSD methodology support |

---

## Commands (Tauri IPC)

Commands are thin wrappers that delegate to repositories/services:

```rust
// commands/task_commands.rs
#[tauri::command]
pub async fn list_tasks(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Task>, AppError> {
    let project_id = ProjectId::from_string(project_id);
    state.task_repo.get_by_project(&project_id).await
}
```

Command categories:
- **Task commands** - CRUD, status changes, blocking
- **Project commands** - Project management
- **Ideation commands** - Sessions, proposals, chat, orchestrator
- **Context chat commands** - Context-aware chat with conversations, agent runs (Phase 15)
- **Permission commands** - Permission resolution for UI-based tool approval (Phase 15)
- **Workflow commands** - Custom workflow schemas
- **Artifact commands** - Artifact and bucket management
- **Research commands** - Research process control
- **Methodology commands** - Methodology activation
- **QA commands** - QA settings and results
- **Review commands** - Code review operations
- **Execution commands** - Pause/resume/stop execution

---

## Error Handling

Unified error type in `error.rs`:

```rust
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Invalid status transition: {from} -> {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

pub type AppResult<T> = Result<T, AppError>;
```

Errors implement `Serialize` for Tauri IPC.

---

## Testing Approach

### Unit Tests

Every module has inline unit tests in `#[cfg(test)]` blocks:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_new_creates_with_defaults() { ... }

    #[tokio::test]
    async fn test_repository_create() { ... }
}
```

### Integration Tests

Located in `tests/` directory:
- `state_machine_flows.rs` - Full task lifecycle flows
- `repository_swapping.rs` - Verify DI works
- `agentic_client_flows.rs` - Agent spawning/communication
- `supervisor_integration.rs` - Event bus and supervision
- `qa_system_flows.rs` - QA preparation and testing
- `review_flows.rs` - Review and approval flows
- `execution_control_flows.rs` - Pause/resume/stop

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_task_new

# Run integration tests only
cargo test --test state_machine_flows
```

---

## Build & Run

### Development

```bash
# From project root (not src-tauri)
npm run tauri dev

# Or from src-tauri
cargo build
cargo run
```

### Production Build

```bash
npm run tauri build
```

### Check/Lint

```bash
cargo check
cargo clippy
cargo fmt --check
```

---

## Database

SQLite database at `ralphx.db` (dev) or app data directory (production).

### Migrations

All migrations in `infrastructure/sqlite/migrations.rs`:
- Migrations run automatically on startup via `run_migrations()`
- Version tracked in `schema_version` table
- Add new migrations to `MIGRATIONS` array

### Tables

Key tables:
- `tasks` - Task records with status, timestamps
- `projects` - Project records
- `status_transitions` - Audit log for state changes
- `task_qa` - QA criteria and results
- `reviews` - Code review records
- `ideation_sessions` - Ideation sessions
- `task_proposals` - Task proposals
- `proposal_dependencies` - Proposal DAG
- `chat_messages` - Chat history (with tool_calls JSON field)
- `chat_conversations` - Chat conversations with Claude session IDs (Phase 15)
- `agent_runs` - Agent execution tracking (Phase 15)
- `task_dependencies` - Task blockers
- `workflows` - Workflow schemas
- `artifacts` / `artifact_buckets` - Artifact system
- `research_processes` - Research tracking
- `methodologies` - Methodology extensions

---

## Conventions

### Naming

- **Types**: PascalCase (`TaskId`, `InternalStatus`)
- **Functions/methods**: snake_case (`get_by_id`, `create_task`)
- **Files**: snake_case (`task_repository.rs`)
- **Modules**: snake_case (`state_machine`)

### Serialization

- All API types use `#[serde(rename_all = "snake_case")]` for enums
- JSON field names are snake_case
- Dates are RFC3339 format

### Async

- All repository methods are `async`
- Use `#[async_trait]` for async traits
- Tokio runtime for async execution

### Error Handling

- Return `AppResult<T>` from all fallible functions
- Use `?` for error propagation
- Convert infrastructure errors to `AppError`

### Testing

- TDD is mandatory - write tests first
- Every public function should have tests
- Use in-memory repos for unit tests
- Use `tempfile` for integration tests needing files

---

## Important Files Quick Reference

| Purpose | File |
|---------|------|
| App entry | `lib.rs` |
| DI container | `application/app_state.rs` |
| Error types | `error.rs` |
| Task entity | `domain/entities/task.rs` |
| Status enum | `domain/entities/status.rs` |
| Task repo trait | `domain/repositories/task_repository.rs` |
| SQLite task repo | `infrastructure/sqlite/sqlite_task_repo.rs` |
| State machine | `domain/state_machine/machine.rs` |
| Agent trait | `domain/agents/agentic_client.rs` |
| Migrations | `infrastructure/sqlite/migrations.rs` |

---

## Agent System

### AgenticClient Trait

Abstraction for AI agents (Claude Code, future: Codex, Gemini):

```rust
#[async_trait]
pub trait AgenticClient: Send + Sync {
    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<AgentHandle>;
    async fn stop_agent(&self, handle: &AgentHandle) -> AgentResult<()>;
    async fn wait_for_completion(&self, handle: &AgentHandle) -> AgentResult<AgentOutput>;
    async fn send_prompt(&self, handle: &AgentHandle, prompt: &str) -> AgentResult<AgentResponse>;
    fn stream_response(&self, handle: &AgentHandle, prompt: &str)
        -> Pin<Box<dyn Stream<Item = AgentResult<ResponseChunk>> + Send>>;
    fn capabilities(&self) -> &ClientCapabilities;
    async fn is_available(&self) -> AgentResult<bool>;
}
```

### Implementations

- `ClaudeCodeClient` - Production (spawns `claude` CLI)
- `MockAgenticClient` - Testing (returns canned responses)

### Agent Roles

- `Worker` - Executes tasks
- `Reviewer` - Reviews implementations
- `Supervisor` - Oversees execution
- `QA` - Runs QA tests
- `Orchestrator` - Handles ideation chat (with MCP tools)
- `chat-task` - Task-focused chat (with MCP tools)
- `chat-project` - Project-focused chat (with MCP tools)

---

## Context-Aware Chat System (Phase 15)

### Architecture Overview

The context-aware chat system enables multi-conversation chat with full MCP tool integration:

```
Frontend (React)
    ↓ Tauri IPC
Backend (Rust)
    ├─→ HTTP Server (port 3847) ─→ MCP Server (TypeScript proxy)
    │                                      ↓
    │                              RalphX business logic via HTTP
    └─→ Claude CLI (--agent flag, --resume for continuation)
           ├─→ RALPHX_AGENT_TYPE env var passed to MCP
           └─→ MCP Server returns scoped tools per agent type
```

### Key Components

**Backend:**
- `http_server.rs` - Axum HTTP server (port 3847) exposing RalphX operations to MCP server
- `orchestrator_service.rs` - Spawns Claude CLI with `--agent` and `--resume` flags, captures session IDs
- `permission_state.rs` - Long-polling permission bridge for UI-based tool approval
- `context_chat_commands.rs` - Tauri commands for sending messages, managing conversations
- `permission_commands.rs` - Tauri commands for resolving permission requests
- `chat_conversation_repository.rs` - Manages conversations with Claude session ID tracking
- `agent_run_repository.rs` - Tracks agent execution status (running/completed/failed)

**MCP Server (ralphx-mcp-server/):**
- TypeScript proxy that exposes RalphX tools to Claude via MCP protocol
- Reads `RALPHX_AGENT_TYPE` env var to scope tools per agent
- Forwards all tool calls to Tauri backend via HTTP (no business logic in MCP)
- Implements `permission_request` MCP tool for UI-based approval

**Tool Scoping:**
| Agent Type | Allowed MCP Tools |
|------------|------------------|
| `orchestrator-ideation` | create_task_proposal, update_task_proposal, delete_task_proposal, add_proposal_dependency |
| `chat-task` | update_task, add_task_note, get_task_details |
| `chat-project` | suggest_task, list_tasks |
| `reviewer` | complete_review (submit review decision) |
| `worker`, `supervisor`, `qa-prep`, `qa-tester` | None (no MCP tools) |

### Session Management

**Two types of IDs:**
1. **RalphX Context ID** - Our internal IDs (ideation session, task, project)
2. **Claude Session ID** - Claude's session ID for `--resume` flag

**Flow:**
1. User sends first message → Claude CLI spawned with `--agent orchestrator-ideation`
2. Claude returns `session_id` in stream-json output → stored in `chat_conversations.claude_session_id`
3. User sends follow-up → Claude CLI spawned with `--resume <session_id>` → Claude remembers full context

### Permission Bridge

Enables UI-based approval for non-pre-approved tools:

1. Agent calls `permission_request` MCP tool
2. MCP server POSTs to `/api/permission/request` → returns `request_id`
3. Tauri backend emits `permission:request` event → PermissionDialog appears
4. MCP server long-polls `/api/permission/await/:request_id` (5 min timeout)
5. User clicks Allow/Deny → calls `resolve_permission_request` Tauri command
6. Backend signals waiting MCP request → MCP returns decision to Claude CLI
7. Claude CLI continues or stops based on decision

### Events

The orchestrator service emits real-time Tauri events:
- `chat:message_created` - New message saved
- `chat:chunk` - Streaming response chunk
- `chat:tool_call` - Tool call detected in stream
- `chat:run_completed` - Agent finished (triggers queue processing)
