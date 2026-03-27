# Plan Verification

## Overview

Plan Verification is an automated adversarial review loop that stress-tests your ideation session's plan before it is accepted. A dedicated critic agent systematically finds gaps — critical, high, medium, and low severity — and the orchestrator corrects them across multiple rounds until convergence.

This feature was motivated by manual adversarial review of the plan verification feature itself: 4 rounds of manual review found 95 gaps, evolving the plan from v1 to v5. Plan Verification automates that exact process for every plan.

## Verification Flow

Verification runs in a **hidden child session** with a dedicated `plan-verifier` agent. The parent session stays unblocked for ideation work while the round loop runs independently.

```
create_plan_artifact() OR trigger_verification_http()
        │
        ▼
trigger_auto_verify_sync()  [atomic DB: status=reviewing, in_progress=1, generation++]
        │
        ▼
create_verification_child_session()
  ├─ session_purpose = Verification
  ├─ title = "Auto-verification (gen N)"
  ├─ description = "Run verification round loop. parent_session_id: ..., generation: ..., max_rounds: ..."
  └─ routes to plan-verifier agent (purpose-based routing)
        │
        ├─ orchestration_triggered=false? → reset_auto_verify_sync(parent)
        │
        ▼
plan-verifier agent (in child session):
  • Reads plan via get_session_plan (inherited from parent)
  • ROUND LOOP:
      A. Zombie guard: get_plan_verification(parent_session_id) — check generation
      B. Spawn plan-critic-completeness + plan-critic-implementation-feasibility (parallel Task subagents)
      C. Critics fetch plan via get_session_plan MCP tool (no prompt bloat)
      D. Critics return structured gaps (JSON)
         ├─ Parse failure? → record in sliding window
      E. Merge gaps, compute fingerprints, Jaccard similarity
      F. update_plan_verification(parent_session_id, ...) — writes to PARENT session
      G. Convergence check:
           ┌─ 0 critical AND 0 high AND 0 medium (round ≥ 2) → "zero_blocking" ✅
           ├─ Jaccard(round_N, round_N+1) ≥ 0.8 for 2 rounds → "jaccard_converged" ✅
           ├─ current_round ≥ max_rounds → "max_rounds" ✅ (hard cap)
           └─ ≥ 3 parse failures in last 5 rounds → "critic_parse_failure" ✅
      H. Not converged → correct plan via update_plan_artifact / edit_plan_artifact → next round
      I. Converged → update_plan_verification(parent_session_id, in_progress=false)
        │
        ▼
Child session archived automatically on agent exit
Parent session: status transitions to "verified" | "needs_revision" | "skipped"
```

## Convergence Algorithm

### 4-Layer Gap Fingerprint Normalization

Gaps are compared across rounds using normalized fingerprints, not raw text. This tolerates LLM paraphrase divergence, category drift, and morphological variance.

| Layer | What |
|-------|------|
| 1. Lowercase + strip punctuation | `"Auth. Missing!"` → `"auth missing"` |
| 2. Stop-word stripping | Removes `a`, `the`, `is`, `and`, etc. **Preserves negation**: `no`, `not`, `missing`, `lacks`, `without` |
| 3. Suffix stemming (10 rules) | `"authentication"` → `"authenticat"`, `"limiting"` → `"limit"` |
| 4. Sort + SHA256 (first 12 chars) | `sorted.join(" ")` → hash |

Category is **excluded** from the fingerprint — it is display metadata only. The critic may reclassify a gap without breaking convergence.

### Convergence Conditions

| Condition | Trigger | Reason |
|-----------|---------|--------|
| Zero blocking | `critical_count == 0 AND high_count == 0 AND medium_count == 0` (min round 2) | Primary exit |
| Jaccard similarity | `similarity(round_N_fingerprints, round_N+1_fingerprints) ≥ 0.8` for 2 consecutive rounds | Tolerates minor paraphrasing |
| Hard cap | `current_round ≥ max_rounds` | Safety net — always terminates |
| Flaky critic | `≥ 3 parse failures in last 5 rounds` | Sliding window, not strict consecutive |

### Best-Version Tracking

Each round is scored: `critical×10 + high×3 + medium×1`. At hard-cap exit, if the original plan's score is lower than the final auto-corrected version, "Revert & Skip" is prominently suggested with the score comparison shown.

## Verification Status Values

| Status | Meaning | Accept Blocked? |
|--------|---------|-----------------|
| `unverified` | Loop not started | Yes (when `require_verification_for_accept: true`) |
| `reviewing` | Loop active | Yes (in progress) |
| `needs_revision` | Critic found gaps; auto-correction in progress | Yes |
| `verified` | 0 critical gaps, convergence confirmed | No |
| `skipped` | User explicitly skipped | No |

## User Actions

### Start Verification
Available when `status == unverified`. Orchestrator spawns critic loop automatically when user triggers it.

### Skip Verification
Sets `status = skipped`. Bypasses the gate entirely. Use when you trust the plan and want to accept immediately.

### Revert & Skip (Atomic)
Available when verification found gaps and the original plan scored better. Single endpoint — one transaction:
1. Restores original plan artifact version
2. Sets `status = skipped`, `in_progress = false`, `convergence_reason = "user_reverted"`

No two-step race condition: partial failure is impossible.

### Retry (After Crash)
If the orchestrator crashes mid-verification, the reconciliation service resets both `verification_status → unverified` and `verification_in_progress → false` after a configurable timeout (default: 90 min). Users can then restart verification or skip.

## Configuration

In `ralphx.yaml`:

```yaml
ideation:
  verification:
    require_verification_for_accept: true    # Gate enforcement (default: true)
    reconciliation_stale_after_secs: 5400    # 90 min — reset stuck in_progress sessions
    reconciliation_interval_secs: 300        # 5 min — how often reconciler scans
```

Environment variables override yaml settings (prefix: `RALPHX_IDEATION_VERIFICATION_*`).

## Error Variants

Typed errors (no string comparison):

| Variant | When |
|---------|------|
| `NotVerified` | Session unverified, gate enabled |
| `InProgress { round, max_rounds }` | Verification active |
| `HasUnresolvedGaps { count }` | `needs_revision` status |
| `SkippedCannotUpdate` | Critic tries to update already-skipped session |
| `InvalidTransition { from, to }` | Invalid status state machine jump |
| `RoundExceedsMax { round, max }` | Critic reports round > max_rounds |
| `AgentCrashed { round }` | Recovery resets stuck session |

## MCP Tools (Orchestrator)

| Tool | Method | Description |
|------|--------|-------------|
| `update_plan_verification` | POST | Reports round results from critic. Required: `session_id`, `status`. Optional: `gaps`, `round`, `convergence_reason`, `in_progress` |
| `get_plan_verification` | GET | Reads current verification status, round history, and gap list |

Available to: `orchestrator-ideation`, `ideation-team-lead`, `plan-verifier`.

## Tauri Events

Event: `plan_verification:status_changed`

```json
{
  "session_id": "string",
  "status": "unverified | reviewing | verified | needs_revision | skipped",
  "in_progress": true,
  "round": 2,
  "max_rounds": 5,
  "gap_score": 23,
  "convergence_reason": "zero_blocking | jaccard_converged | max_rounds | critic_parse_failure | user_skipped | user_reverted"
}
```

Emitted from: POST verification handler, revert-and-skip handler, conditional reset in `update_plan_artifact`.

## Proposal Verification Gate

When `require_verification_for_proposals: true` (opt-in, default: `false`), the backend blocks proposal mutations on plans that haven't passed adversarial review. This is defense-in-depth: agents cannot create proposals on unreviewed plans.

### Config Field

```yaml
ideation:
  verification:
    require_verification_for_proposals: false   # Opt-in gate (default: false)
    require_verification_for_accept: true        # Acceptance gate (default: true)
```

Both fields are independent. `require_verification_for_proposals` only blocks proposal mutations — it does not affect plan acceptance.

### Gate Behavior by Operation

| Operation | Blocked statuses | Allowed statuses |
|-----------|-----------------|-----------------|
| Create | `Unverified`, `Reviewing`, `NeedsRevision` | `Verified`, `Skipped` |
| Update | `Reviewing`, `NeedsRevision` | `Unverified`, `Verified`, `Skipped` |
| Delete | `Reviewing`, `NeedsRevision` | `Unverified`, `Verified`, `Skipped` |
| Reorder / Toggle selection | Not gated (content-preserving) | — |

Update and delete allow `Unverified` by design — blocking edits before verification starts would lock users out of housekeeping.

### Error Messages

Error messages are relayed verbatim to external agents via the MCP server:

| Status | Error Message |
|--------|--------------|
| `Unverified` | "Cannot create proposals: plan verification has not been run. Either run verification (update_plan_verification with status 'reviewing') or skip it (update_plan_verification with status 'skipped', convergence_reason 'user_skipped')." |
| `Reviewing` | "Cannot {operation} proposals: plan verification is in progress (round {N}/{max}). Complete the current verification round before modifying proposals." |
| `NeedsRevision` | "Cannot {operation} proposals: plan verification found {N} unresolved gap(s). Update the plan to address gaps (update_plan_artifact), then re-run verification." |

`{operation}` = `create`, `update`, or `delete`. HTTP status code: `400 BAD_REQUEST`.

### Child Session Behavior

Child sessions inherit their gate check from the session that owns the plan:

| Child session state | Which status is checked |
|--------------------|------------------------|
| Has its own plan artifact (`plan_artifact_id` set) | Child's own verification status |
| Inherited plan only (`inherited_plan_artifact_id` set) | Parent session's verification status |
| No plan and no inherited plan | Gate skipped entirely (passthrough) |

**Edge cases:**
- **Parent deleted** (FK set to NULL after deletion): gate passes — blocking orphaned children creates dead-end sessions with no escape.
- **Parent archived**: parent's verification status is checked normally. Archived ≠ deleted; session data is intact. If parent is `NeedsRevision`, child is blocked — but child can create its own plan to escape.

### TOCTOU Protection

The gate runs inside a `db.run_transaction()` closure alongside the proposal mutation. All checks — session fetch, settings read, parent session lookup, proposal insert — share a single DB lock.

**Before (vulnerable):**
```
await get_session()  →  check status  →  await count_proposals()  →  await create()
     ↑ lock 1              ↑ stale             ↑ lock 2                  ↑ lock 3
```

**After (safe):**
```
db.run_transaction(|conn| {
    get_session_sync(conn)  →  check gate  →  count_sync(conn)  →  create_sync(conn)
         ↑ same lock               ↑ fresh        ↑ same lock          ↑ same lock
})
```

This prevents a concurrent verification status change from slipping between the check and the insert.

### Implementation

| File | Purpose |
|------|---------|
| `domain/services/verification_gate.rs` | `check_proposal_verification_gate()` — pure function: `(session, settings, parent_status: Option, operation: ProposalOperation) → Result` |
| `http_server/helpers.rs` | `create_proposal_impl()`, `update_proposal_impl()`, `delete_proposal_impl()` — gate wired inside `db.run_transaction()` |
| `infrastructure/sqlite/sqlite_ideation_settings_repo.rs` | `get_settings_sync(conn)` — reads settings inside the proposal transaction |

## Acceptance Path Enforcement

All 3 acceptance paths enforce the verification gate:

| Path | Handler | Gate |
|------|---------|------|
| Tauri IPC | `apply_proposals_to_kanban` | `check_verification_gate()` before `apply_proposals_core()` |
| Internal MCP HTTP | `POST /api/ideation/sessions/:id/apply-proposals` | Same gate |
| External MCP HTTP | `POST /api/external/apply_proposals` | Same gate via project scope check |

## Observability

Structured logs at all lifecycle points:

```
INFO  session_id=... "Verification started"
INFO  session_id=... round=2 gaps=5 critical=1 "Verification round completed"
INFO  session_id=... reason=zero_blocking rounds=3 "Verification converged"
WARN  session_id=... round=3 "Critic output parse failure"
ERROR session_id=... error=... "Verification agent crashed"
INFO  session_id=... "Reconciliation reset stuck verification"
```

## Implementation

| File | Purpose |
|------|---------|
| `domain/entities/ideation/types.rs` | `VerificationStatus`, `VerificationMetadata`, `VerificationGap`, `VerificationError` |
| `domain/services/gap_fingerprint.rs` | 4-layer normalization + Jaccard similarity |
| `domain/services/verification_gate.rs` | `check_verification_gate()` — shared across all 3 acceptance paths |
| `domain/repositories/ideation_session_repository.rs` | `update_verification_state()`, `reset_verification()`, `get_verification_status()`, `revert_plan_and_skip_with_artifact()` |
| `http_server/handlers/ideation.rs` | `POST /verification`, `GET /verification`, `POST /revert-and-skip` |
| `http_server/handlers/external.rs` | `POST /api/external/apply_proposals`, `POST /api/external/trigger_verification` |
| `http_server/handlers/session_linking.rs` | `create_verification_child_session()` — creates child with `session_purpose=Verification` |
| `application/reconciliation/verification_reconciliation.rs` | Startup + periodic stuck-session reset + orphaned child detection |
| `ralphx-plugin/agents/plan-verifier.md` | Dedicated agent owning the round loop in child session |
