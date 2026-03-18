> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# src-tauri/CLAUDE.md — Backend

Quality standards: `@../.claude/rules/code-quality-standards.md` | Rust API safety: `@../.claude/rules/rust-stable-apis.md`

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

### Dual AppState (CRITICAL)
`lib.rs` creates TWO `AppState` instances (Tauri commands + HTTP/MCP server) with separate DB connections. Any `Arc<T>` coordinating between them MUST be cloned in `lib.rs:200-208`. ❌ Relying on `new_production()` defaults.

| Shared State | What Breaks If Not Shared |
|---|---|
| `question_state` | MCP questions never reach Tauri UI |
| `permission_state` | Permission prompts never shown |
| `message_queue` | Messages lost between IPC/HTTP |
| `interactive_process_registry` | Teammate→lead nudge fails |

## Patterns

### Repository Pattern
Trait in `domain/repositories/` → impls: `sqlite_*_repo.rs` | `memory_*_repo.rs`. All async with `#[async_trait]`.

### Newtype IDs
`pub struct TaskId(pub String)` — compile-time safety, can't pass `TaskId` where `ProjectId` expected.

### DbConnection (NON-NEGOTIABLE)
All SQLite repos MUST use `db.run(|conn| { ... })` / `db.query_optional(|conn| { ... })`. ❌ `conn.lock().await`. See `db_connection.rs`.

### DI via AppState
`AppState` holds `Arc<dyn XRepository>` for all repos. `new_production()` → SQLite | `new_test()` → Memory.

### Error Handling
`AppError` enum with domain-specific variants + `AppResult<T>`. ❌ Generic string errors. ❌ `error == "some string"` — use `matches!(err, MyError::Variant)`. External strings → named `pub(crate) const` (e.g., `AGENT_ERROR_PREFIX`).

## Rules

### State Machine (CRITICAL)
Refs: task-state-machine.md (24 states) | task-git-branching.md (git/merge) | task-execution-agents.md (agents)
❌ `task.internal_status = X` | ✅ `handler.handle_transition(&state, &TaskEvent::Schedule).await`
Auto-transitions: QaPassed→PendingReview | PendingReview→Reviewing | RevisionNeeded→ReExecuting | Approved→PendingMerge
API layer: see api-layer.md

### Command Handlers (THIN)
5-10 lines max — extract, delegate to service, return. ❌ Business logic in commands.

### Permission Bridge Flow
Agent → `permission_request` MCP → POST `/api/permission/request` → backend emits `permission:request` → UI dialog → user Allow/Deny → `resolve_permission_request` → MCP long-poll returns decision

### Test File Separation (NON-NEGOTIABLE)
❌ `#[cfg(test)] mod` or `#[path = "..."]` in production files. Tests → dedicated `*_tests.rs` importing from `crate::`.

### Conventions
Types: PascalCase | Functions/files: snake_case | Enums: `#[serde(rename_all="snake_case")]` | Tauri inputs: `#[serde(rename_all = "camelCase")]` | JSON: snake_case | Dates: RFC3339

### Architectural Patterns
New pattern → add one-liner here. Pattern name + rule only.

| Pattern | Rule |
|---|---|
| ExecutionState Propagation | `Arc<ExecutionState>` → `TaskTransitionService::new()` + `AgenticClientSpawner::with_execution_state()` |
| Agent MCP Tool Allowlist | Three-layer system — see `agent-mcp-tools.md`. Source of truth: `agent_config.rs` (`AGENT_CONFIGS`) |
| Git Modes & Merge | Two modes (Local/Worktree), two-level branches (plan→task) — see task-git-branching.md |
| PreMergeCleanup | Kill agents + kill_worktree_processes BEFORE git worktree ops (TOCTOU race prevention) |
| MergeDeadline | `attempt_programmatic_merge` wraps cleanup + strategy in bounded deadline (`attempt_merge_deadline_secs`) |
| No Inline Timeout Consts | All durations → `runtime_config` + `ralphx.yaml`, never Rust `const` |
| Rust test runner split | Use targeted `cargo test` for pinpoint Rust validation and doctests; use `cargo nextest run` for broad Rust lib runs; fixture rules and commands live in `.claude/rules/rust-test-execution.md` |
| Oversized lib suite split | Move massive orchestration/state-machine/worktree suites out of `src/**` lib tests into `src-tauri/tests/*.rs` integration binaries, and expose only the minimum internal-facing API needed for them |
| HTTP handler suite split | Move large handler sidecar suites to `src-tauri/tests/*.rs`; import via `ralphx_lib::http_server::{handlers,types}` and use `AppState::new_sqlite_test()` only for SQLite-backed handler cases |
| Integration helper visibility | When a moved integration suite needs private handler/helpers, expose the minimum surface as `#[doc(hidden)] pub` instead of keeping `#[cfg(test)]` sidecar-only access |
| Tokio spawn | `tokio::spawn` → async fn ONLY. Sync code → `std::thread::spawn` \| `tauri::async_runtime::spawn`. See `.claude/rules/tokio-runtime-safety.md` |
| Rust std API stability | Avoid unstable std APIs in production code (e.g., `is_multiple_of`) — use stable equivalents (e.g., `%`). See `.claude/rules/rust-stable-apis.md` |

## Code Quality
Multi-stream workflow: `.claude/rules/stream-*.md` (features/refactor/polish). File limits + migration rules: `.claude/rules/code-quality-standards.md`.
**500 lines max** (refactor@400). Zero warnings policy — see root CLAUDE.md #8. Public API → doc `/// # Errors` section.

## Database
`ralphx.db` (dev) | Migrations: `infrastructure/sqlite/migrations/` | System: `.claude/rules/code-quality-standards.md`
New migration: `vN_description.rs` + register in `MIGRATIONS` + bump `SCHEMA_VERSION` | Use `IF NOT EXISTS` | `helpers::add_column_if_not_exists()`

## Commands
❌ `cargo check` (hangs) | ❌ full broad `cargo test` | ❌ `--nocapture`
```bash
cargo build                                                              # build
cargo test --manifest-path src-tauri/Cargo.toml <filter> --lib           # pinpoint lib tests
cargo test --manifest-path src-tauri/Cargo.toml --test <target>          # targeted integration tests
cargo nextest run --manifest-path src-tauri/Cargo.toml --lib             # broad Rust lib run
cargo clippy --all-targets --all-features -- -D warnings                 # lint
```
Selective Rust test commands + SQLite test fixture rules → `.claude/rules/rust-test-execution.md`

## Real Integration Tests
Pattern: `tempfile::TempDir` + git CLI → `Memory*Repository` → `TaskServices::new_mock()` | `MockChatService` → `TransitionHandler` → assert state + git.
Shared helpers: `transition_handler/tests/helpers.rs` — `setup_real_git_repo()`, `PendingMergeSetup`, `RealGitRepo`.

| File | Tests | Real | Mocked |
|------|-------|------|--------|
| `tests/merge_system_hardening.rs` | 23 | git, MemoryTaskRepo | — |
| `tests/deferred_main_merge_integration.rs` | 8 | git, MemoryTaskRepo | — |
| `transition_handler/tests/real_git_integration.rs` | 8 | git, merge dispatch | MockChatService |
| `transition_handler/tests/orchestration_chain_tests.rs` | 3 | git, full state machine | MockChatService |
| `transition_handler/tests/plan_update_from_main.rs` | 7 | git, pure fn | — |
| `transition_handler/tests/source_update_from_target.rs` | 7 | git, pure fn | — |
| `transition_handler/tests/rc12_rc13_stale_worktree.rs` | 3 | git worktrees | — |
| `transition_handler/tests/merge_cleanup.rs` | 7 | transitions | TaskServices::new_mock() |

## Allowed Clippy Lints
derivable_impls, redundant_closure, too_many_arguments, type_complexity,
unnecessary_literal_unwrap, bool_comparison, useless_vec, let_and_return
