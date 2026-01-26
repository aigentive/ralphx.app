# src-tauri/CLAUDE.md (COMPACT) — Backend

## Stack
Rust 2021 | Tauri 2.0 | rusqlite 0.32 | statig 0.3 (async state machine)
tokio 1.x | serde 1.x | chrono 0.4 | thiserror 1.x | async-trait 0.1 | tracing 0.1 | uuid 1.x

## Structure
```
src-tauri/
├─ src/
│  ├─ main.rs, lib.rs         # Entry, app setup, command registration
│  ├─ error.rs                # AppError enum, AppResult<T>
│  ├─ domain/
│  │  ├─ entities/            # Task, TaskContext(Ph17), Project, InternalStatus(14), TaskQA, Review,
│  │  │                       # IdeationSession, TaskProposal, ChatMessage, ChatConversation(Ph15),
│  │  │                       # AgentRun(Ph15), WorkflowSchema, Artifact, ResearchProcess, MethodologyExtension
│  │  ├─ repositories/        # Traits (interfaces)
│  │  ├─ state_machine/       # machine.rs, transition_handler.rs, context.rs, events.rs, types.rs
│  │  ├─ agents/              # AgenticClient trait
│  │  ├─ supervisor/          # Events, patterns
│  │  ├─ qa/, review/         # QA settings, review config
│  │  ├─ ideation/            # IdeationSettings (Ph16)
│  │  ├─ services/            # ExecutionMessageQueue
│  │  └─ tools/               # Tool definitions
│  ├─ application/
│  │  ├─ app_state.rs         # DI container (16+ repos)
│  │  ├─ *_service.rs         # qa, review, supervisor, ideation, dependency, priority, apply,
│  │  │                       # orchestrator (--resume), execution_chat (Ph15B), task_context (Ph17)
│  │  ├─ permission_state.rs  # UI tool approval bridge
│  │  └─ http_server.rs       # Axum :3847 for MCP proxy
│  ├─ commands/               # Thin Tauri IPC: task, task_context(Ph17), project, ideation,
│  │                          # context_chat(Ph15A), execution_chat(Ph15B), permission, workflow,
│  │                          # artifact, research, methodology, qa, review, execution
│  └─ infrastructure/
│     ├─ sqlite/              # sqlite_*.rs repos + migrations.rs
│     ├─ memory/              # memory_*.rs (test repos)
│     ├─ agents/claude/       # ClaudeCodeClient
│     └─ supervisor/          # Event bus impl
└─ tests/                     # state_machine_flows, repository_swapping, agentic_client_flows,
                              # supervisor_integration, qa_system_flows, review_flows, execution_control_flows
```

## Architecture: Clean/Hexagonal
```
Commands (Tauri IPC)
    ↓
Application Services
    ↓
Domain Layer (Entities, Repo Traits, State Machine) ← NO INFRA DEPS
    ↓
Infrastructure (SQLite, Memory, Claude CLI)
```

## Repository Pattern
```rust
// domain/repositories/task_repository.rs
#[async_trait]
pub trait TaskRepository: Send + Sync {
    async fn create(&self, task: Task) -> AppResult<Task>;
    async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>>;
    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<Task>>;
}
// Impls: infrastructure/sqlite/sqlite_task_repo.rs | infrastructure/memory/memory_task_repo.rs
```

## Newtype Pattern (Type-Safe IDs)
```rust
pub struct TaskId(pub String);
pub struct ProjectId(pub String);
impl TaskId { fn new()->Self{Self(Uuid::new_v4().to_string())} fn from_string(s)->Self{Self(s)} fn as_str(&self)->&str{&self.0} }
// Compile-time safety: can't pass TaskId where ProjectId expected
```

## DI via AppState
```rust
pub struct AppState {
    pub task_repo: Arc<dyn TaskRepository>,
    pub project_repo: Arc<dyn ProjectRepository>,
    pub agent_client: Arc<dyn AgenticClient>,
    // ... 16+ repos
}
impl AppState {
    fn new_production() -> AppResult<Self> {...}  // SQLite
    fn new_test() -> Self {...}                   // Memory
}
```

## ⚠️ STATE MACHINE (CRITICAL)
14 states: Backlog→Ready→Executing→ExecutionDone→QaRefining→QaTesting→QaPassed→PendingReview→Approved
Failures: Executing→Failed|Blocked | QaTesting→QaFailed→RevisionNeeded→Executing | PendingReview→RevisionNeeded→Executing

**NEVER update status directly. ALWAYS use TransitionHandler:**
```rust
// ❌ WRONG
task.internal_status = InternalStatus::Executing;
task_repo.update(&task).await?;

// ✅ CORRECT
let services = TaskServices::new(agent_spawner, event_emitter, notifier, dependency_manager, review_starter, execution_chat_service);
let context = TaskContext::new(&task_id, &project_id, services);
let mut machine = TaskStateMachine::new(context);
let mut handler = TransitionHandler::new(&mut machine);
let result = handler.handle_transition(&current_state, &TaskEvent::Schedule).await;
```

### Entry Actions (on_enter)
| State | Action |
|-------|--------|
| Ready | Spawn QA prep (if enabled) |
| Executing | **Spawn worker via ExecutionChatService.spawn_with_persistence()** |
| QaRefining | Spawn QA refiner |
| QaTesting | Spawn QA tester |
| QaPassed | Emit qa_passed |
| QaFailed | Emit qa_failed, notify user |
| PendingReview | Start AI review, spawn reviewer |
| Approved | Emit task_completed, unblock dependents |
| Failed | Emit task_failed |

### Auto-Transitions
ExecutionDone→QaRefining (QA on) | ExecutionDone→PendingReview (QA off)
QaPassed→PendingReview | RevisionNeeded→Executing (retry)

### TaskServices Dependencies
```rust
struct TaskServices {
    agent_spawner: Arc<dyn AgentSpawner>,           // Spawn agents
    event_emitter: Arc<dyn EventEmitter>,           // Tauri events
    notifier: Arc<dyn Notifier>,                    // User notifications
    dependency_manager: Arc<dyn DependencyManager>, // Task deps
    review_starter: Arc<dyn ReviewStarter>,         // AI reviews
    execution_chat_service: Arc<dyn ExecutionChatService>, // Worker exec
}
// Prod: TauriEventEmitter, AgenticClientSpawner, ClaudeExecutionChatService
// Test: LoggingNotifier, NoOpDependencyManager, NoOpReviewStarter
```

## Commands (Tauri IPC)
```rust
#[tauri::command]
pub async fn list_tasks(project_id: String, state: State<'_, AppState>) -> Result<Vec<Task>, AppError> {
    state.task_repo.get_by_project(&ProjectId::from_string(project_id)).await
}
```

### ⚠️ Param Conventions
| Type | Rust | JS |
|------|------|---|
| Direct | `context_type: String` | `{ contextType }` (Tauri converts) |
| Struct | `input: CreateInput` | `{ input: { context_type } }` (serde exact-match) |
| Struct+rename | `#[serde(rename_all="camelCase")]` | `{ input: { contextType } }` |

## Error Handling
```rust
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")] Database(String),
    #[error("Task not found: {0}")] TaskNotFound(String),
    #[error("Invalid transition: {from} -> {to}")] InvalidTransition{from:String,to:String},
    #[error("Validation error: {0}")] Validation(String),
    #[error("Agent error: {0}")] Agent(String),
    #[error("Not found: {0}")] NotFound(String),
}
pub type AppResult<T> = Result<T, AppError>;
```

## AgenticClient Trait
```rust
#[async_trait]
pub trait AgenticClient: Send + Sync {
    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<AgentHandle>;
    async fn stop_agent(&self, handle: &AgentHandle) -> AgentResult<()>;
    async fn wait_for_completion(&self, handle: &AgentHandle) -> AgentResult<AgentOutput>;
    async fn send_prompt(&self, handle: &AgentHandle, prompt: &str) -> AgentResult<AgentResponse>;
    fn stream_response(&self, handle: &AgentHandle, prompt: &str) -> Pin<Box<dyn Stream<Item=AgentResult<ResponseChunk>>+Send>>;
    fn capabilities(&self) -> &ClientCapabilities;
    async fn is_available(&self) -> AgentResult<bool>;
}
// Impls: ClaudeCodeClient (prod), MockAgenticClient (test)
// Roles: Worker, Reviewer, Supervisor, QA, Orchestrator, chat-task, chat-project
```

## Context-Aware Chat (Ph15A/15B)
```
Frontend → Tauri IPC → Backend
  ├→ HTTP :3847 → MCP Server (TS proxy) → RalphX logic via HTTP
  └→ Claude CLI (--agent, --resume for continuation)
       └→ RALPHX_AGENT_TYPE env → MCP returns scoped tools
```

### Tool Scoping
| Agent | MCP Tools |
|-------|-----------|
| orchestrator-ideation | create/update/delete_task_proposal, add_proposal_dependency, *_plan_artifact |
| chat-task | update_task, add_task_note, get_task_details |
| chat-project | suggest_task, list_tasks |
| reviewer | complete_review |
| worker | get_task_context, get_artifact*, search_project_artifacts (Ph17) |
| supervisor/qa-* | None |

### Session Management
- RalphX Context ID: our IDs (ideation session, task, project)
- Claude Session ID: for `--resume` flag, stored in `chat_conversations.claude_session_id`

### Execution Chat (Ph15B)
```rust
// ExecutionChatService
spawn_with_persistence(agent, task_id)  // Creates conversation, spawns, persists
persist_stream_event(conv_id, event)    // Saves chunks/tool_calls
complete_execution(conv_id, session_id) // Processes queue via --resume

// ExecutionMessageQueue (in-memory, per-task)
queue(task_id, message)
pop(task_id) → Option<Message>
get_queued(task_id) → Vec<Message>
clear(task_id)
```
Events: `execution:chunk|tool_call|run_completed` (ChatPanel) | `agent:message` (Activity Stream)

### Permission Bridge
1. Agent calls `permission_request` MCP tool
2. MCP POSTs `/api/permission/request` → returns request_id
3. Backend emits `permission:request` → PermissionDialog
4. MCP long-polls `/api/permission/await/:id` (5min timeout)
5. User Allow/Deny → `resolve_permission_request` command
6. Backend signals MCP → returns decision to Claude

## Ideation Plans (Ph16)
```rust
pub enum IdeationPlanMode { Required, Optional, Parallel }
pub struct IdeationSettings { plan_mode, require_plan_approval, suggest_plans_for_complex, auto_link_to_session_plan }
// Single-row pattern in ideation_settings table

// Data model additions
IdeationSession { plan_artifact_id: Option<ArtifactId> }
TaskProposal { plan_artifact_id, plan_version_at_creation }
Task { source_proposal_id, plan_artifact_id }
```

### HTTP Endpoints (:3847)
POST /api/create_plan_artifact | POST /api/update_plan_artifact | GET /api/get_plan_artifact/:id
POST /api/link_proposals_to_plan | GET /api/get_session_plan/:id
GET /api/get_ideation_settings | POST /api/update_ideation_settings

### Proactive Sync
Flow `plan_updated_sync`: artifact_updated(Specification) → find_linked_proposals → emit `plan:proposals_may_need_update`

## Worker Artifact Context (Ph17)
```rust
pub struct TaskContext { task, source_proposal: Option<TaskProposalSummary>, plan_artifact: Option<ArtifactSummary>, related_artifacts: Vec<ArtifactSummary>, context_hints: Vec<String> }
pub struct ArtifactSummary { id, title, artifact_type, current_version, content_preview } // 500-char

// TaskContextService aggregates from task_repo, proposal_repo, artifact_repo
```

### HTTP Endpoints (:3847)
GET /api/task_context/:id | GET /api/artifact/:id | GET /api/artifact/:id/version/:v
GET /api/artifact/:id/related | POST /api/artifacts/search

### MCP Tools (worker)
get_task_context(task_id) → TaskContext
get_artifact(artifact_id) → Artifact
get_artifact_version(artifact_id, version) → Artifact
get_related_artifacts(artifact_id) → ArtifactRelation[]
search_project_artifacts(project_id, query, types?) → ArtifactSummary[]

### Worker Instructions
1. get_task_context first | 2. get_artifact(planArtifact) if present | 3. get_related_artifacts optional | 4. implement

## Database (SQLite)
`ralphx.db` (dev) | app data dir (prod)
Migrations: `infrastructure/sqlite/migrations.rs`, auto-run on startup, version in `schema_version`

### Key Tables
tasks, projects, status_transitions, task_qa, reviews, ideation_sessions, task_proposals,
proposal_dependencies, chat_messages, chat_conversations(Ph15), agent_runs(Ph15),
ideation_settings(Ph16, single-row), task_dependencies, workflows, artifacts, artifact_buckets,
research_processes, methodologies

## Build & Run
```bash
npm run tauri dev   # from root
cargo build|run     # from src-tauri
npm run tauri build # prod
```

## Linting (ALWAYS before commit)
```bash
cargo clippy --all-targets --all-features -- -D warnings  # REQUIRED
cargo fmt --check  # verify
cargo fmt          # auto-format
cargo check        # type check
```
Allowed clippy: derivable_impls, redundant_closure, too_many_arguments, type_complexity,
unnecessary_literal_unwrap, bool_comparison, while_let_loop, useless_vec, let_and_return,
unwrap_or_default, unnecessary_map_or

## Testing
```bash
cargo test
cargo test -- --nocapture
cargo test test_name
cargo test --test state_machine_flows
```
TDD mandatory | Use in-memory repos for unit tests | Use tempfile for integration tests

## Conventions
- Types: PascalCase | Functions: snake_case | Files: snake_case | Modules: snake_case
- Enums: `#[serde(rename_all="snake_case")]` | JSON: snake_case | Dates: RFC3339
- All repos: async | Use `#[async_trait]` | Return `AppResult<T>` | `?` for propagation
