# System Card: Human↔Coordinator↔Subagent Orchestration Pattern

> Derived from two plan-to-delivery cycles on 2026-02-13 in RalphX. Grounded in JSONL conversation logs, plan files (`shimmying-bouncing-creek.md`, `majestic-cuddling-wilkinson.md`), and git commit history (`895fa9b1`→`e1973fce`).

---

## 1. System Overview

Three-layer orchestration: a Human steers at 2-3 touchpoints, a Coordinator (Claude Opus 4.6) decomposes work into dependency graphs and executes via scoped Subagents in parallel waves with commit gates.

```
Human (steering — 2-3 touchpoints per 1-2h session)
  │
  ▼
Coordinator (Claude Opus 4.6 — plan design, direct execution, agent dispatch, commit gates)
  │
  ├──▶ Explore agents (read-only recon, 30-46 tool_uses each, ~100s)
  ├──▶ Plan agents (read-only synthesis, 37-40 tool_uses, ~160-200s)
  ├──▶ general-purpose agents (scoped file set, write tests/docs)
  │       │
  │       ▼
  │    Commit Gate (typecheck + tests + lint) → next wave
  │
  └──▶ Coordinator also executes directly when it has sufficient context
```

**Key finding:** The coordinator absorbs most execution work directly, delegating to subagents primarily for (1) time-expensive exploration where waiting would block progress, and (2) embarrassingly parallel test writing.

---

## 2. Lifecycle Phases

| Phase | Name | Hardening (observed) | Unification (observed) | Key Mechanics |
|-------|------|---------------------|----------------------|---------------|
| 1 | Discovery | 1 Explore agent, ~4 min | 3 parallel Explore agents, 2m 35s | Parallel read-only agents → codebase inventory |
| 2 | Plan Design | 3 parallel Explore audits, ~20 min | 2 parallel Plan agents, 4m 48s | Dependency graph, wave schedule, agent assignment |
| 3 | Plan Approval | ExitPlanMode → rejected → revised → approved | ExitPlanMode → rejected ("parallelize") → v2 approved | Human-gated; rejection refines plan quality |
| 4 | Execution | 3 parallel agents (test writing), ~2h | 5 waves, 6 agents + coordinator direct, 42m | Wave-based dispatch or independent parallel |
| 5 | Verification | `cargo test --lib`, `cargo clippy` | `npm run typecheck`, `vitest`, lint | Per-wave gates + final full suite |

**Observed:** Plan rejection is productive — both sessions had 1 rejection that improved the plan (Hardening: added two-layer test strategy; Unification: added 5-wave parallel strategy with conflict rules).

---

## 3. Agent Taxonomy

| Type | Tools | Scope | Observed Usage |
|------|-------|-------|----------------|
| Explore | Read, Grep, Glob | Read-only | Hardening: 1 infra audit + 3 code audits. Unification: 3 discovery + 1 panel scope |
| Plan | Read, Grep, Glob | Read-only | Unification: 2 parallel (doc plan + UI plan), 37-40 tool_uses each |
| general-purpose | Read, Write, Edit, Bash | Scoped file set | Hardening: 3 agents writing 8 test files. Unification: 2 agents writing test files + 1 doc |
| Bash | Bash only | Shell | Git ops, test runs, linting |

**Subagent performance (from Unification discovery):**

| Agent | tool_uses | tokens | duration |
|-------|-----------|--------|----------|
| Explore backend (a8f8cc7) | 46 | 89,569 | 100.4s |
| Explore frontend (a63be70) | 30 | 97,695 | 100.1s |
| Explore event flow (a90e37f) | 34 | 109,678 | 101.8s |
| Plan documentation (a94183b) | 37 | 105,334 | 162.7s |
| Plan UI unification (acda804) | 40 | 122,339 | 205.7s |

---

## 4. Parallel Execution Model

### Wave Model (Unification — 5 waves, 6 commits)

| Wave | Agents | Files | Gate |
|------|--------|-------|------|
| 1 | Agent A (docs) + Coordinator (registry) | 3 created | `npm run typecheck` + 63 tests |
| 2 | Coordinator (registry wiring, 5 files) | 5 modified | 140 tests pass, zero behavior changes |
| 3 | Coordinator (3 hooks) + 2 test agents | 4 created + 1 modified | 173 tests pass |
| 4 | Explore agent (scope) + Coordinator (wire) | 2 modified + 1 created | typecheck + 173 tests |
| 5 | Coordinator (delete + migrate + docs) | 5 deleted + 4 modified | full suite green, no dead imports |

### Independent Model (Hardening — 3 tiers, non-overlapping files, 2 commits)

Agent 1 (Critical: B1, H2, B2/H3) → `side_effects.rs`, `transition_handler/mod.rs`, `spawner.rs` | Agent 2 (High: E7, C5, D4) → `reconciliation.rs`, `chat_service_handlers.rs` | Agent 3 (Medium: F4-B5) → `git_service/mod.rs`, `git_commands.rs`, `task_cleanup_service.rs`

### Conflict Prevention Rules (from Unification plan, line 290-296)

| # | Rule |
|---|------|
| 1 | **File ownership** — each agent has exclusive write access; no two agents modify the same file in the same wave |
| 2 | **Create-before-modify** — waves 1-3 create new files first; agent crash doesn't corrupt existing code |
| 3 | **Commit gates** — every wave ends with a verified commit; no wave starts until previous is committed |
| 4 | **Read-only sources** — agents read existing files for reference but only modify files in their scope |
| 5 | **No cascading deletes** — files deleted only in waves 4-5, after replacements are verified working |

---

## 5. Plan Anatomy

### Agent Prompt Template (from Unification plan, line 298-314)

```
STRICT SCOPE:
- You may ONLY create/modify: [file list]
- You must NOT modify: [exclusion list]
- Read for reference only: [reference file list]

TASK: [specific deliverable]

TESTS: Write tests for your new code. Do NOT modify existing test files.

VERIFICATION: After completing, run [lint command] on modified files only.
```

### Two Archetypes (with observed commit structure)

| Archetype | Plan File | Ordering | Commits |
|-----------|-----------|----------|---------|
| Phase-driven | `majestic-cuddling-wilkinson.md` | 7 phases → 5 waves → 6 commits | Wave-gated |
| Tier-driven | `shimmying-bouncing-creek.md` | 4 tiers → 3 agents → 2 commits | Phase-gated (tests, then fixes) |

---

## 6. Human Steering Model

### Hardening — 6 interventions across 2 sessions (~2h 29m)

| # | Timestamp | Intervention | Effect |
|---|-----------|-------------|--------|
| 1 | 16:01 UTC | Provided full PRD with 43 scenarios (A1-H4) | 1 Explore + 3 parallel test agents |
| 2 | 17:02 UTC | "can you verify that by running the specs yourself?" | Coordinator ran `cargo test --lib` (112 pass) |
| 3 | 17:03 UTC | "launch another round of agents to identify discrepancies and gaps in the actual implementation" | 3 parallel Explore audit agents |
| 4 | 17:06 UTC | Rejected plan v1: "wouldn't be helpful to have a separate set of specs?" | Two-layer TDD strategy added (GAP + fix specs) |
| 5 | 17:22 UTC | Rejected plan v2 (chose new session with plan injected) | Phase 2 execution in fresh context |
| 6 | 18:30 UTC | "commit" | `e2b81f56` committed |

### Unification — 3 interventions across ~1h

| # | Timestamp | Intervention | Effect |
|---|-----------|-------------|--------|
| 1 | 18:57 UTC | "launch more agents to identify what needs to be done to unify all your chat interfaces" | 3 parallel Explore agents |
| 2 | 19:08 UTC | "we need to highly parallelize the execution of this plan using agents in a safe way" | Plan v2: 5-wave strategy, 9 agents, conflict rules added |
| 3 | 19:16 UTC | Provided plan as input to execution session | 8 TaskCreate calls in 25 seconds, execution began |

**Zero mid-execution interventions in either session.**

---

## 7. TDD Integration

| Pattern | Session | Flow | Observed |
|---------|---------|------|----------|
| Two-layer (GAP + fix specs) | Hardening | Layer 1: GAP tests assert broken behavior → Layer 2: Fix specs assert correct behavior (red→green) | 112 GAP tests + 24 fix spec tests across `hardening_fixes/` |
| Test-alongside | Unification | Create hook → Delegate test writing to parallel agent → Verify → Commit | 2 test agents ran in parallel (18+15 tests in ~2min each) |

**Test counts at Unification checkpoints:** After Wave 1+2: 140 → After Wave 3: 173 → Final: 173 (96 new tests total: 63 registry + 18 events + 15 actions)

---

## 8. Commit Strategy

### Hardening — 2 commits (phase-gated)

| Hash | Timestamp | Message | +/- |
|------|-----------|---------|-----|
| `895fa9b1` | 18:38:38 +0200 | `test: Add hardening test suite for agent execution pipeline` | 12 files, +3,702 |
| `e2b81f56` | 20:31:13 +0200 | `fix: Harden agent execution pipeline (Phase 2 — 11 fixes)` | 22 files, +945/-18 |

### Unification — 6 commits (wave-gated)

| Hash | Timestamp | Message | +/- |
|------|-----------|---------|-----|
| `e3d7a428` | 21:26:59 +0200 | `feat: Add chat context registry and unify store key builders (Phases 0-1)` | 8 files, +807/-66 |
| `c0bafc4c` | 21:36:24 +0200 | `feat: Add unified event/actions hooks (Phases 2-4)` | 5 files, +1,607/-24 |
| `daa1c9f9` | 21:42:45 +0200 | `feat: Wire unified hooks into IntegratedChatPanel (Phase 5a)` | 2 files, +198/-187 |
| `64a426c2` | 21:44:15 +0200 | `chore: Update IntegratedChatPanel test mocks` | 1 file, +11/-6 |
| `22574c9a` | 21:45:15 +0200 | `chore: Delete old hooks replaced by unified versions (Phase 6)` | 3 files, -863 |
| `e1973fce` | 21:58:09 +0200 | `feat: Migrate floating ChatPanel to unified hooks and delete old code (Phase 6)` | 5 files, +132/-792 |

---

## 9. Tool Usage Patterns

### Unification Execution Session — 221 tool calls

| Tool | Count | % | Primary Phase |
|------|-------|---|---------------|
| Bash | 61 | 28% | `npm run typecheck`, `vitest`, `git commit`, `rm` |
| Edit | 45 | 20% | Registry wiring, hook replacements, test mock updates |
| Read | 40 | 18% | Reading source files for understanding before editing |
| Grep | 26 | 12% | Dead import cleanup, usage verification |
| TaskUpdate | 22 | 10% | `in_progress` → `completed` transitions |
| TaskCreate | 8 | 4% | All 8 phases registered in 25 seconds |
| Task | 6 | 3% | Subagent launches (2 explore + 1 doc + 2 test + 1 explore) |
| Write | 5 | 2% | New files (registry, hooks, recovery) |

### Typical Coordinator Sequence

```
Read (understand) → Write/Edit (implement) → Bash (typecheck + test) → Grep (verify no dead refs) → TaskUpdate (checkpoint) → Bash (git commit)
```

---

## 10. Metrics & Benchmarks

| Metric | Hardening | Unification |
|--------|-----------|-------------|
| End-to-end wall-clock | ~2h 29m (16:01→18:31 UTC, 2 sessions) | ~1h 2m (18:57→19:58 UTC) |
| Active work time (excl. human idle) | ~53m | ~42m |
| Commits | 2 | 6 |
| Subagents spawned | 13 (1 Explore + 6 gen-purpose + 3 Explore audit + 3 fix agents) | 11 (3 Explore + 2 Plan + 6 execution) |
| Max parallel subagents | 3 | 3 (discovery), 2 (execution) |
| Files changed | 31 | 23 |
| Lines added | +4,643 | +2,618 |
| Lines deleted | -14 | -1,801 |
| Tests written | 136 (112 GAP + 24 fix specs) | 96 (63 + 18 + 15) |
| Coordinator tool calls | 197 (108 + 89) | 221 |
| Human interventions | 6 (PRD, verify, audit, 2 rejections, commit) | 3 |
| Error recovery cycles | 4 (context overflow + test fix + GAP update + clippy) | 4 (3 type errors + context exhaustion) |

---

## 11. Anti-Patterns & Failure Modes

| Anti-Pattern | Observed Risk | Mitigation (from sessions) |
|-------------|--------------|---------------------------|
| Two agents modify same file | Merge conflicts | File ownership model — Unification Wave 3 had 3 agents creating non-overlapping files |
| Delete before replace | Broken intermediate state | Create-before-delete — new hooks existed 2 commits before old ones deleted |
| Skip typecheck between waves | Cascading TS errors | Commit gates — `npm run typecheck` ran 8 times during Unification execution |
| Vague agent prompts | Hardening: 3 background agents superseded after context overflow; re-launched foreground with enriched prompts (9.5K→10.4K chars) | STRICT SCOPE template + code snippets + exact file paths + mock patterns |
| Coordinator delegates too eagerly | Agent round-trip slower than direct execution | Coordinator absorbed 7 of 9 planned agents in Unification, delegated only exploration + tests |
| Context window exhaustion mid-execution | Lost progress | Auto-continuation preserved all written files; 3 exhaustions across sessions, zero work lost |
| Aspirational verification commands | Silent failures | Every plan included exact `cargo test --lib`, `npm run typecheck`, `vitest run` commands |

---

## 12. Reproducible Process — Checklist

1. **Quantify the problem** — Hardening: 38 gap scenarios across 8 categories. Unification: 30+ duplicated conditionals, 8+ files touched per new context type.
2. **Choose plan archetype** — phase-driven (temporal waves for features/refactors) or tier-driven (priority ordering for bug fixes)
3. **Launch parallel Explore agents** — 2-3 agents, ~100s each, non-overlapping file sets. Capture: file inventory, duplication sites, dependency graph.
4. **Design plan with agent assignment table** — per agent: Create / Modify / Delete / Must NOT touch. Use STRICT SCOPE template (§5).
5. **Submit plan for human approval** — expect 1 rejection; rejection improves plan quality (observed in both sessions)
6. **Register tasks with dependencies** — `TaskCreate` batch + `TaskUpdate` dependency wiring (Unification: 8 tasks + 6 deps in 40s)
7. **Execute in waves** — 2-3 agents max per wave. Coordinator executes directly when context is sufficient; delegates exploration + tests.
8. **Commit gate per wave** — typecheck clean + tests green + lint pass on modified files. No wave starts until previous committed.
9. **Verify & clean up** — dead code `Grep`, full test suite, lint. Delete old files only after replacements verified (§4 Rule 5).
