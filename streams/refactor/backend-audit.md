# Backend Refactor Audit — 2026-02-20

## Summary

| Metric | Value |
|--------|-------|
| Total .rs files scanned | 295+ |
| Files over 500 lines | ~50 |
| Files over 1000 lines | ~35 |
| Files under 300 lines | ~235 |

## Top Candidates (ranked by priority)

### #1: commands/execution_commands.rs (4,717 lines)
- **Prod:** ~2,145 lines | **Tests:** ~2,572 lines | **Functions:** 117 | **Structs:** 14 | **Enums:** 2
- **Why:** Largest file in codebase. God object combining execution state management, Tauri command handlers (16+ commands), resume/restart logic, process management, quota sync, and 33 inline tests. 15 importers — deeply coupled.
- **Split proposal:**
  - `execution_state.rs` — `ActiveProjectState`, `ExecutionState`, atomics (~270 lines)
  - `execution_types.rs` — Response/Input structs: `ExecutionStatusResponse`, `ExecutionSettingsResponse`, `GlobalExecutionSettingsResponse`, `UpdateExecutionSettingsInput`, etc. (~200 lines)
  - `execution_resume.rs` — `ResumeCategory`, `CategorizedResume`, `ResumeValidationResult`, `validate_resume()`, `restart_task()`, `categorize_resume_state()` (~400 lines)
  - `execution_processes.rs` — `RunningProcess`, `get_running_processes()`, GC/prune helpers (~200 lines)
  - `execution_commands.rs` — Remaining Tauri command handlers (~1,000 lines)
  - `tests/` — Extract 33 tests to separate test files (~2,500 lines)
- **Risk:** High (15 importers). Must re-export from `mod.rs` or update all consumers.

### #2: infrastructure/agents/claude/stream_processor.rs (2,842 lines)
- **Prod:** ~1,500 lines | **Tests:** ~1,342 lines | **Functions:** 70 | **Structs:** 9 | **Enums:** 5
- **Why:** Dense parsing logic mixing stream message type definitions (StreamMessage, ContentBlock, etc.) with parsing/extraction functions and 60 inline tests. Only 3 importers — low risk.
- **Split proposal:**
  - `stream_types.rs` — All `StreamMessage`, `ContentBlock`, `ContentDelta`, etc. enums/structs (~300 lines)
  - `stream_parser.rs` — Parsing/extraction functions (~500 lines)
  - `stream_processor.rs` — Orchestration logic (~700 lines)
  - `tests/` — Extract 60 tests (~1,300 lines)
- **Risk:** Low (3 importers).

### #3: application/task_scheduler_service.rs (2,415 lines)
- **Prod:** ~842 lines | **Tests:** ~1,573 lines | **Functions:** 56 | **Structs:** 3
- **Why:** Production code is borderline acceptable at 842 lines, but has 1,573 lines of inline tests that should be extracted. Complex scheduling + retry + contention logic could benefit from separation.
- **Split proposal:**
  - `task_scheduler_service.rs` — Keep core scheduling logic (~500 lines)
  - `task_scheduler_retry.rs` — Contention retry + backoff logic (~300 lines)
  - `tests/` — Extract all tests (~1,573 lines)
- **Risk:** Medium (13 importers).

### #4: application/chat_service/ (11,099 lines total across 9 files)
- **Largest file:** `chat_service_errors.rs` — 1,927 lines (~549 prod, ~1,378 tests), 99 functions
- **Other big files:** `chat_service_context.rs` (1,392), `chat_service_handlers.rs` (1,368), `chat_service_streaming.rs` (1,313), `mod.rs` (921)
- **Why:** Already split into sub-files, but several exceed 500-line limit. `chat_service_errors.rs` has 78 inline tests that should be extracted. `chat_service_handlers.rs` and `chat_service_context.rs` likely have mixed concerns.
- **Split proposal:**
  - Extract tests from `chat_service_errors.rs` → `chat_service_errors_tests.rs` (~1,378 lines)
  - Extract tests from other sub-files
  - Review `chat_service_handlers.rs` and `chat_service_context.rs` for further decomposition
- **Risk:** Low-Medium (7 importers for errors, contained within module).

### #5: infrastructure/agents/claude/agent_config/team_config.rs (1,564 lines)
- **Prod:** ~600 lines | **Tests:** ~964 lines | **Functions:** 92 | **Structs:** 7 | **Enums:** 2
- **Why:** 68 inline tests. Production code includes team constraints validation, spawn request processing, and process mapping — three distinct concerns.
- **Split proposal:**
  - `team_constraints.rs` — `TeamConstraints`, validation logic (~200 lines)
  - `spawn_requests.rs` — `TeammateSpawnRequest`, process slot logic (~200 lines)
  - `team_config.rs` — Core config types (~200 lines)
  - `tests/` — Extract 68 tests (~964 lines)
- **Risk:** Low (used within agent_config module, 5 importers total).

### #6: infrastructure/agents/claude/agent_config/mod.rs (1,681 lines)
- **Prod:** ~1,000 lines | **Tests:** ~681 lines | **Functions:** 74 | **Structs:** 7
- **Why:** 37 inline tests. Config resolution logic, tool allowlist computation, and agent defaults mixed together.
- **Split proposal:**
  - `agent_defaults.rs` — Default configs and constants (~300 lines)
  - `tool_resolution.rs` — Tool allowlist/include resolution (~400 lines)
  - `mod.rs` — Keep re-exports and core types (~300 lines)
  - `tests/` — Extract 37 tests (~681 lines)
- **Risk:** Low (5 importers, mostly internal).

### #7: infrastructure/agents/claude/claude_code_client.rs (1,658 lines)
- **Prod:** ~900 lines | **Tests:** ~758 lines | **Functions:** 82 | **Structs:** 4
- **Why:** 45 inline tests. Mixes spawn logic, streaming logic, teammate interactive mode, and CLI argument building.
- **Split proposal:**
  - `spawn.rs` — Agent spawn + CLI arg building (~400 lines)
  - `streaming.rs` — Stream processing integration (~300 lines)
  - `types.rs` — StreamEvent, StreamingSpawnResult, etc. (~200 lines)
  - `tests/` — Extract 45 tests (~758 lines)
- **Risk:** Low-Medium (7 importers).

### #8: Entity files with inline tests (8 files, 8,378 lines total)
| File | Total | Prod | Tests | Test count |
|------|-------|------|-------|------------|
| task.rs | 1,238 | ~276 | ~961 | 56 |
| status.rs | 1,099 | ~264 | ~834 | 87 |
| workflow.rs | 1,080 | ~498 | ~581 | 43 |
| types.rs | 1,060 | ~320 | ~739 | 95 |
| review.rs | 1,059 | ~576 | ~482 | 38 |
| review_issue.rs | 1,015 | ~604 | ~410 | 25 |
| task_metadata.rs | 955 | ~293 | ~661 | 38 |
| project.rs | 872 | ~278 | ~593 | 47 |
- **Why:** Every entity file is test-heavy. Production code is mostly under 500 lines, but total file sizes exceed limits due to inline tests. Batch test extraction would save ~5,261 lines.
- **Split proposal:** Extract `#[cfg(test)] mod tests` from each file into `tests.rs` sibling files (pattern already used by reconciliation/).
- **Risk:** Very Low (tests only, no API surface changes).

### #9: http_server/ large handlers (5,594 lines across 4 files)
| File | Lines | Functions |
|------|-------|-----------|
| handlers/teams.rs | 1,481 | 30 |
| handlers/ideation.rs | 1,230 | 22 |
| helpers.rs | 1,179 | 29 |
| types.rs | 1,156 | 9 (107 structs!) |
- **Why:** `types.rs` has 107 structs (request/response DTOs) in a single file. `teams.rs` and `ideation.rs` are sprawling handler files. `helpers.rs` mixes unrelated helper functions.
- **Split proposal:**
  - `types.rs` → Split by domain: `types/ideation.rs`, `types/tasks.rs`, `types/teams.rs`, `types/git.rs`, etc.
  - `handlers/teams.rs` → Group by feature: `teams_crud.rs`, `teams_messaging.rs`, `teams_lifecycle.rs`
  - `helpers.rs` → Group by domain: `helpers/task_helpers.rs`, `helpers/git_helpers.rs`
- **Risk:** Medium (18 importers for HttpServerState from types.rs).

### #10: application/team_state_tracker.rs (1,154 lines)
- **Prod:** ~824 lines | **Tests:** ~330 lines | **Functions:** 52 | **Structs:** 14 | **Enums:** 4
- **Why:** 18 types defined in one file. Mixes team lifecycle, teammate management, message handling, cost tracking, and process handle management.
- **Split proposal:**
  - `team_types.rs` — `TeammateStatus`, `TeammateCost`, `TeammateInfo`, etc. (~200 lines)
  - `team_state_tracker.rs` — Core tracker logic (~400 lines)
  - `teammate_handles.rs` — `TeammateHandle`, process management (~200 lines)
  - `tests/` — Extract tests (~330 lines)
- **Risk:** High (16 importers).

### #11: application/review_service.rs (1,435 lines)
- **Prod:** ~502 lines | **Tests:** ~933 lines | **Functions:** 93
- **Why:** 93 functions crammed into one service — very high fn density. Test extraction straightforward.
- **Split proposal:** Extract tests to `review_service/tests.rs` (~933 lines). Further split production code if warranted after extraction.
- **Risk:** Low (4 importers).

### #12: SQLite repos over 700 lines (9 files, ~9,092 lines total)
| File | Lines | Functions |
|------|-------|-----------|
| sqlite_artifact_repo.rs | 1,574 | 63 |
| sqlite_activity_event_repo.rs | 1,154 | 34 |
| sqlite_ideation_session_repo.rs | 1,150 | 53 |
| sqlite_review_repo.rs | 1,050 | 40 |
| sqlite_methodology_repo.rs | 1,028 | 56 |
| sqlite_task_dependency_repo.rs | 964 | 47 |
| sqlite_process_repo.rs | 781 | 41 |
| sqlite_task_qa_repo.rs | 745 | 26 |
| sqlite_task_step_repo.rs | 647 | 26 |
- **Why:** All exceed 500-line limit. Repos mix CRUD, complex queries, and row mapping. Most have no inline tests (tests in separate files).
- **Split proposal:** For each large repo: extract `{repo}_queries.rs` (complex queries) and `{repo}_mappers.rs` (row mapping helpers).
- **Risk:** Low (repos implement trait interfaces, changes are internal).

### #13: domain/state_machine/transition_handler/merge_validation.rs (1,214 lines)
- **Prod:** unknown split | **Functions:** 3 | **Impls:** 0
- **Why:** Only 3 functions but 1,214 lines — likely very long validation functions with many branches.
- **Split proposal:** Needs deeper investigation. Could split by validation concern.
- **Risk:** Low (contained within transition_handler module).

### #14: domain/state_machine/transition_handler/on_enter_states.rs (1,012 lines)
- **Prod:** unknown split | **Functions:** 2 | **Impls:** 1
- **Why:** Only 2 functions — extreme method length. One `on_enter` handler doing everything.
- **Split proposal:** Extract per-state handlers into separate functions/methods.
- **Risk:** Low (contained within transition_handler module).

## Already-Refactored (skip)
- `application/reconciliation/` — refactored 2026-02-20
- `application/reconciliation.rs` — now 112 lines (post-refactor)
- `domain/state_machine/transition_handler/side_effects.rs` — 583 lines (previously 7,273)
- `domain/state_machine/transition_handler/merge_outcome_handler.rs` — 450 lines
- `domain/state_machine/transition_handler/merge_strategies.rs` — 370 lines

## Quick-Win: Batch Test Extraction

The single highest-ROI refactor is extracting inline tests from ~20 files. This is mechanical, low-risk, and would remove ~15,000+ lines from production files:

| Category | Files | Estimated Test Lines |
|----------|-------|---------------------|
| Entity files | 8 | ~5,261 |
| execution_commands.rs | 1 | ~2,572 |
| task_scheduler_service.rs | 1 | ~1,573 |
| chat_service_errors.rs | 1 | ~1,378 |
| team_config.rs | 1 | ~964 |
| review_service.rs | 1 | ~933 |
| task_context_service.rs | 1 | ~748 |
| agent_config/mod.rs | 1 | ~681 |
| claude_code_client.rs | 1 | ~758 |
| stream_processor.rs | 1 | ~1,342 |
| team_state_tracker.rs | 1 | ~330 |
| **Total** | **18** | **~16,540** |

## Priority Tiers

### Tier 1: Critical (>1000 prod lines, structural issues)
1. `execution_commands.rs` — god object, split into 5+ modules
2. `stream_processor.rs` — types + parsing + orchestration mixed
3. `http_server/types.rs` — 107 structs in one file

### Tier 2: High (500-1000 prod lines, clean splits available)
4. `task_scheduler_service.rs` — scheduling + retry separation
5. `chat_service_errors.rs` — error types + classification separation
6. `team_state_tracker.rs` — types + tracker + handles separation
7. `team_config.rs` — constraints + spawn + config separation
8. `agent_config/mod.rs` — defaults + resolution + types separation

### Tier 3: Medium (quick-win test extractions)
9. All 8 entity files — batch test extraction
10. `review_service.rs` — test extraction
11. `claude_code_client.rs` — test extraction + optional structural split

### Tier 4: Low (large but stable)
12. SQLite repos — split only if being actively modified
13. HTTP handlers — split only if being actively modified
14. `merge_validation.rs` / `on_enter_states.rs` — contained within module

## Files Under 300 Lines (no action needed)
~235 files — healthy size, no refactoring needed.
