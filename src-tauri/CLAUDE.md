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
    // Archive methods (Ph18)
    async fn archive(&self, task_id: &TaskId) -> AppResult<Task>;
    async fn restore(&self, task_id: &TaskId) -> AppResult<Task>;
    async fn get_archived_count(&self, project_id: &ProjectId) -> AppResult<u32>;
    async fn get_by_project_filtered(&self, project_id: &ProjectId, include_archived: bool) -> AppResult<Vec<Task>>;
    // Pagination & Search (Ph18)
    async fn list_paginated(&self, project_id: &ProjectId, offset: u32, limit: u32, include_archived: bool) -> AppResult<Vec<Task>>;
    async fn count_tasks(&self, project_id: &ProjectId, include_archived: bool) -> AppResult<u32>;
    async fn search(&self, project_id: &ProjectId, query: &str, include_archived: bool) -> AppResult<Vec<Task>>;
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

// Archive System (Ph18)
archive_task(task_id) → Task (emits task:archived event)
restore_task(task_id) → Task (emits task:restored event)
permanently_delete_task(task_id) → () (emits task:deleted event, only if archived)
get_archived_count(project_id) → u32

// Search & Pagination (Ph18)
search_tasks(project_id, query, include_archived?) → Vec<Task>  // Server-side search
list_tasks(project_id, status?, offset?, limit?, include_archived?) → TaskListResponse
get_valid_transitions(task_id) → Vec<StatusTransition>  // Query state machine

// Tauri Events (Ph18)
task:archived → { task_id, project_id }
task:restored → { task_id, project_id }
task:deleted → { task_id, project_id }
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
| worker | get_task_context, get_artifact*, search_project_artifacts (Ph17), get_task_steps, start_step, complete_step, skip_step, fail_step, add_step, get_step_progress (Ph19) |
| supervisor/qa-* | None |

### Task Steps (Ph19)
Worker agents track deterministic progress using task steps:
```bash
# Worker workflow
1. get_task_steps(task_id)         # Fetch all steps
2. start_step(step_id)              # Mark step in_progress
3. [work on step]                   # Implement
4. complete_step(step_id, note?)    # Mark completed with optional note
# OR skip_step(step_id, reason)     # Skip if not needed
# OR fail_step(step_id, error)      # Mark failed
```

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

## Task Execution Experience (Ph19)
```rust
// Entities (domain/entities/)
pub struct TaskStep {
    id: TaskStepId,
    task_id: TaskId,
    title: String,
    description: Option<String>,
    status: TaskStepStatus,
    sort_order: u32,
    depends_on: Option<TaskStepId>,
    created_by: String,
    completion_note: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

pub enum TaskStepStatus {
    Pending, InProgress, Completed, Skipped, Failed, Cancelled
}

pub struct StepProgressSummary {
    task_id: String,
    total: u32, completed: u32, in_progress: u32, pending: u32, skipped: u32, failed: u32,
    current_step: Option<TaskStep>,   // First InProgress
    next_step: Option<TaskStep>,       // First Pending
    percent_complete: f32,             // (completed + skipped) / total * 100
}

impl TaskStep {
    fn new(task_id: TaskId, title: String, sort_order: u32, created_by: String) -> Self;
    fn can_start(&self) -> bool;      // status == Pending
    fn is_terminal(&self) -> bool;    // Completed|Skipped|Failed|Cancelled
}

impl StepProgressSummary {
    fn from_steps(task_id: &TaskId, steps: &[TaskStep]) -> Self;
}

// Repository (domain/repositories/task_step_repository.rs)
#[async_trait]
pub trait TaskStepRepository: Send + Sync {
    async fn create(&self, step: TaskStep) -> AppResult<TaskStep>;
    async fn get_by_id(&self, id: &TaskStepId) -> AppResult<Option<TaskStep>>;
    async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<TaskStep>>;
    async fn get_by_task_and_status(&self, task_id: &TaskId, status: TaskStepStatus) -> AppResult<Vec<TaskStep>>;
    async fn update(&self, step: &TaskStep) -> AppResult<()>;
    async fn delete(&self, id: &TaskStepId) -> AppResult<()>;
    async fn delete_by_task(&self, task_id: &TaskId) -> AppResult<()>;
    async fn count_by_status(&self, task_id: &TaskId) -> AppResult<HashMap<TaskStepStatus, u32>>;
    async fn bulk_create(&self, steps: Vec<TaskStep>) -> AppResult<Vec<TaskStep>>;
    async fn reorder(&self, task_id: &TaskId, step_ids: Vec<TaskStepId>) -> AppResult<()>;
}
// Impls: SqliteTaskStepRepository, MemoryTaskStepRepository

// Commands (commands/task_step_commands.rs)
create_task_step(task_id, title, description?, sort_order?) → TaskStep
get_task_steps(task_id) → Vec<TaskStep>
update_task_step(step_id, title?, description?, sort_order?) → TaskStep
delete_task_step(step_id) → ()
reorder_task_steps(task_id, step_ids: Vec<String>) → Vec<TaskStep>
get_step_progress(task_id) → StepProgressSummary
start_step(step_id) → TaskStep         // Pending → InProgress, emits step:updated
complete_step(step_id, note?) → TaskStep  // InProgress → Completed, emits step:updated
skip_step(step_id, reason) → TaskStep   // Pending|InProgress → Skipped, emits step:updated
fail_step(step_id, error) → TaskStep    // InProgress → Failed, emits step:updated

// HTTP Endpoints (:3847 for MCP)
GET /api/task_steps/:task_id → Vec<StepResponse>
POST /api/start_step { step_id } → StepResponse
POST /api/complete_step { step_id, note? } → StepResponse
POST /api/skip_step { step_id, reason } → StepResponse
POST /api/fail_step { step_id, error } → StepResponse
POST /api/add_step { task_id, title, description?, after_step_id? } → StepResponse
GET /api/step_progress/:task_id → StepProgressSummary

// Integration
create_task(steps?: Vec<String>) — creates TaskSteps with created_by="user"
task created from proposal — imports steps from proposal.steps with created_by="proposal"
get_task_context — includes steps and step_progress in TaskContext

// Events (Tauri)
step:created → { task_id, step_id }
step:updated → { task_id, step_id }
step:deleted → { task_id, step_id }
steps:reordered → { task_id, step_ids }
```

## Database (SQLite)
`ralphx.db` (dev) | app data dir (prod)
Migrations: `infrastructure/sqlite/migrations.rs`, auto-run on startup, version in `schema_version`

### Key Tables
tasks (Ph18: archived_at), projects, status_transitions, task_qa, reviews, ideation_sessions,
task_proposals, proposal_dependencies, chat_messages, chat_conversations(Ph15), agent_runs(Ph15),
ideation_settings(Ph16, single-row), task_dependencies, workflows, artifacts, artifact_buckets,
research_processes, methodologies, task_steps (Ph19)

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

## Code Quality Rules

### File Size (STRICT)
**Maximum 500 lines per file** — no exceptions for new code, refactor existing violations.

| Threshold | Action |
|-----------|--------|
| 400 lines | Plan extraction before hitting limit |
| 500 lines | MUST refactor before merge |

### When to Extract
| Condition | Action |
|-----------|--------|
| Helper functions > 100 lines total | Extract to `{module}_helpers.rs` |
| Multiple type definitions (>5 structs/enums) | Extract to `{module}_types.rs` |
| Enum with >10 variants + impl blocks | Separate file |
| Service method > 50 lines | Extract to helper function |
| Validation logic > 30 lines | Extract to `{module}_validation.rs` |

**Example: Splitting a large service**
```
application/
├── chat_service.rs           # 400 lines - main impl, public API
├── chat_service_helpers.rs   # Parsing, formatting, internal logic
└── chat_service_types.rs     # Internal DTOs (not re-exported)
```

### Command Handlers (THIN)
Commands in `commands/*.rs` must be **thin IPC wrappers** — extract, delegate, return:
```rust
// ✅ CORRECT: 5-10 lines max
#[tauri::command]
async fn create_task(state: State<'_, AppState>, input: CreateTaskInput) -> Result<Task, AppError> {
    state.task_service.create(input).await
}

// ❌ WRONG: Business logic in command handler
#[tauri::command]
async fn create_task(...) -> Result<Task, AppError> {
    // 50+ lines of validation, transformation, side effects
    // This belongs in a service!
}
```

### Entity Organization
Split entities when they have multiple concerns:
```
domain/entities/
├── task.rs              # Core Task struct + TaskBuilder
├── task_status.rs       # InternalStatus enum + transitions
├── task_types.rs        # Priority, Category, supporting types
└── task_validation.rs   # Validation helpers
```

### Documentation (MANDATORY for public API)
```rust
/// Brief description (one line)
///
/// Detailed explanation if non-obvious behavior exists.
///
/// # Errors
/// - `AppError::NotFound` if task doesn't exist
/// - `AppError::InvalidTransition` if status change not allowed
///
/// # Panics
/// Panics if `project_id` is empty (document only if applicable).
pub async fn transition_task(&self, task_id: &TaskId, event: TaskEvent) -> AppResult<Task>
```

### Error Handling
```rust
// ✅ PREFER: Domain-specific error variants
#[derive(Error, Debug)]
pub enum IdeationError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Circular dependency: {path:?}")]
    CircularDependency { path: Vec<TaskProposalId> },
}

// ❌ AVOID: Generic string errors scattered throughout
Err(AppError::Validation("something went wrong".to_string()))
```

### Files Needing Refactoring (Priority)
| File | Lines | Refactor Strategy |
|------|-------|-------------------|
| `ideation_commands.rs` | 2,580 | Extract validation → `ideation_validation.rs`, split by operation |
| `chat_service.rs` | 2,039 | Extract parsing → `chat_parsing.rs`, stream handling → `chat_stream.rs` |
| `ideation.rs` | 3,979 | Split: core entity, status enum, settings, builder |
| `task_commands.rs` | 1,865 | Move business logic to services, thin down handlers |
| `apply_service.rs` | 1,833 | Extract cycle detection, dependency copying |
| `http_server.rs` | 1,793 | Extract route handlers to `handlers/` submodule |

### Pre-Commit Quality Check
```bash
# Add to your workflow before committing:
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings

# Check no file exceeds 500 lines (warning)
find src -name "*.rs" -exec wc -l {} + | awk '$1 > 500 {print "⚠️  OVER 500:", $2, "("$1" lines)"}'
```
