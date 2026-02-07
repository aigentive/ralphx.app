# src-tauri/CLAUDE.md — Backend

Quality standards: @../.claude/rules/code-quality-standards.md

## Stack
Rust 2021 | Tauri 2.0 | rusqlite 0.32 | statig 0.3 (async state machine)
tokio 1.x | serde 1.x | chrono 0.4 | thiserror 1.x | async-trait 0.1 | tracing 0.1

## Key Directories
```
src-tauri/src/
├─ domain/
│  ├─ entities/        # Task, Project, InternalStatus, etc.
│  ├─ repositories/    # Traits (interfaces)
│  ├─ state_machine/   # machine.rs, transition_handler.rs
│  └─ agents/          # AgenticClient trait
├─ application/
│  ├─ app_state.rs     # DI container
│  ├─ *_service.rs     # Business logic
│  └─ http_server.rs   # Axum :3847 for MCP
├─ commands/           # Thin Tauri IPC wrappers
└─ infrastructure/
   ├─ sqlite/          # Repo implementations
   └─ memory/          # Test repos
```

## Architecture: Clean/Hexagonal
```
Commands (Tauri IPC) → Application Services → Domain Layer ← NO INFRA DEPS → Infrastructure
```

## Patterns

### Repository Pattern
```rust
#[async_trait]
pub trait TaskRepository: Send + Sync {
    async fn create(&self, task: Task) -> AppResult<Task>;
    async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>>;
}
// Impls: sqlite_*_repo.rs | memory_*_repo.rs
```

### Newtype IDs (Type Safety)
```rust
pub struct TaskId(pub String);
pub struct ProjectId(pub String);
// Compile-time safety: can't pass TaskId where ProjectId expected
```

### DI via AppState
```rust
pub struct AppState {
    pub task_repo: Arc<dyn TaskRepository>,
    pub project_repo: Arc<dyn ProjectRepository>,
    // ... repos
}
// new_production() → SQLite | new_test() → Memory
```

### Error Handling
```rust
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Task not found: {0}")] TaskNotFound(String),
    #[error("Invalid transition: {from} -> {to}")] InvalidTransition{from:String,to:String},
}
pub type AppResult<T> = Result<T, AppError>;
// Prefer domain-specific error variants over generic strings
```

## Rules

### State Machine (CRITICAL)
14 states: Backlog→Ready→Executing→ExecutionDone→QaRefining→QaTesting→QaPassed→PendingReview→Approved

**NEVER update status directly. ALWAYS use TransitionHandler:**
```rust
// ❌ task.internal_status = InternalStatus::Executing;
// ✅ handler.handle_transition(&current_state, &TaskEvent::Schedule).await;
```

### Auto-Transitions
ExecutionDone→QaRefining (QA on) | ExecutionDone→PendingReview (QA off)
QaPassed→PendingReview | RevisionNeeded→Executing (retry)

### API Layer Patterns
See @.claude/rules/api-layer.md for param conventions, response serialization, and cross-layer patterns.

### Command Handlers (THIN)
Commands must be 5-10 lines max — extract, delegate, return:
```rust
#[tauri::command]
async fn create_task(state: State<'_, AppState>, input: CreateTaskInput) -> Result<Task, AppError> {
    state.task_service.create(input).await  // Business logic in service
}
```

### Permission Bridge Flow
1. Agent calls `permission_request` MCP tool
2. MCP POSTs `/api/permission/request` → returns request_id
3. Backend emits `permission:request` → PermissionDialog
4. MCP long-polls `/api/permission/await/:id`
5. User Allow/Deny → `resolve_permission_request` command
6. Backend signals MCP → returns decision to Claude

### Conventions
- Types: PascalCase | Functions: snake_case | Files: snake_case
- Enums: `#[serde(rename_all="snake_case")]`
- JSON: snake_case | Dates: RFC3339
- All repos: async with `#[async_trait]`

### Document Patterns Inline
When introducing a new architectural pattern, add a one-liner here. Pattern name + rule only.
Example: "ServiceExtraction Pattern: business logic in *_service.rs, commands just delegate"

**ExecutionState Propagation:** `Arc<ExecutionState>` must be passed to `TaskTransitionService::new()` and `AgenticClientSpawner::with_execution_state()` for spawn gating and running count tracking.

**Agent MCP Tool Allowlist:** Three-layer system — see `@../.claude/rules/agent-mcp-tools.md`. Rust source of truth: `infrastructure/agents/claude/agent_config.rs` (`AGENT_CONFIGS`).

## Code Quality

### Multi-Stream Workflow
Quality work is now split into dedicated streams. See `.claude/rules/stream-*.md`:
- **features**: PRD tasks + P0 gap fixes
- **refactor**: P1 large file splits (>500 LOC)
- **polish**: P2/P3 cleanup, lint, type fixes

**Targets:** clippy, error handling (domain variants), naming, dead code, helpers

### File Size Limits
**See:** `.claude/rules/code-quality-standards.md` (single source of truth)

Quick reference: **500 lines max** — refactor at 400 lines.

### Documentation (public API)
```rust
/// Brief description.
///
/// # Errors
/// - `AppError::NotFound` if task doesn't exist
pub async fn transition_task(&self, task_id: &TaskId, event: TaskEvent) -> AppResult<Task>
```

## Task Management (MANDATORY)
Use TaskCreate/TaskUpdate/TaskList for complex work. See `.claude/rules/task-management.md`

## Database
`ralphx.db` (dev) | Migrations in `infrastructure/sqlite/migrations/`

**Migration system:** See `.claude/rules/code-quality-standards.md` → "Database Migrations"

Quick reference:
- Add new migration: create `vN_description.rs`, register in `MIGRATIONS` array, bump `SCHEMA_VERSION`
- Use `IF NOT EXISTS` for idempotency
- Use `helpers::add_column_if_not_exists()` for ALTER TABLE

## Commands
```bash
cargo build                    # build
cargo test                     # test
cargo test -- --nocapture      # verbose
cargo fmt                      # format
cargo clippy --all-targets --all-features -- -D warnings  # lint (REQUIRED before commit)
```

## Allowed Clippy Lints
derivable_impls, redundant_closure, too_many_arguments, type_complexity,
unnecessary_literal_unwrap, bool_comparison, useless_vec, let_and_return
