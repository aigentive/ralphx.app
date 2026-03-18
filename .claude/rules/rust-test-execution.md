---
paths:
  - "src-tauri/**/*.rs"
  - "src-tauri/CLAUDE.md"
  - ".claude/rules/*.md"
---

# Rust Test Execution

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

## Non-Negotiables

| Rule | Detail |
|---|---|
| Run targeted Rust tests | ✅ `cargo test --manifest-path src-tauri/Cargo.toml --test <file_stem>` | ❌ full `cargo test` |
| No broad formatter runs | ❌ `cargo fmt` / broad `rustfmt` unless user explicitly asks; they can touch hundreds of files and hide the real diff |
| Keep diffs reviewable | Use `apply_patch` for code edits, then verify `git diff` / `git diff --staged` only shows intended hunks |
| Heavy SQLite tests use shared temp DB fixtures | Use `ralphx_lib::testing::SqliteTestDb` / `SqliteStateFixture` instead of rerunning migrations into fresh `:memory:` DBs |

## Selective Commands

```bash
cargo test --manifest-path src-tauri/Cargo.toml db_connection --lib
cargo test --manifest-path src-tauri/Cargo.toml --test research_integration --test workflow_integration --test artifact_integration --test repository_swapping --test methodology_integration --test gsd_integration
cargo test --manifest-path src-tauri/Cargo.toml --test state_machine_flows --test qa_system_flows
cargo test --manifest-path src-tauri/Cargo.toml --test per_project_execution_scoping
cargo test --manifest-path src-tauri/Cargo.toml --test review_flows
cargo test --manifest-path src-tauri/Cargo.toml --test execution_control_flows
```

## Shared SQLite Test Setup

| Scenario | Pattern |
|---|---|
| AppState + async SQLite repos | `SqliteStateFixture::new("suite-name", |db, state| { state.repo = Arc::new(SqliteRepo::from_shared(db.shared_conn())); })` |
| Sync `TaskStateMachineRepository` tests | `let db = SqliteTestDb::new("suite"); let conn = db.new_connection(); let repo = TaskStateMachineRepository::new(conn);` |
| Mixed async + sync repos in one suite | One `SqliteTestDb` → `db.shared_conn()` for async repos + `db.new_connection()` for sync repos |
| Fixture lifetime | Keep the fixture bound as `_db` in each test so the temp directory and DB file stay alive for the whole test |
| Raw setup SQL | Insert rows through the opened file-backed connection after fixture creation; do not rerun migrations in each helper |

## Formatter Warning

| Situation | Action |
|---|---|
| Need to change Rust code | Edit the smallest surface possible |
| Think "`cargo fmt` will be harmless" | Don’t do it here |
| Formatting is truly required | Ask first, keep it scoped, and commit it separately from logic changes |

