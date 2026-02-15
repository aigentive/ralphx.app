> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# Agent Teams: Context Window Relaunch Strategy

## Problem

Workers approaching 0% context → automatic compaction → quality degradation. Observed:

| Symptom | Root Cause |
|---------|-----------|
| Code shipped without tests | Compaction loses test-first discipline |
| Markdown docs instead of code | Worker loses implementation context |
| Thinking loops at low context | Insufficient runway for decisions |
| Quality drops significantly | <10% remaining = compacted context |

## Strategy: Proactive Relaunch at 20%

### Protocol

1. **Self-report**: Worker stops at ~20% remaining context
2. **Checkpoint**: Commit all work (even partial) to branch
3. **Handoff**: Worker messages team lead with: what's done, what's remaining, gotchas discovered
4. **Shutdown**: Team lead sends `shutdown_request`
5. **Relaunch**: Fresh worker spawned with handoff summary + task items + same worktree/branch

### Threshold Analysis

| % Remaining | Decision | Rationale |
|------------|----------|-----------|
| 30%+ | ❌ Too early | Wastes good capacity |
| 20-25% | ✅ Relaunch trigger | 5–10 min for handoff + commit, then ~100% fresh capacity |
| 10-15% | ⚠️ Late | Compaction already happening, handoff quality degrading |
| <5% | ❌ Too late | Worker in danger zone, output already poor |

## Why Git Is the State

| Question | Answer |
|----------|--------|
| How is work preserved? | Git commits → checkpoint always recoverable |
| How does new worker resume? | Read recent git diff → understand what was done |
| How is "what to do next" communicated? | Task descriptions (unchanged) |
| Cost of relaunch? | ~5–10 min for new worker to re-read git history |

## Team Lead Relaunch Prompt

```
You are {name}, continuing work from a previous worker that hit context limits.

YOUR WORKTREE: {worktree_path}
YOUR BRANCH: {branch}

HANDOFF FROM PREVIOUS WORKER:
{handoff_summary}

REMAINING WORK:
{remaining_items}

Start by reading the recent git diff to understand what's been implemented.
```

## Integration

- **Include in every worker prompt**: "Self-report when reaching ~20% context remaining."
- **Team lead action**: No manual monitoring — workers self-report.
- **Verifier/blocked tasks**: Unaffected — they wait for task completion regardless.
- **Multiple relaunches**: Fine — each picks up from the last commit.

## Observed Results

| Phase | Strategy | Context at End | Output Quality |
|-------|----------|----------------|-----------------|
| Phase 1 | None | 7% (rust), 0% (frontend) | Degraded at end |
| Phase 2 | Self-report at 20% | ~80%+ per relaunch | Consistent throughout |
