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
| Use `cargo-nextest` for broad Rust runs | ✅ `cargo nextest run --manifest-path src-tauri/Cargo.toml --lib --profile ci` for broad lib coverage and CI; keep `cargo test` for pinpoint filters and doctests |
| `cargo test` name filters are single-filter only | `cargo test <TESTNAME>` / `cargo test --lib <FILTER>` accepts one substring filter; do not append multiple test names and expect Cargo/libtest to combine them |
| No broad formatter runs | ❌ `cargo fmt` / broad `rustfmt` unless user explicitly asks; they can touch hundreds of files and hide the real diff |
| Keep diffs reviewable | Use `apply_patch` for code edits, then verify `git diff` / `git diff --staged` only shows intended hunks |
| Heavy SQLite tests use shared temp DB fixtures | Use `ralphx_lib::testing::SqliteTestDb` / `SqliteStateFixture` instead of rerunning migrations into fresh `:memory:` DBs |
| Don’t over-convert narrow utility tests | Pure formatting/connection tests that never run migrations can stay on lightweight `:memory:` setup or direct connection helpers |

## Standard Stack

| Layer | Standard |
|---|---|
| Test runner | `cargo test` for targeted filters and single suites; `cargo nextest run` for broad lib/test runs and CI |
| Target discovery | `cargo test --manifest-path src-tauri/Cargo.toml --lib -- --list | rg "<module>"` |
| Async SQLite repo tests | `SqliteTestDb` + repo `from_shared(db.shared_conn())` |
| AppState integration tests | `SqliteStateFixture::new(...)` |
| Sync SQLite repo tests | `SqliteTestDb` + `db.new_connection()` |
| Setup/seeding | Shared suite helpers/builders on top of `SqliteTestDb`; one migration pass per temp DB only |
| Concurrency | File-backed temp DBs for shared access; `:memory:` only for intentionally isolated narrow tests |
| Broad-run runner config | Repo config lives in `.config/nextest.toml`; keep group changes there, not in ad hoc shell flags |
| Formatter policy | No broad `cargo fmt`; if formatting is required, keep it scoped and separate |

## Scale Direction

| Topic | Direction |
|---|---|
| Shared state | Keep tests isolated and parallel-safe; avoid shared DB state except for explicitly serialized cases |
| Fixture style | Rust has no built-in fixture system here; use helper modules, suite-local `setup_*()` functions, and small builders |
| Compile vs run | Optimize both separately: narrow targets to reduce compile scope, then keep per-test runtime setup cheap |
| Large-suite runner | `cargo-nextest` is the adopted broad-runner for large-scale execution; targeted edit-loop runs still stay on `cargo test` |
| Test layers | Keep fast repo/unit suites separate from slower integration/state-machine/git suites |
| Internal support | Invest early in a thin shared test-support layer under `src-tauri/src/testing/` when setup repeats |
| CI coverage split | CI runs broad lib coverage via `cargo nextest run --lib --profile ci` and doctests via separate `cargo test --doc` |

## Selective Commands

```bash
cargo test --manifest-path src-tauri/Cargo.toml db_connection --lib
cargo test --manifest-path src-tauri/Cargo.toml --test research_integration --test workflow_integration --test artifact_integration --test repository_swapping --test methodology_integration --test gsd_integration
cargo test --manifest-path src-tauri/Cargo.toml --test state_machine_flows --test qa_system_flows
cargo test --manifest-path src-tauri/Cargo.toml --test per_project_execution_scoping
cargo test --manifest-path src-tauri/Cargo.toml --test review_flows
cargo test --manifest-path src-tauri/Cargo.toml --test execution_control_flows
cargo nextest run --manifest-path src-tauri/Cargo.toml --lib
cargo nextest run --manifest-path src-tauri/Cargo.toml --lib --profile ci
```

## Nextest Setup

| Need | Command |
|---|---|
| Repo Rust toolchain | `rust-toolchain.toml` pins Rust `1.91.0`; keep CI and local development aligned to that file |
| Activate pinned toolchain locally | `rustup toolchain install 1.91.0 && rustup override set 1.91.0` from repo root |
| Install on macOS | `brew install cargo-nextest` |
| Install from Cargo | `cargo install cargo-nextest --locked` |
| Broad local lib run | `cargo nextest run --manifest-path src-tauri/Cargo.toml --lib` |
| Broad CI-style lib run | `cargo nextest run --manifest-path src-tauri/Cargo.toml --lib --profile ci` |
| Pinpoint module/test validation | `cargo test --manifest-path src-tauri/Cargo.toml <filter> --lib` or `cargo test --manifest-path src-tauri/Cargo.toml --test <target>` |
| Doctests | `cargo test --manifest-path src-tauri/Cargo.toml --doc` |
| CI broad coverage | `cargo nextest run --manifest-path src-tauri/Cargo.toml --lib --profile ci && cargo test --manifest-path src-tauri/Cargo.toml --doc` |

## Nextest Groups

| Group | Purpose |
|---|---|
| `git-heavy` | Caps the heaviest git/worktree integration binaries at 2 threads |
| `sqlite-integration` | Caps file-backed SQLite integration binaries at 4 threads |
| `perf-serial` | Forces `plan_selector_performance` to 1 thread |
| Config source | Edit `.config/nextest.toml` rather than pasting long `-E` filters into docs or CI |

## Filter Rules

| Need | Use |
|---|---|
| One unit-test/module substring | `cargo test --manifest-path src-tauri/Cargo.toml <filter> --lib` |
| Multiple integration targets in one run | `cargo test --manifest-path src-tauri/Cargo.toml --test review_flows --test execution_control_flows` |
| Multiple unrelated unit-test filters | Run separate `cargo test ... --lib` commands sequentially |
| Fast module-path guess | Derive `folder::tree::module::tests::` from the source tree first; for `#[path = "foo_tests.rs"] mod tests;` under `foo.rs`, prefer `...::foo::tests::` |
| Sidecar `*_tests.rs` under a production module | Prefer the parent module path first: `application/review_issue_service_tests.rs` → `application::review_issue_service::tests::`, not `application::review_issue_service_tests::` |
| Legacy standalone `*_tests.rs` modules still exist | Some suites keep the file stem path (`sqlite_team_message_repo_tests`); if the parent-module guess is not obvious, use `-- --list | rg ...` immediately instead of guessing twice |
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
| Shared seed API | Prefer `db.seed_project(...)`, `db.seed_task(...)`, `db.seed_ideation_session(...)`, `db.seed_ideation_conversation()`, `db.seed_task_conversation(...)`, `db.insert_conversation(...)`, `db.insert_review_note(...)` before adding new suite-local SQL |

## Best Practices

| Rule | Detail |
|---|---|
| Default to isolated file-backed fixtures | Rust tests should stay parallel-safe; use temp file DBs instead of shared globals |
| One helper per suite shape | Extract `setup_*()` returning fixture + repo + seeded IDs when 2+ tests share setup |
| Builders over repeated SQL | Promote repeated inserts into `seed_project(...)`, `seed_task(...)`, `seed_review_note(...)` helpers instead of cloning raw SQL blocks |
| Helpers take `&SqliteTestDb` when possible | Keep seeding logic reusable across async repos, sync repos, and mixed suites |
| Use `db.with_connection(...)` for direct SQL checks | If a suite needs repo calls plus raw SQL assertions, keep the repo on `db.new_connection()` and use `db.with_connection(...)` for direct setup or verification |
| Use `db.shared_conn()` for shared-connection variants | For `from_shared(...)` coverage, pass `db.shared_conn()` instead of building a second ad hoc `Arc<Mutex<Connection>>` |
| Shared helper fixtures keep `_db` alive | If a suite returns `(shared_conn, repo)`-style helpers, include the `SqliteTestDb` in that helper result so temp DB cleanup never races the shared async connection |
| Extend `SqliteTestDb` when patterns repeat | If the same row graph appears across suites, add a shared helper in `src-tauri/src/testing/sqlite_test_db.rs` instead of copying another local setup block |
| Service suites keep fixture ownership explicit | Prefer a small `TestContext { _db, service, ids... }` so the temp DB lifetime is obvious and setup is reused across tests |
| Keep suite-local seed helpers when the shape is narrow | If only one suite needs a specific FK graph, add a local `setup_repo()` / `seed_*()` helper on top of `SqliteTestDb` instead of reaching through `repo.db.inner()` in each test |
| Keep migrations out of per-test setup | Create one temp DB, migrate once, then seed rows; do not call `run_migrations()` inside every helper |
| Prefer explicit fixture ownership | Bind fixture as `_db` in the test body so cleanup timing stays obvious |
| Split slow suites from narrow logic tests | Keep pure/unit logic off SQLite when possible; reserve DB fixtures for repository and integration coverage |
| Sandbox-safe temp paths | If a test only needs “under HOME”, prefer `tempdir_in(std::env::current_dir()?)` over writing into `$HOME` root directly |
| Discover exact libtest paths first | If a filter misses, use `-- --list` before guessing more Cargo invocations |
| Run selective jobs sequentially | Many small targeted runs beat broad runs and avoid `.cargo-lock` contention |
| When a builder repeats across files, centralize it | Move shared fixture/builders into `src-tauri/src/testing/` once multiple suites need the same seeded graph |

## Agent Guidance

| Situation | Action |
|---|---|
| Converting an old SQLite test | Replace `open_memory_connection() + run_migrations()` with `SqliteTestDb` first, then extract shared seed helpers |
| Seeing remaining `open_memory_connection()` calls after migration work | Check whether the suite is connection/formatting-only before converting it; optimize real migration-replay hotspots first |
| Adding a new repo suite | Start from a suite-local `setup_*()` helper; only introduce a shared helper when repetition appears in multiple files |
| Verifying a migration | Test the migration itself explicitly; do not force every repo test to replay the full migration chain |
| Considering `cargo-nextest` tuning | Adjust `.config/nextest.toml` groups/profiles instead of ad hoc command-line concurrency flags |

## Ongoing Tuning

| Improvement | Why |
|---|---|
| Tune `cargo-nextest` groups/profiles as suites grow | Better concurrency control, retries, partitioning, and resource grouping for thousands of tests |
| Add shared seed helpers for common row graphs | Removes repeated SQL and makes suite setup cheaper to maintain |
| Group resource-sensitive tests explicitly | Prevent DB/file/git-heavy tests from competing with fast unit coverage |

## Formatter Warning

| Situation | Action |
|---|---|
| Need to change Rust code | Edit the smallest surface possible |
| Think "`cargo fmt` will be harmless" | Don’t do it here |
| Formatting is truly required | Ask first, keep it scoped, and commit it separately from logic changes |
