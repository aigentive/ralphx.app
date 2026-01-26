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
│   │   ├── entities/       # Core data types (Task, Project, TaskContext - Phase 17, etc.)
│   │   ├── repositories/   # Repository traits (interfaces)
│   │   ├── state_machine/  # Task lifecycle state machine
│   │   ├── agents/         # Agent abstraction (AgenticClient trait)
│   │   ├── supervisor/     # Supervisor events and patterns
│   │   ├── qa/             # QA settings and criteria
│   │   ├── review/         # Review configuration
│   │   ├── ideation/       # Ideation domain (IdeationSettings config - Phase 16)
│   │   ├── services/       # Domain services (ExecutionMessageQueue, etc.)
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
│   │   ├── execution_chat_service.rs # Task execution chat with persistence (Phase 15B)
│   │   ├── task_context_service.rs  # Task context aggregation (Phase 17)
│   │   ├── permission_state.rs      # Permission bridge for UI-based tool approval
│   │   └── http_server.rs           # HTTP server (port 3847) for MCP proxy
│   │
│   ├── commands/           # Tauri commands (thin IPC layer)
│   │   ├── task_commands.rs
│   │   ├── task_context_commands.rs  # Task context commands (Phase 17)
│   │   ├── project_commands.rs
│   │   ├── ideation_commands.rs
│   │   ├── context_chat_commands.rs  # Context-aware chat commands (Phase 15A)
│   │   ├── execution_chat_commands.rs # Execution chat commands (Phase 15B)
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
| `TaskContext` | `entities/task_context.rs` | Task context aggregation (Phase 17) |
| `TaskProposalSummary` | `entities/task_context.rs` | Proposal summary for worker context (Phase 17) |
| `ArtifactSummary` | `entities/task_context.rs` | Artifact summary with 500-char preview (Phase 17) |
| `Project` | `entities/project.rs` | Project container with path and git mode |
| `InternalStatus` | `entities/status.rs` | 14-state enum with transition rules |
| `TaskQA` | `entities/task_qa.rs` | QA test criteria and results |
| `Review` | `entities/review.rs` | Code review records |
| `IdeationSession` | `entities/ideation.rs` | Chat-based ideation session |
| `TaskProposal` | `entities/ideation.rs` | Proposed task from ideation (includes planArtifactId - Phase 16) |
| `ChatMessage` | `entities/ideation.rs` | Chat messages (with tool_calls field) |
| `ChatConversation` | `entities/chat_conversation.rs` | Chat conversation with Claude session tracking (Phase 15) |
| `AgentRun` | `entities/agent_run.rs` | Agent execution status tracking (Phase 15) |
| `IdeationSettings` | `domain/ideation/config.rs` | Ideation plan mode configuration (Phase 16) |
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
- **Task context commands** - Task context aggregation, artifact fetching (Phase 17)
- **Project commands** - Project management
- **Ideation commands** - Sessions, proposals, chat, orchestrator
- **Context chat commands** - Context-aware chat with conversations, agent runs (Phase 15A)
- **Execution chat commands** - Task execution chat with persistence, queue management (Phase 15B)
- **Permission commands** - Permission resolution for UI-based tool approval (Phase 15A)
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
- `ideation_settings` - Ideation plan mode configuration (single-row, Phase 16)
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

## Context-Aware Chat System (Phase 15A & 15B)

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
- `orchestrator_service.rs` - Spawns Claude CLI with `--agent` and `--resume` flags, captures session IDs (Phase 15A)
- `execution_chat_service.rs` - Spawns worker with persistence, processes message queue (Phase 15B)
- `execution_message_queue.rs` - In-memory per-task message queue (Phase 15B)
- `permission_state.rs` - Long-polling permission bridge for UI-based tool approval
- `context_chat_commands.rs` - Tauri commands for sending messages, managing conversations (Phase 15A)
- `execution_chat_commands.rs` - Tauri commands for execution conversations, queue management (Phase 15B)
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
| `orchestrator-ideation` | create_task_proposal, update_task_proposal, delete_task_proposal, add_proposal_dependency, create_plan_artifact, update_plan_artifact, get_plan_artifact, link_proposals_to_plan, get_session_plan |
| `chat-task` | update_task, add_task_note, get_task_details |
| `chat-project` | suggest_task, list_tasks |
| `reviewer` | complete_review (submit review decision) |
| `worker` | get_task_context, get_artifact, get_artifact_version, get_related_artifacts, search_project_artifacts (Phase 17) |
| `supervisor`, `qa-prep`, `qa-tester` | None (no MCP tools) |

### Session Management

**Two types of IDs:**
1. **RalphX Context ID** - Our internal IDs (ideation session, task, project)
2. **Claude Session ID** - Claude's session ID for `--resume` flag

**Flow:**
1. User sends first message → Claude CLI spawned with `--agent orchestrator-ideation`
2. Claude returns `session_id` in stream-json output → stored in `chat_conversations.claude_session_id`
3. User sends follow-up → Claude CLI spawned with `--resume <session_id>` → Claude remembers full context

### Task Execution Chat (Phase 15B)

**Worker output persistence:**
- Worker execution creates `task_execution` conversation automatically
- All output (text chunks, tool calls) persisted to database
- User can view execution as chat in ChatPanel
- Past execution attempts accessible via ConversationSelector

**Message queue:**
- Messages sent during worker execution are queued (in-memory)
- When worker completes, queue is processed via `--resume`
- Queue is per-task, isolated from other tasks

**Dual event emission:**
Both Activity Stream and ChatPanel receive worker output:
- **ChatPanel**: `execution:chunk`, `execution:tool_call`, `execution:run_completed` (persisted)
- **Activity Stream**: `agent:message` (memory only)

**ExecutionChatService:**
Key service for worker execution with persistence:
- `spawn_with_persistence(agent, task_id)` - Creates conversation, spawns worker, persists output
- `persist_stream_event(conversation_id, event)` - Saves chunks/tool calls to database
- `complete_execution(conversation_id, claude_session_id)` - Processes queued messages via --resume

**ExecutionMessageQueue:**
In-memory queue for per-task messages:
- `queue(task_id, message)` - Add message to queue
- `pop(task_id)` - Get next queued message
- `get_queued(task_id)` - View all queued messages
- `clear(task_id)` - Clear queue for task

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

**Context-aware chat (Phase 15A):**
- `chat:message_created` - New message saved
- `chat:chunk` - Streaming response chunk
- `chat:tool_call` - Tool call detected in stream
- `chat:run_completed` - Agent finished (triggers queue processing)

**Execution chat (Phase 15B):**
- `execution:chunk` - Worker text output (persisted to ChatPanel)
- `execution:tool_call` - Worker tool call (persisted to ChatPanel)
- `execution:run_completed` - Worker finished (triggers queue processing)
- `agent:message` - Worker output (sent to Activity Stream)

**Plan artifacts (Phase 16):**
- `plan:proposals_may_need_update` - Plan updated, proposals may need revision (proactive sync)

---

## Ideation Plan Artifacts (Phase 16)

### Overview

The ideation system supports implementation plans as artifacts before task proposal creation. Users can configure workflow modes (Required, Optional, Parallel), and the orchestrator creates `Specification` artifacts that serve as implementation plans linked to proposals.

### IdeationSettings Entity

**Location:** `domain/ideation/config.rs`

```rust
pub enum IdeationPlanMode {
    Required,   // Plan must be created before proposals
    Optional,   // Plan suggested for complex features (default)
    Parallel,   // Plan and proposals created together
}

pub struct IdeationSettings {
    pub plan_mode: IdeationPlanMode,
    pub require_plan_approval: bool,      // Require explicit approval in Required mode
    pub suggest_plans_for_complex: bool,  // Suggest plans in Optional mode
    pub auto_link_to_session_plan: bool,  // Auto-link proposals to plan
}
```

**Repository:**
- Trait: `domain/repositories/ideation_settings_repository.rs`
- Implementation: `infrastructure/sqlite/sqlite_ideation_settings_repo.rs`
- Single-row pattern (only one settings record)
- Default: Optional mode, no approval required, suggest plans enabled

### Data Model Changes

**IdeationSession entity (`entities/ideation.rs`):**
```rust
pub struct IdeationSession {
    pub id: IdeationSessionId,
    pub plan_artifact_id: Option<ArtifactId>, // Link to implementation plan
    // ... other fields
}
```

**TaskProposal entity (`entities/ideation.rs`):**
```rust
pub struct TaskProposal {
    pub id: TaskProposalId,
    pub plan_artifact_id: Option<ArtifactId>,       // Link to implementation plan
    pub plan_version_at_creation: Option<u32>,      // Plan version when proposal created
    // ... other fields
}
```

**Task entity (`entities/task.rs`):**
```rust
pub struct Task {
    pub id: TaskId,
    pub source_proposal_id: Option<TaskProposalId>, // Traceability to proposal
    pub plan_artifact_id: Option<ArtifactId>,       // Traceability to plan
    // ... other fields
}
```

### HTTP Endpoints for MCP Proxy

**Location:** `application/http_server.rs`

All plan artifact endpoints are exposed on port 3847 for the MCP server to proxy:

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/create_plan_artifact` | POST | Create new plan artifact (Specification type, prd-library bucket) |
| `/api/update_plan_artifact` | POST | Update plan content (creates new version with previous_version_id) |
| `/api/get_plan_artifact/:id` | GET | Fetch plan artifact by ID |
| `/api/link_proposals_to_plan` | POST | Link proposal IDs to plan artifact |
| `/api/get_session_plan/:session_id` | GET | Get current session's plan artifact |
| `/api/get_ideation_settings` | GET | Get ideation settings |
| `/api/update_ideation_settings` | POST | Update ideation settings |

### MCP Tools

The MCP server (ralphx-mcp-server) exposes plan tools to `orchestrator-ideation` agent:

- `create_plan_artifact(session_id, title, content)` - Creates Specification artifact in prd-library bucket
- `update_plan_artifact(artifact_id, content)` - Updates plan, increments version
- `get_plan_artifact(artifact_id)` - Retrieves plan content
- `link_proposals_to_plan(proposal_ids, artifact_id)` - Links proposals to plan
- `get_session_plan(session_id)` - Gets session's current plan

### Proactive Sync ArtifactFlow

**Flow name:** `plan_updated_sync`
**Trigger:** `artifact_updated` event on `Specification` type

**Steps:**
1. `find_linked_proposals` - Query proposals with matching `plan_artifact_id`
2. Emit `plan:proposals_may_need_update` event with:
   - `artifact_id` - Updated plan ID
   - `proposal_ids` - Array of affected proposal IDs

**Frontend handling:**
- Subscribe to `plan:proposals_may_need_update` event
- Show notification: "Plan updated. N proposals may need revision. [Review]"
- Highlight affected proposals
- Provide [Undo] button to revert auto-updates

### Methodology Integration

**Infrastructure (no specific configs yet):**

The system provides generic infrastructure for methodologies to define plan configurations:

```rust
// domain/entities/methodology.rs
pub struct MethodologyPlanArtifactConfig {
    pub artifact_type: String,  // e.g., "Specification", "TechnicalDesign"
    pub bucket_id: String,       // e.g., "prd-library", "design-docs"
}

pub struct MethodologyPlanTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub template_content: String,
}

pub struct MethodologyExtension {
    pub plan_artifact_config: Option<MethodologyPlanArtifactConfig>,
    pub plan_templates: Vec<MethodologyPlanTemplate>,
    // ... other fields
}
```

**Default behavior:**
- When no methodology active: use `Specification` type, `prd-library` bucket
- When methodology active: use methodology's config if provided, else default
- Template selector only shown when methodology provides templates

### Task Traceability

When a proposal is applied to create a task:

```rust
// application/apply_service.rs
pub async fn apply_proposal(&self, proposal_id: &TaskProposalId) -> AppResult<Task> {
    let proposal = self.proposal_repo.get_by_id(proposal_id).await?;

    let task = Task {
        source_proposal_id: Some(proposal.id.clone()),           // Trace to proposal
        plan_artifact_id: proposal.plan_artifact_id.clone(),     // Trace to plan
        // ... other fields
    };

    self.task_repo.create(task).await
}
```

This enables workers to fetch plan context during task execution (Phase 17).

### Database Migration

**Location:** `infrastructure/sqlite/migrations.rs`

```sql
-- Add plan fields to ideation entities
ALTER TABLE ideation_sessions ADD COLUMN plan_artifact_id TEXT;
ALTER TABLE task_proposals ADD COLUMN plan_artifact_id TEXT;
ALTER TABLE task_proposals ADD COLUMN plan_version_at_creation INTEGER;

-- Add traceability fields to tasks
ALTER TABLE tasks ADD COLUMN source_proposal_id TEXT;
ALTER TABLE tasks ADD COLUMN plan_artifact_id TEXT;

-- Create ideation_settings table (single-row pattern)
CREATE TABLE IF NOT EXISTS ideation_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    plan_mode TEXT NOT NULL DEFAULT 'optional',
    require_plan_approval INTEGER NOT NULL DEFAULT 0,
    suggest_plans_for_complex INTEGER NOT NULL DEFAULT 1,
    auto_link_to_session_plan INTEGER NOT NULL DEFAULT 1
);

-- Insert default settings
INSERT OR IGNORE INTO ideation_settings (id, plan_mode) VALUES (1, 'optional');
```

---

## Worker Artifact Context System (Phase 17)

### Overview

Workers can dynamically fetch and use artifacts linked to the task being executed. This provides implementation plans, research documents, and related artifacts as context before beginning work.

### TaskContext Entities

**Location:** `domain/entities/task_context.rs`

```rust
pub struct TaskContext {
    pub task: Task,
    pub source_proposal: Option<TaskProposalSummary>,
    pub plan_artifact: Option<ArtifactSummary>,
    pub related_artifacts: Vec<ArtifactSummary>,
    pub context_hints: Vec<String>,
}

pub struct TaskProposalSummary {
    pub id: TaskProposalId,
    pub title: String,
    pub description: Option<String>,
    pub acceptance_criteria: Vec<String>,
    pub implementation_notes: Option<String>,
    pub plan_version_at_creation: Option<u32>,
}

pub struct ArtifactSummary {
    pub id: ArtifactId,
    pub title: String,
    pub artifact_type: String,
    pub current_version: u32,
    pub content_preview: String, // 500-char preview
}
```

### TaskContextService

**Location:** `application/task_context_service.rs`

Aggregates task context by:
1. Fetching task by ID
2. If `source_proposal_id` present, fetch proposal and create summary
3. If `plan_artifact_id` present, fetch artifact and create summary with 500-char preview
4. Fetch related artifacts via `ArtifactRelation`
5. Generate context hints based on available context
6. Return `TaskContext`

```rust
pub struct TaskContextService {
    task_repo: Arc<dyn TaskRepository>,
    proposal_repo: Arc<dyn TaskProposalRepository>,
    artifact_repo: Arc<dyn ArtifactRepository>,
}

impl TaskContextService {
    pub async fn get_task_context(&self, task_id: &TaskId) -> AppResult<TaskContext> {
        // Aggregates context from multiple repositories
    }
}
```

### HTTP Endpoints for MCP Proxy

**Location:** `application/http_server.rs`

Exposed on port 3847 for MCP server:

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/task_context/:task_id` | GET | Get full task context (TaskContext) |
| `/api/artifact/:artifact_id` | GET | Get full artifact content by ID |
| `/api/artifact/:artifact_id/version/:version` | GET | Get specific version of artifact |
| `/api/artifact/:artifact_id/related` | GET | Get related artifacts (ArtifactRelation[]) |
| `/api/artifacts/search` | POST | Search artifacts by query and type |

### MCP Tools for Workers

**Location:** `ralphx-mcp-server/src/tools/worker-context-tools.ts`

5 tools scoped to `worker` agent type:

| Tool | Parameters | Returns |
|------|-----------|---------|
| `get_task_context` | task_id | TaskContext (task, proposal summary, plan preview, related artifacts) |
| `get_artifact` | artifact_id | Artifact (full content) |
| `get_artifact_version` | artifact_id, version | Artifact (specific version) |
| `get_related_artifacts` | artifact_id, relation_types? | ArtifactRelation[] |
| `search_project_artifacts` | project_id, query, artifact_types? | ArtifactSummary[] |

### Tauri Commands

**Location:** `commands/task_context_commands.rs`

Frontend-facing commands:

```rust
#[tauri::command]
pub async fn get_task_context(task_id: String, state: State<'_, AppState>) -> Result<TaskContext, AppError>;

#[tauri::command]
pub async fn get_artifact_full(artifact_id: String, state: State<'_, AppState>) -> Result<Artifact, AppError>;

#[tauri::command]
pub async fn get_artifact_version(artifact_id: String, version: u32, state: State<'_, AppState>) -> Result<Artifact, AppError>;

#[tauri::command]
pub async fn get_related_artifacts(artifact_id: String, state: State<'_, AppState>) -> Result<Vec<ArtifactRelation>, AppError>;

#[tauri::command]
pub async fn search_artifacts(
    project_id: String,
    query: String,
    artifact_types: Option<Vec<String>>,
    state: State<'_, AppState>
) -> Result<Vec<ArtifactSummary>, AppError>;
```

### Worker Agent Integration

**Location:** `ralphx-plugin/agents/worker.md`

Worker prompt instructs:

1. **Step 1: Get Task Context** - Always call `get_task_context` first
2. **Step 2: Read Implementation Plan** - If `plan_artifact` exists, fetch with `get_artifact`
3. **Step 3: Fetch Related Artifacts** - Optional for complex tasks
4. **Step 4: Begin Implementation** - Start with full context

### Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Manual context fetch** | Workers have agency to decide relevance; keeps initial prompt lean |
| **500-char preview** | Prevents context bloat; full content requires explicit `get_artifact` call |
| **No caching for MVP** | Keep implementation simple; fetches infrequent; can add later |
| **5 MCP tools** | Covers all context needs without overwhelming worker |
| **Worker calls first** | Prompt enforces `get_task_context` as first step |

---
