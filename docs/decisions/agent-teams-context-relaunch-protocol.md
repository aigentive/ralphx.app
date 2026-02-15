# Decision: Agent Teams Context Window Relaunch Protocol

**Date:** 2026-02-15
**Status:** APPROVED
**Approver:** Product Owner (Laza Bogdan)

## Summary

Adding context window relaunch protocol as built-in behavior for agent teams. Workers and specialists self-report at ~20% context remaining, commit work, provide handoff summary, and team leads relaunch fresh agents to continue from checkpointed state.

## Rationale

During Phase 1 implementation, observed quality degradation when workers approached 0% context:

| Observed Issue | Root Cause | Impact |
|----------------|-----------|---------|
| Code shipped without tests | Compaction loses test-first discipline | Quality violations |
| Markdown docs instead of executable code | Worker loses implementation context | Deliverable mismatch |
| Thinking loops at low context | Insufficient runway for decisions | Stalled progress |
| Significant quality drop below 10% | Automatic context compaction | Degraded output |

**Proactive relaunch at 20% solves this:**
- Fresh agent has ~100% context capacity for quality work
- 5-10 min overhead for handoff/commit acceptable vs. quality loss
- Git commits preserve state; new agent resumes from checkpoint
- Observed: Phase 2 with self-report maintained 80%+ context, consistent quality

## Relationship to Approved Briefs

This decision is **ADDITIVE** — does not modify approved designs from the 6 agent teams briefs. It extends team coordination protocols with a new quality assurance mechanism. Compatible with both ideation teams (Brief 1) and worker teams (Brief 2).

## Required Modifications

| File | Change Description |
|------|-------------------|
| `ralphx-plugin/agents/ideation-team-lead.md` | Add section: Lead responsibilities for context relaunch protocol (include self-report in teammate prompts, handle handoff messages, perform relaunch sequence) |
| `ralphx-plugin/agents/worker-team.md` | Add section: Same lead-side protocol for worker-team coordinator |
| `ralphx-plugin/agents/orchestrator-ideation-specialist.md` | Add one-liner: "Self-report when reaching ~20% context remaining. Commit work, message lead with handoff summary." |
| `ralphx-plugin/agents/orchestrator-ideation-critic.md` | Add one-liner: "Self-report when reaching ~20% context remaining. Commit work, message lead with handoff summary." |
| `ralphx-plugin/agents/orchestrator-ideation-advocate.md` | Add one-liner: "Self-report when reaching ~20% context remaining. Commit work, message lead with handoff summary." |
| `ralphx-plugin/agents/coder.md` | Add one-liner: "Self-report when reaching ~20% context remaining. Commit work, message lead with handoff summary." |
| `ralphx-plugin/agents/ralphx-worker-team.md` | Same as worker-team.md (if separate file exists) |
| `src-tauri/src/infrastructure/agents/team_config.rs` | Add `context_relaunch_threshold: Option<u8>` to `TeamConstraints` struct with serde default |
| `ralphx.yaml` | Add `context_relaunch_threshold: 20` to `team_constraints._defaults` section |
| `src-tauri/src/domain/artifacts/mod.rs` | Consider adding `TeamHandoff` artifact type (optional — can use TeamSummary for handoffs) |
| `src-tauri/src/infrastructure/agents/team_state_tracker.rs` | Add `relaunch_count: u8` field to teammate tracking for observability |

## Implementation Scope Estimate

**Size:** Small — primarily prompt text changes + minimal backend config

| Component | Estimated Lines Changed |
|-----------|------------------------|
| Prompt files (7 agents) | ~150 lines added (30-40 per lead, 5-10 per specialist) |
| `TeamConstraints` struct | ~5 lines (1 field + serde default fn) |
| `ralphx.yaml` defaults | ~1 line |
| Artifact type (if added) | ~15 lines (enum variant + serde impl) |
| Team state tracker | ~3 lines (field + initialization) |
| **Total** | ~175 lines |

**Risk:** Low — no state machine changes, no UI changes, additive to existing prompts

## Phase Assignment

**Phase 1C** — After Phase 1B (ideation/worker integration) and before Phase 2 (split-pane UI)

**Rationale:**
- Depends on team coordination primitives (SendMessage, shutdown protocol) from Phase 1B
- Independent of UI work in Phase 2
- Improves quality for all team sessions once deployed
- Can ship as hotfix if quality issues emerge in Phase 1B testing

## Implementation Order

1. Add `context_relaunch_threshold` to `TeamConstraints` + defaults
2. Update team lead prompts (ideation-team-lead.md, worker-team.md)
3. Update specialist/advocate/critic prompts
4. Add `relaunch_count` tracking (observability)
5. (Optional) Add `TeamHandoff` artifact type if structured handoffs prove valuable
6. Test with synthetic long-running task to validate relaunch behavior
