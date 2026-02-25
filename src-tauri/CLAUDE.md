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

### Dual AppState — Shared In-Memory State (CRITICAL)

`lib.rs` creates TWO `AppState` instances: one for Tauri commands, one for the HTTP/MCP server. They have **separate DB connections** but MUST share in-memory state. When adding new in-memory state to `AppState`, you MUST share it in `lib.rs:200-208` or the HTTP server and Tauri commands will operate on different data.

| Shared State | Purpose | What Breaks If Not Shared |
|---|---|---|
| `question_state` | MCP question/answer flow | MCP questions never reach Tauri UI |
| `permission_state` | MCP permission requests | Permission prompts never shown |
| `message_queue` | Queued chat messages | Messages lost between IPC/HTTP |
| `interactive_process_registry` | Lead stdin handles for nudging | Teammate→lead nudge fails (different HashMap) |

**Rule:** Any `Arc<T>` field on `AppState` that coordinates between Tauri commands and HTTP handlers MUST be cloned from the primary AppState to the HTTP AppState in `lib.rs`. ❌ Relying on `new_production()` defaults — each call creates independent instances.

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

**No fragile string comparisons:** NEVER use `error == "some string"` or `.contains("error text")` to detect error types. Use `matches!(err, MyError::Variant)` or enum discriminants. For external/uncontrolled strings (CLI stderr), extract match strings to named `pub(crate) const` with doc comments noting the source. Example: `AGENT_ERROR_PREFIX` in `chat_service/mod.rs`.

## Rules

### State Machine (CRITICAL)

**Full reference:** See task-state-machine.md (24 states, transitions, side effects), task-git-branching.md (git modes, merge workflow), task-execution-agents.md (agent configs).

**NEVER update status directly. ALWAYS use TransitionHandler:**
```rust
// ❌ task.internal_status = InternalStatus::Executing;
// ✅ handler.handle_transition(&current_state, &TaskEvent::Schedule).await;
```

### Auto-Transitions
QaPassed→PendingReview | PendingReview→Reviewing | RevisionNeeded→ReExecuting | Approved→PendingMerge

### API Layer Patterns
See api-layer.md for param conventions, response serialization, and cross-layer patterns.

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
- **Tauri command input structs:** `#[serde(rename_all = "camelCase")]` — frontend `invoke()` callers must pass camelCase field names
- JSON: snake_case | Dates: RFC3339
- All repos: async with `#[async_trait]`

### Document Patterns Inline
When introducing a new architectural pattern, add a one-liner here. Pattern name + rule only.
Example: "ServiceExtraction Pattern: business logic in *_service.rs, commands just delegate"

**ExecutionState Propagation:** `Arc<ExecutionState>` must be passed to `TaskTransitionService::new()` and `AgenticClientSpawner::with_execution_state()` for spawn gating and running count tracking.

**Agent MCP Tool Allowlist:** Three-layer system — see `@../.claude/rules/agent-mcp-tools.md`. Rust source of truth: `infrastructure/agents/claude/agent_config.rs` (`AGENT_CONFIGS`).

**Git Modes & Merge Workflow:** Two modes (Local/Worktree), two-level branch hierarchy (plan→task), programmatic+agent merge — see task-git-branching.md.

**PreMergeCleanup Ordering:** Always kill agents + kill_worktree_processes before git worktree ops to prevent TOCTOU race where agent holds worktree files.

**MergeDeadline Pattern:** `attempt_programmatic_merge` wraps cleanup + strategy dispatch in a bounded configurable deadline (`attempt_merge_deadline_secs` in reconciliation config).

**No Inline Timeout Consts:** All timeout/delay/duration values are operational knobs — always add to runtime_config + ralphx.yaml, never as Rust `const`. Follow the `attempt_merge_deadline_secs` pattern.

## Code Quality

### Multi-Stream Workflow
Quality work is now split into dedicated streams. See `.claude/rules/stream-*.md`:
- **features**: PRD tasks + P0 gap fixes
- **refactor**: P1 large file splits (>500 LOC)
- **polish**: P2/P3 cleanup, lint, type fixes

**Targets:** clippy, error handling (domain variants), naming, dead code, helpers

### Zero Warnings Policy (NON-NEGOTIABLE)
Fix ALL clippy warnings and test failures before completing work — including pre-existing ones. ❌ "It's pre-existing" is not an excuse. Run `cargo clippy --all-targets --all-features -- -D warnings` and fix everything.

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
When using Claude/automation: run **only** `cargo test --lib`; **do not run** `cargo check` or full `cargo test` (they hang). No `--nocapture`/verbose. `cargo test --lib` can take 5–8+ min; use **10 min timeout** and `tail` so the run finishes before chat/stream limits (~600s), or run a focused subset (e.g. by module).
```bash
cargo build                    # build
# Unit tests: can take 5–8+ min. From repo root use 10m timeout + tail (accommodates slow runs):
#   timeout 10m cargo test --lib --manifest-path src-tauri/Cargo.toml 2>&1 | tail -40
# Or run only tests for the area you changed (faster):
#   cd src-tauri && cargo test --lib <module_or_test_name>
cargo test --lib                # unit tests only (full run; full cargo test hangs)
# Do NOT run cargo check in automation — it hangs
cargo fmt                      # format
cargo clippy --all-targets --all-features -- -D warnings  # lint (REQUIRED before commit)
```

## Real Integration Tests (Reference)

Tests using real git repos + real DB (memory impls) + mocked agent spawns. Use these as patterns when adding new integration tests.

| File | Tests | What's Real | What's Mocked | Pattern |
|------|-------|-------------|---------------|---------|
| `tests/merge_system_hardening.rs` | 23 | git (tempdir), MemoryTaskRepo, MemoryPlanBranchRepo | No agents | Standalone helpers, `setup_test_repo()` |
| `tests/deferred_main_merge_integration.rs` | 8 | git, MemoryTaskRepo | No agents | Deferred merge retry scenarios |
| `domain/state_machine/transition_handler/tests/real_git_integration.rs` | 8 | git, MemoryTaskRepo, merge strategy dispatch | MockChatService, MockAgentSpawner | `setup_real_git_repo()` → `PendingMergeSetup` |
| `domain/state_machine/transition_handler/tests/orchestration_chain_tests.rs` | 3 | git, MemoryTaskRepo, full state machine | MockChatService (`.call_count()`) | Real git + real DB + mock agent spawns |
| `domain/state_machine/transition_handler/tests/plan_update_from_main.rs` | 7 | git, `update_plan_from_main()` directly | Nothing | Pure git function tests |
| `domain/state_machine/transition_handler/tests/source_update_from_target.rs` | 7 | git, `update_source_from_target()` directly | Nothing | Pure git function tests |
| `domain/state_machine/transition_handler/tests/rc12_rc13_stale_worktree.rs` | 3 | git worktrees, GitService | Nothing | Worktree lifecycle tests |
| `domain/state_machine/transition_handler/tests/merge_cleanup.rs` | 7 | State machine transitions | TaskServices::new_mock() | Cleanup step ordering |

**Shared helpers:** `domain/state_machine/transition_handler/tests/helpers.rs` (414 lines) — `setup_real_git_repo()`, `PendingMergeSetup`, `RealGitRepo`, all mock services.

**Pattern:** `tempfile::TempDir` + `std::process::Command` git CLI → `MemoryTaskRepository` + `MemoryProjectRepository` → `TaskServices::new_mock()` or custom `MockChatService` → `TransitionHandler::new(&mut machine)` → assert state + git.

## Allowed Clippy Lints
derivable_impls, redundant_closure, too_many_arguments, type_complexity,
unnecessary_literal_unwrap, bool_comparison, useless_vec, let_and_return
