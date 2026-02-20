# Reconciliation Extraction Plan

## Current State
- `reconciliation.rs`: 2,384 lines (monolithic)
- `reconciliation/tests.rs`: 3,173 lines (already extracted)
- Module structure: Rust 2018+ (`reconciliation.rs` + `reconciliation/` dir coexist)

## Target State
```
reconciliation.rs        (~110 lines — struct + constructor + mod declarations)
reconciliation/
├── policy.rs            (~270 lines — types + pure decision logic)
├── handlers.rs          (~1250 lines — all reconcile_* + apply_recovery_decision)
├── metadata.rs          (~180 lines — retry counters, SHA tracking, backoff)
├── events.rs            (~350 lines — evidence, prompts, event recording, lookups)
├── helpers.rs           (~60 lines — free functions + simple checkers)
└── tests.rs             (3,173 lines — already exists)
```

**Note:** handlers.rs exceeds 500-line limit. Future split recommended into execution/merge/orchestration/paused sub-handlers.

---

## Pass 1: policy.rs (Lines 35–304, ~270 lines)

All types + RecoveryPolicy pure decision logic (no I/O, no async):

| Item | Lines | Type |
|------|-------|------|
| `RecoveryContext` enum | 35–43 | enum |
| `RecoveryActionKind` enum | 45–52 | enum |
| `RecoveryDecision` struct | 54–58 | struct |
| `RecoveryEvidence` struct + impl | 60–77 | struct+impl |
| `RecoveryPolicy` struct + impl | 79–267 | struct+impl |
| `RecoveryPromptAction` struct | 269–274 | struct (Serialize) |
| `RecoveryPromptEvent` struct | 276–285 | struct (Serialize) |
| `UserRecoveryAction` enum | 287–291 | pub enum |
| `ShaComparisonResult` enum | 293–304 | enum |

**Imports needed:** `serde::Serialize`, entity types (`AgentRunStatus`, `InternalStatus`)
**Visibility:** `pub(crate)` for all types (accessed by handlers, events, metadata)
**Dependencies:** None (pure logic)

---

## Pass 2: handlers.rs (Lines 378–1777, ~1250 lines)

All `reconcile_*` methods, orchestration, and `apply_recovery_decision`:

| Method | Lines | Notes |
|--------|-------|-------|
| `recover_timeout_failures` | 378–461 | pub, calls task_is_timeout_failure (metadata), record_auto_retry_metadata (metadata) |
| `task_is_timeout_failure` | 463–476 | private, pure metadata read — BUT used only by recover_timeout_failures, keep here or metadata |
| `reconcile_stuck_tasks` | 478–524 | pub entry point |
| `prune_stale_running_registry_entries` | 526–639 | calls context_matches_task_status (helpers), process_is_alive (helpers) |
| `reconcile_task` | 641–659 | pub dispatcher |
| `reconcile_completed_execution` | 661–742 | calls build_run_evidence (events), policy, metadata fns |
| `reconcile_reviewing_task` | 744–826 | same pattern |
| `reconcile_merging_task` | 828–934 | calls record_merge_timeout_event (events) |
| `reconcile_qa_task` | 936–1022 | same pattern |
| `reconcile_pending_merge_task` | 1024–1110 | calls latest_deferred_blocker_id (events) |
| `reconcile_merge_incomplete_task` | 1112–1223 | calls metadata + helpers fns |
| `reconcile_merge_conflict_task` | 1225–1331 | calls SHA comparison (metadata), events |
| `reconcile_paused_provider_error` | 1333–1508 | self-contained |
| `reconcile_paused_provider_error_legacy` | 1510–1590 | self-contained |
| `recover_execution_stop` | 1592–1635 | pub |
| `apply_recovery_decision` | 1706–1777 | private, called by all reconcile_* |

**Imports needed:** `use super::*;` + policy types, metadata/events/helpers functions
**Dependencies:** policy.rs must exist first

---

## Pass 3: metadata.rs (Lines 1876–2086 + 463–476, ~180 lines)

Retry counters, SHA tracking, backoff delays — all metadata read/write methods:

| Method | Lines | Type |
|--------|-------|------|
| `task_is_timeout_failure` | 463–476 | `&self` (reads task.metadata) |
| `merging_auto_retry_count` | 1876–1888 | `fn(task)` static |
| `auto_retry_count_for_status` | 1890–1903 | `fn(task, status)` static |
| `record_auto_retry_metadata` | 1905–1930 | `&self` async (writes metadata) |
| `merge_incomplete_auto_retry_count` | 1932–1943 | `fn(task)` static |
| `merge_incomplete_retry_delay` | 1945–1953 | `fn(count)` static + rand |
| `merge_conflict_auto_retry_count` | 1955–1966 | `fn(task)` static |
| `merge_conflict_retry_delay` | 1968–1976 | `fn(count)` static + rand |
| `get_rate_limit_retry_after` | 2001–2007 | `fn(task)` static |
| `clear_rate_limit_retry_after` | 2009–2032 | `&self` async |
| `last_stored_source_sha` | 2034–2045 | `pub(crate) fn(task)` static |
| `get_current_source_sha` | 2047–2064 | `&self` async |
| `check_source_sha_changed` | 2066–2086 | `&self` async |

**Imports needed:** `use super::*;` + rand, chrono, entity types, GitService, reconciliation_config
**Dependencies:** None (operates on task metadata)

---

## Pass 4: events.rs (Lines 1637–1704 + 1779–1874 + 2088–2342, ~350 lines)

Evidence building, event recording, prompts, lookups:

| Method | Lines | Type |
|--------|-------|------|
| `apply_user_recovery_action` | 1637–1704 | pub |
| `build_run_evidence` | 1779–1800 | async |
| `load_execution_run` | 1802–1844 | async |
| `latest_status_transition_age` | 1846–1874 | async |
| `record_merge_auto_retry_event_with_sha` | 2088–2134 | async |
| `latest_deferred_blocker_id` | 2136–2146 | `&self` (reads metadata) |
| `deferred_blocker_is_active` | 2148–2167 | async |
| `record_merge_auto_retry_event` | 2169–2210 | async |
| `record_merge_timeout_event` | 2212–2265 | async |
| `emit_recovery_prompt` | 2267–2313 | async |
| `clear_prompt_marker` | 2315–2319 | async |
| `lookup_latest_run_for_task_context` | 2321–2342 | async |

**Imports needed:** `use super::*;` + policy types (RecoveryContext, etc.), entity types
**Dependencies:** policy.rs must exist

---

## Pass 5: helpers.rs (Lines 1978–1999 + 2344–2381, ~60 lines)

Free functions + simple metadata checkers:

| Function | Lines | Type |
|----------|-------|------|
| `is_agent_reported_failure` | 1978–1989 | `pub(crate) fn(task)` static |
| `validation_revert_count` | 1991–1999 | `pub(crate) fn(task)` static |
| `context_matches_task_status` | 2344–2355 | free fn |
| `process_is_alive` | 2357–2381 | free fn |

**Imports needed:** entity types (Task, InternalStatus, ChatContextType, MergeFailureSource), AGENT_ACTIVE_STATUSES
**Dependencies:** None

---

## Remaining in reconciliation.rs (~110 lines)

| Item | Lines | Notes |
|------|-------|-------|
| Imports | ~20 | Reduced (submodules have own imports) |
| `mod` declarations | ~8 | `pub(crate) mod policy/handlers/metadata/events/helpers; mod tests;` |
| `pub use` re-exports | ~5 | `UserRecoveryAction`, `ReconciliationRunner` (already pub) |
| `ReconciliationRunner` struct | 306–325 | ~20 lines |
| `impl new()` | 329–366 | ~38 lines |
| `with_app_handle()` | 368–371 | ~4 lines |
| `with_plan_branch_repo()` | 373–376 | ~4 lines |

---

## Extraction Order & Dependencies

```
Pass 1: policy.rs      ← no deps, extract first
Pass 2: handlers.rs    ← depends on policy types
Pass 3: metadata.rs    ← depends on policy types (ShaComparisonResult)
Pass 4: events.rs      ← depends on policy types (RecoveryContext, etc.)
Pass 5: helpers.rs     ← no deps on other submodules + cleanup mod.rs
```

## Module Wiring Pattern

Each submodule uses:
```rust
use super::*;  // gets ReconciliationRunner, imports from mod.rs
use super::policy::{RecoveryContext, RecoveryActionKind, RecoveryDecision, RecoveryEvidence, ...};
```

Cross-module calls (handlers → metadata/events/helpers):
```rust
use super::metadata::{...};
use super::events::{...};
use super::helpers::{context_matches_task_status, process_is_alive};
```

## Commit Format
```
refactor: extract reconciliation/<module>.rs

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
```
