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
| `cargo test` name filters are single-filter only | `cargo test <TESTNAME>` / `cargo test --lib <FILTER>` accepts one substring filter; do not append multiple test names and expect Cargo/libtest to combine them |
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

## Filter Rules

| Need | Use |
|---|---|
| One unit-test/module substring | `cargo test --manifest-path src-tauri/Cargo.toml <filter> --lib` |
| Multiple integration targets in one run | `cargo test --manifest-path src-tauri/Cargo.toml --test review_flows --test execution_control_flows` |
| Multiple unrelated unit-test filters | Run separate `cargo test ... --lib` commands sequentially |
| Filter misses unexpectedly | `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --list | rg "<repo_or_module>"` → then rerun with the real module-path prefix |
| Parallel verification | ❌ do not start multiple Cargo test jobs against the same target dir; they block on `.cargo-lock` and add noise instead of speed |

Example:

```bash
cargo test --manifest-path src-tauri/Cargo.toml sqlite_chat_conversation_repo_tests --lib
cargo test --manifest-path src-tauri/Cargo.toml sqlite_memory_entry_repo_tests --lib
```

Module-path example:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib -- --list | rg "sqlite_question_repo"
cargo test --manifest-path src-tauri/Cargo.toml 'infrastructure::sqlite::sqlite_question_repo::tests::' --lib
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
