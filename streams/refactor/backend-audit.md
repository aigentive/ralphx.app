# Backend Refactor Audit — 2026-02-20

## Summary

| Metric | Value |
|--------|-------|
| Total .rs files scanned | 295+ |
| Files over 500 lines | ~50 |
| Files over 1000 lines | ~35 |
| Files under 300 lines | ~235 |

## Session Progress (2026-02-20)

| Work | Result |
|------|--------|
| `reconciliation.rs` module split | ✅ 2,384 → 112 lines (policy, handlers/merge, handlers/execution, metadata, events) |
| Batch inline test extraction | ✅ ~238 files processed, ~37k test lines moved to `*_tests.rs` companions |
| `agent_config` test failures (13) | 🔧 In progress |

**Tool created:** `scripts/extract_tests.py` — raw-string-aware programmatic test extractor for future use.

---

## Top Candidates (ranked by priority)

### #1: commands/execution_commands.rs (4,717 lines) — NEXT TARGET
- **Prod:** ~2,145 lines | **Tests:** ~2,572 lines | **Functions:** 117 | **Structs:** 14 | **Enums:** 2
- **Why:** Largest file in codebase. God object combining execution state management, Tauri command handlers (16+ commands), resume/restart logic, process management, quota sync. 15 importers — deeply coupled.
- **Split proposal:**
  - `execution_state.rs` — `ActiveProjectState`, `ExecutionState`, atomics (~270 lines)
  - `execution_types.rs` — Response/Input structs (~200 lines)
  - `execution_resume.rs` — `ResumeCategory`, `validate_resume()`, `restart_task()` (~400 lines)
  - `execution_processes.rs` — `RunningProcess`, GC/prune helpers (~200 lines)
  - `execution_commands.rs` — Remaining Tauri command handlers (~1,000 lines)
  - `tests/` — Extract 33 tests (~2,500 lines)
- **Risk:** High (15 importers). Must re-export from `mod.rs` or update all consumers.

### #2: infrastructure/agents/claude/stream_processor.rs (2,842 lines → ~1,500 prod after test extraction)
- **Tests already extracted** ✅ (stream_processor_tests.rs — 1,862 lines, commit `6c373814`)
- **Remaining prod:** ~1,500 lines | **Functions:** 70 | **Structs:** 9 | **Enums:** 5
- **Why:** Dense parsing logic mixing stream message type definitions with parsing/extraction functions. Only 3 importers — low risk.
- **Split proposal:**
  - `stream_types.rs` — All `StreamMessage`, `ContentBlock`, `ContentDelta`, etc. (~300 lines)
  - `stream_parser.rs` — Parsing/extraction functions (~500 lines)
  - `stream_processor.rs` — Orchestration logic (~700 lines)
- **Risk:** Low (3 importers).

### #3: application/task_scheduler_service.rs (~842 prod lines after test extraction)
- **Tests already extracted** ✅ (task_scheduler_service_tests.rs)
- **Prod:** ~842 lines | **Functions:** 56 | **Structs:** 3
- **Why:** Complex scheduling + retry + contention logic could benefit from separation.
- **Split proposal:**
  - `task_scheduler_service.rs` — Core scheduling logic (~500 lines)
  - `task_scheduler_retry.rs` — Contention retry + backoff logic (~300 lines)
- **Risk:** Medium (13 importers).

### #4: application/chat_service/ (11,099 lines total across 9 files)
- **Tests extracted** ✅ from most sub-files
- **chat_service_errors.rs tests extracted** ✅ (commit `04e70197`, 1,379 lines)
- **Remaining large prod files:** `chat_service_context.rs` (~719 prod), `chat_service_handlers.rs` (~700 prod), `chat_service_streaming.rs` (~700 prod), `mod.rs` (~921)
- **Why:** Several still exceed 500-line limit. Mixed concerns in handlers and context.
- **Split proposal:** Review `chat_service_handlers.rs` and `chat_service_context.rs` for further decomposition.
- **Risk:** Low-Medium (7 importers, contained within module).

### #5: infrastructure/agents/claude/agent_config/team_config.rs (~600 prod lines after test extraction)
- **Tests extracted** ✅ (team_config_tests.rs)
- **Prod:** ~600 lines | **Functions:** 92 | **Structs:** 7 | **Enums:** 2
- **Why:** Team constraints validation, spawn request processing, and process mapping — three distinct concerns.
- **Split proposal:**
  - `team_constraints.rs` — `TeamConstraints`, validation logic (~200 lines)
  - `spawn_requests.rs` — `TeammateSpawnRequest`, process slot logic (~200 lines)
  - `team_config.rs` — Core config types (~200 lines)
- **Risk:** Low (5 importers total).

### #6: infrastructure/agents/claude/agent_config/mod.rs (~1,000 prod lines after test extraction)
- **Tests extracted** ✅ (agent_config/tests.rs)
- **Prod:** ~1,000 lines | **Functions:** 74 | **Structs:** 7
- **Why:** Config resolution logic, tool allowlist computation, and agent defaults mixed together.
- **Split proposal:**
  - `agent_defaults.rs` — Default configs and constants (~300 lines)
  - `tool_resolution.rs` — Tool allowlist/include resolution (~400 lines)
  - `mod.rs` — Keep re-exports and core types (~300 lines)
- **Risk:** Low (5 importers, mostly internal).

### #7: infrastructure/agents/claude/claude_code_client.rs (~900 prod lines after test extraction)
- **Tests extracted** ✅ (claude_code_client_tests.rs)
- **Prod:** ~900 lines | **Functions:** 82 | **Structs:** 4
- **Why:** Mixes spawn logic, streaming logic, teammate interactive mode, and CLI argument building.
- **Split proposal:**
  - `spawn.rs` — Agent spawn + CLI arg building (~400 lines)
  - `streaming.rs` — Stream processing integration (~300 lines)
  - `types.rs` — StreamEvent, StreamingSpawnResult, etc. (~200 lines)
- **Risk:** Low-Medium (7 importers).

### #8: ~~Entity files with inline tests~~ ✅ DONE
- All 8 entity files extracted: task.rs, status.rs, workflow.rs, types.rs, review.rs, review_issue.rs, task_metadata.rs, project.rs
- **~5,261 test lines removed** from production files.

### #9: http_server/ large handlers (5,594 lines across 4 files)
- **Tests extracted** ✅ from all handler files
- **Remaining prod:**

| File | Lines | Functions |
|------|-------|-----------|
| handlers/teams.rs | ~1,150 prod | 30 |
| handlers/ideation.rs | ~900 prod | 22 |
| helpers.rs | ~870 prod | 29 |
| types.rs | ~1,156 | 9 (107 structs!) |

- **Why:** `types.rs` has 107 structs (request/response DTOs) in a single file.
- **Split proposal:**
  - `types.rs` → Split by domain: `types/ideation.rs`, `types/tasks.rs`, `types/teams.rs`, `types/git.rs`
  - `handlers/teams.rs` → `teams_crud.rs`, `teams_messaging.rs`, `teams_lifecycle.rs`
  - `helpers.rs` → `helpers/task_helpers.rs`, `helpers/git_helpers.rs`
- **Risk:** Medium (18 importers for HttpServerState from types.rs).

### #10: application/team_state_tracker.rs (~824 prod lines after test extraction)
- **Tests extracted** ✅ (team_state_tracker_tests.rs)
- **Prod:** ~824 lines | **Functions:** 52 | **Structs:** 14 | **Enums:** 4
- **Why:** 18 types defined in one file. Mixes team lifecycle, teammate management, message handling, cost tracking.
- **Split proposal:**
  - `team_types.rs` — `TeammateStatus`, `TeammateCost`, `TeammateInfo`, etc. (~200 lines)
  - `team_state_tracker.rs` — Core tracker logic (~400 lines)
  - `teammate_handles.rs` — `TeammateHandle`, process management (~200 lines)
- **Risk:** High (16 importers).

### #11: application/review_service.rs (~502 prod lines after test extraction)
- **Tests extracted** ✅ (review_service_tests.rs)
- **Prod:** ~502 lines | **Functions:** 93
- **Why:** 93 functions — very high fn density. At the limit but manageable.
- **Action:** Monitor. Split production code only if it grows further.
- **Risk:** Low (4 importers).

### #12: SQLite repos over 700 lines (9 files, ~9,092 lines total)
- **Tests extracted** ✅ from all repo files
- **Remaining prod lines per file:**

| File | Approx Prod Lines | Functions |
|------|-------------------|-----------|
| sqlite_artifact_repo.rs | ~800 | 63 |
| sqlite_activity_event_repo.rs | ~600 | 34 |
| sqlite_ideation_session_repo.rs | ~600 | 53 |
| sqlite_review_repo.rs | ~550 | 40 |
| sqlite_methodology_repo.rs | ~550 | 56 |
| sqlite_task_dependency_repo.rs | ~500 | 47 |
| sqlite_process_repo.rs | ~400 | 41 |
| sqlite_task_qa_repo.rs | ~400 | 26 |
| sqlite_task_step_repo.rs | ~350 | 26 |

- **Split proposal:** For each large repo: extract `{repo}_queries.rs` (complex queries) and `{repo}_mappers.rs` (row mapping).
- **Risk:** Low (repos implement trait interfaces).

### #13: domain/state_machine/transition_handler/merge_validation.rs (1,214 lines)
- **Prod:** unknown split | **Functions:** 3 | **Impls:** 0
- **Why:** Only 3 functions but 1,214 lines — very long validation functions with many branches.
- **Split proposal:** Needs deeper investigation. Split by validation concern.
- **Risk:** Low (contained within transition_handler module).

### #14: domain/state_machine/transition_handler/on_enter_states.rs (1,012 lines)
- **Prod:** unknown split | **Functions:** 2 | **Impls:** 1
- **Why:** Only 2 functions — extreme method length. One `on_enter` handler doing everything.
- **Split proposal:** Extract per-state handlers into separate functions/methods.
- **Risk:** Low (contained within transition_handler module).

---

## Already-Refactored (skip)

| Module | Before | After | Date |
|--------|--------|-------|------|
| `application/reconciliation/` | 2,384 lines | 112 lines + 5 submodules | 2026-02-20 |
| `application/reconciliation/handlers/` | 1,502 lines | mod.rs + merge.rs + execution.rs | 2026-02-20 |
| `domain/state_machine/transition_handler/side_effects.rs` | 7,273 lines | 583 lines | prior |
| `transition_handler/merge_outcome_handler.rs` | new | 450 lines | prior |
| `transition_handler/merge_strategies.rs` | new | 370 lines | prior |
| Batch inline test extraction | ~238 files | ~37k lines moved to `*_tests.rs` | 2026-02-20 |

---

## Priority Tiers (updated)

### Tier 1: Critical — next structural splits
1. `execution_commands.rs` — 4,717 lines, god object, split into 5+ modules
2. `stream_processor.rs` — ~1,500 prod lines, types + parsing + orchestration mixed
3. `http_server/types.rs` — 107 structs in one file

### Tier 2: High — prod files still over 500 lines
4. `agent_config/mod.rs` — ~1,000 prod lines
5. `task_scheduler_service.rs` — ~842 prod lines
6. `team_state_tracker.rs` — ~824 prod lines
7. `team_config.rs` — ~600 prod lines
8. `claude_code_client.rs` — ~900 prod lines

### Tier 3: Medium — prod files 500-700 lines, clean splits available
9. `chat_service_context.rs`, `chat_service_handlers.rs`, `chat_service_streaming.rs`
10. `sqlite_artifact_repo.rs` — ~800 prod lines
11. `merge_validation.rs` / `on_enter_states.rs` — contained, split when touching

### Tier 4: Done / Monitor
- All inline test extractions complete ✅
- `review_service.rs` — at limit, monitor
- SQLite repos under 600 lines — acceptable after test extraction

---

## Files Under 300 Lines (no action needed)
~235 files — healthy size, no refactoring needed.
