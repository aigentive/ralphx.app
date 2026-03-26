> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# Synthetic Failure Archetype Reference

> Derived from synthetic data analysis: 4,052 commits (22.5% fix rate), 228k log lines, 33 agent specs, 23 rule files.
> Use this file to cross-check proposed changes against known recurring failure patterns BEFORE implementation begins.

## Purpose

Evaluation specialists and plan critics MUST check proposed changes against these 5 archetypes during ideation verification. Each archetype includes trigger conditions, affected files, quantified evidence, and a cross-reference to the applicable guard checklist.

---

## Archetype 1: Merge Worktree Lifecycle

| Field | Detail |
|-------|--------|
| **Trigger** | New merge path leaves worktree state inconsistent (create without matching cleanup, phantom branch reference, concurrent access) |
| **Affected Files** | `src-tauri/src/side_effects.rs`, `src-tauri/src/chat_service/chat_service_merge.rs` |
| **Evidence** | 164 fix commits, 19,350 phantom branch errors logged |
| **Still Active?** | Yes — highest-volume archetype in dataset |
| **Check** | Does this change add or modify a worktree creation/deletion path? → `.claude/rules/merge-worktree-invariants.md` |

**Key failure modes:**
- Worktree created but cleanup skipped on error/timeout/cancel exit paths
- Branch reference checked once, stale by the time worktree create runs
- Reconciler races with active merge phase, clobbers in-flight state

---

## Archetype 2: Auto-Transition Churn

| Field | Detail |
|-------|--------|
| **Trigger** | Auto-transition fires while agent is still live, OR fires repeatedly on app restart |
| **Affected Files** | `src-tauri/src/task_transition_service.rs`, `src-tauri/src/on_enter_states.rs` |
| **Evidence** | 30 fix commits, still active as of 2026-03-24 |
| **Still Active?** | Yes — most recent archetype in dataset |
| **Check** | Does this change modify state transitions or add new pipeline stages? → Verify single-fire guard exists on every auto-transition |

**Key failure modes:**
- Transition fires on Executing state when worker already running (double-fire)
- Restart replays all pending transitions without idempotency guard
- New state added without on_enter handler falls through to wrong default

---

## Archetype 3: SQLite Concurrent Access

| Field | Detail |
|-------|--------|
| **Trigger** | New async fn accesses SQLite without db.run wrapper |
| **Affected Files** | Any new async fn touching the database layer |
| **Evidence** | 45 DB-theme fix commits, 60-second lock starvation observed |
| **Still Active?** | Periodic — flares up when new DB access code is added |
| **Check** | Does this add async DB access? → DbConnection rule in CLAUDE.md (Rule #16) |

**Key failure modes:**
- Direct conn.lock().await in async context holds lock across await points
- Blocking query on async executor thread starves other tasks
- Lock starvation causes 60s timeout cascades across unrelated operations

---

## Archetype 4: Agent Status Desync

| Field | Detail |
|-------|--------|
| **Trigger** | New session or agent type added without wiring UI store key, event handlers, or status transitions |
| **Affected Files** | `src/components/IntegratedChatPanel.tsx`, `src/hooks/useAgentEvents.ts`, `src-tauri/src/commands/execution_commands.rs` |
| **Evidence** | 49 UI fix commits, 10 silent exit events observed |
| **Still Active?** | Periodic — flares up with each new agent type |
| **Check** | Does this add a new agent/session type? → `.claude/rules/event-coverage-checklist.md` |

**Key failure modes:**
- New context type has no store key → status stays idle even when agent running
- agent:run_completed not handled → spinner never clears
- Silent exit (no execution_complete call) → task stuck in Executing forever

---

## Archetype 5: Incomplete Event Coverage

| Field | Detail |
|-------|--------|
| **Trigger** | New feature ships without ALL exit paths (error, timeout, cancel, user action) emitting the required UI events |
| **Affected Files** | Ideation pipeline handlers, `src-tauri/src/commands/`, `ralphx-plugin/agents/` pipeline stage handlers |
| **Evidence** | 5 same-day finalize_proposals fix commits, cross-project gate bypass (5 errors) |
| **Still Active?** | Yes — surface area grows with every new pipeline feature |
| **Check** | Does this add pipeline functionality, a new MCP tool, or a new agent type? → `.claude/rules/event-coverage-checklist.md` |

**Key failure modes:**
- Happy path emits event; error path silently swallows exception with no UI feedback
- Timeout path never fires cleanup → UI shows stale in-progress state
- Gate condition checked only on one code path; alternate route bypasses guard

---

## Quick Reference: Archetype → Guard Mapping

| Archetype | Trigger Signal | Guard Checklist |
|-----------|---------------|-----------------|
| #1 Merge Worktree Lifecycle | Touches worktree create/delete paths | `.claude/rules/merge-worktree-invariants.md` |
| #2 Auto-Transition Churn | Modifies state transitions or adds pipeline stage | Verify single-fire guard on every auto-transition |
| #3 SQLite Concurrent Access | New async fn with DB access | `CLAUDE.md` Rule #16 (DbConnection) |
| #4 Agent Status Desync | New agent/session/context type | `.claude/rules/event-coverage-checklist.md` |
| #5 Incomplete Event Coverage | New pipeline feature, MCP tool, or agent type | `.claude/rules/event-coverage-checklist.md` |

---

## Evidence Source

All numbers derived from synthetic dataset:
- **4,052 commits** analyzed (22.5% fix rate = ~912 fix commits)
- **228,000 log lines** parsed for error patterns
- **33 agent spec files** audited for tool-prompt alignment
- **23 rule files** reviewed for gap coverage
- Date range: Covers full project history through 2026-03-25
