# Plan Verification

## Overview

Plan Verification is an automated adversarial review loop that stress-tests your ideation session's plan before it is accepted. A dedicated critic agent systematically finds gaps — critical, high, medium, and low severity — and the orchestrator corrects them across multiple rounds until convergence.

This feature was motivated by manual adversarial review of the plan verification feature itself: 4 rounds of manual review found 95 gaps, evolving the plan from v1 to v5. Plan Verification automates that exact process for every plan.

## Verification Flow

```
User triggers verification
        │
        ▼
Orchestrator spawns critic (Task subagent)
        │
        ▼
Critic returns structured gaps (JSON)
        │
        ├─ Parse failure? → record in sliding window
        │
        ▼
Orchestrator processes round:
  • Computes gap fingerprints (4-layer normalization)
  • Tracks Jaccard similarity vs. previous round
  • Records gap_score = critical×10 + high×3 + medium×1
  • Tracks best_round_index (lowest gap_score)
        │
        ▼
Convergence check:
  ┌─ 0 critical AND high_count ≤ previous round → "zero_critical" ✅
  ├─ Jaccard(round_N, round_N+1) ≥ 0.8 for 2 consecutive rounds → "jaccard_converged" ✅
  ├─ current_round ≥ max_rounds → "max_rounds" ✅ (hard cap)
  └─ ≥ 3 parse failures in last 5 rounds → "critic_parse_failure" ✅
        │
        ├─ Not converged → orchestrator corrects plan → next round
        │
        ▼
Status transitions to "verified" | "needs_revision" | "skipped"
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
| Zero critical | `critical_count == 0 AND high_count ≤ previous_round_high_count` | Primary exit |
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

Available to: `orchestrator-ideation`, `ideation-team-lead`.

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
  "convergence_reason": "zero_critical | jaccard_converged | max_rounds | critic_parse_failure | user_skipped | user_reverted"
}
```

Emitted from: POST verification handler, revert-and-skip handler, conditional reset in `update_plan_artifact`.

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
INFO  session_id=... reason=zero_critical rounds=3 "Verification converged"
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
| `http_server/handlers/external.rs` | `POST /api/external/apply_proposals` |
| `application/reconciliation/verification_reconciliation.rs` | Startup + periodic stuck-session reset |
