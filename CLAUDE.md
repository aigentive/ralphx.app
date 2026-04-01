> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# CLAUDE.md

## Priority Zero — Owner Strategy Alignment (NON-NEGOTIABLE)

Before ANY user-facing content, documentation, UI copy, or messaging work, agents MUST load these on demand:
- `~/.ralphx/founder/founder-profile.md` — Owner vision and non-negotiables
- `~/.ralphx/strategy/project-goal-card.md` — Messaging architecture, positioning, ICPs, competitive landscape
- `~/.ralphx/strategy/project-metrics.md` — Verifiable project data points

These are the **owner's directives**. They override default agent judgment on messaging. Do not keep them as always-on `@` imports in project memory.

---

## Project: RalphX
Native Mac GUI for autonomous AI dev: Kanban, multi-agent orchestration, ideation chat.
Code quality: `.claude/rules/code-quality-standards.md` | State machine: `.claude/rules/task-state-machine.md` | Git/merge: `.claude/rules/task-git-branching.md` | Agents: `.claude/rules/task-execution-agents.md` | Follow-up blocker dedupe: `.claude/rules/followup-blocker-dedupe.md` | Agent type map: `.claude/rules/agent-type-map.md` | Task detail views: `.claude/rules/task-detail-views.md` | Rust API safety: `.claude/rules/rust-stable-apis.md` | Rust test execution: `.claude/rules/rust-test-execution.md`

## Structure
```
ralphx/
├─ src/                   # Frontend (React/TS) → src/CLAUDE.md
├─ src-tauri/             # Backend (Rust/Tauri) → src-tauri/CLAUDE.md
│  └─ ralphx.db           # SQLite (dev)
├─ ralphx-plugin/         # Claude plugin (agents/skills/hooks)
│  ├─ ralphx-mcp-server/  # Internal agent MCP (stdio → :3847)
│  └─ ralphx-external-mcp/# External API MCP (HTTP :3848 → :3847)
```

## Context Window Preservation (NON-NEGOTIABLE)

This is a **large codebase** (~100k+ lines across Rust backend + React frontend). Every agent — lead, teammate, or standalone — MUST protect its context window.

| Rule | Detail |
|------|--------|
| **Never explore manually** | ❌ Reading file after file yourself. ✅ Spawn `Task(Explore)` or `Task(general-purpose)` subagents to search/read in parallel. |
| **Leads only delegate** | Team leads coordinate and review. ❌ Leads doing research, running tests, or reading code directly. ✅ Spawn teammates/subagents for ALL work. |
| **Parallel exploration** | Need info from 3+ files? Spawn 3 subagents in parallel. ❌ Sequential file reads bloating context. |
| **Direct reads only for confirmation** | Read a specific file:line only when you already know exactly what you need to confirm. ❌ Browsing/scanning files to "understand." |
| **Subagents for search** | Any `Grep`/`Glob` that might need >2 rounds → use `Task(Explore)` agent. It's designed for this. |
| **Teammates are disposable, context is not** | Spawn cheap subagents liberally. Your context window is expensive — don't fill it with raw code. Have subagents summarize findings. |
| **Research via agents, not yourself** | Before ANY implementation: spawn a research agent to gather context. Don't read the code yourself — get a summary back. |
| **Memory files exist — use them** | Check your auto-memory `MEMORY.md` (at `~/.claude/projects/<project-slug>/memory/`) before exploring. Past findings are already there. |
| **Always-on memory stays minimal** | Only universal invariants belong in always-loaded `CLAUDE.md` / unconditional `.claude/rules/*.md`. Specialized guidance → path-scoped rules, skills, or on-demand file reads. |

## Team Management
> Apply whenever TeamCreate is available (includes delegate/team mode).

**Model selection:** Default → `sonnet`. Escalate to `opus` ONLY for: deep multi-file investigation, complex architecture across modules, subtle race conditions, or when Sonnet produced insufficient results. ❌ `"inherit"` model — breaks Plan agents (they go idle without responding). Always specify explicit model.
**Verification rule:** When lower-tier models (Sonnet/Haiku) implement, verify with max-tier (Opus) before committing. ❌ Committing Sonnet work without Opus review.

| Rule | Detail |
|------|--------|
| **Always managed teams** | Every task → TeamCreate first. No standalone Task spawns. Even single-agent tasks use a team. |
| **Parallelization** | Multiple independent streams → separate teammates per stream. ❌ Serialize on one agent. |
| **Convergent investigation** | Bug investigation → ≥2 parallel agents (logs + code). Compare hypotheses before implementing. |
| **Incremental reporting (CRITICAL)** | Teammates MUST send progress updates to the lead via `SendMessage` after each significant finding or milestone — ❌ one big report at the end. Context windows expire; if a teammate dies mid-work, the lead loses everything unless incremental updates were sent. Rule of thumb: any finding worth remembering → send it to the lead immediately. |
| **Teammate reporting cadence** | At minimum: (1) after initial exploration/research, (2) after each root cause or key finding, (3) after implementation, (4) after tests pass. ❌ Silent for 10+ minutes then one final dump. |
| **Leads must request updates** | If a teammate has been idle or silent for >5 minutes, send a message asking for a progress update. Don't wait for the final report. |
| **Message timing** | Confirm all messages answered before shutdown. ❌ Send questions + shutdown in quick succession. |
| **TDD by default** | Tests FIRST. No "done" without pass/fail counts reported. |
| **Lead reviews coverage** | Review test gaps before approving commits. Every code change = tests. |
| **Report test results** | Teammates report pass/fail counts in completion messages. No "done" without test evidence. |
| **Every change = tests** | Code changes without corresponding test coverage are incomplete. |
| **Audit ALL code paths** | When fixing a guard, search ALL paths to same destination. ❌ Fix one, miss another. |
| **Shared safety helpers** | Extract guard logic to shared fn — ❌ duplicate across paths. |
| **Adversarial plan convergence (NON-NEGOTIABLE)** | See "Adversarial Plan Convergence" section below. Non-trivial plans MUST pass multi-round adversarial debate before implementation. |
| **Verify end-to-end** | After fix, verify user-visible behavior changed. Stale logs/UI can make working fixes look broken. |
| **Dual-spawn architecture** | Agent teams need BOTH in-process Task subagents (do actual work, write to sidechain JSONL) AND external CLI processes (registry workers, `approve_team_plan`). ❌ Remove either — both are required by design. See `src-tauri/manual_agent_teams_process.txt`. |
| **Sidechain output capture** | In-process Task subagents write to `~/.claude/projects/<slug>/<session>/subagents/agent-*.jsonl`, NOT to parent stdout. The lead's stream reader only sees parent stdout. If lead timeout (`team_line_read_secs`) kills the team, subagent work is lost even though the JSONL shows full conversations. |

## Agent Teams Architecture (CRITICAL — READ THIS)

RalphX agent teams use a **dual-spawn model**. Both components are required:

| Component | Purpose | Spawned By | Output |
|-----------|---------|------------|--------|
| In-process Task subagents | Do actual work (research, code, etc.) | Lead agent's `Task` tool | Sidechain JSONL (`~/.claude/projects/.../subagents/agent-*.jsonl`) |
| External CLI processes | Registry workers, `approve_team_plan`, message delivery | `tokio::process::Command` in backend | Stdout stream read by backend |

**Why both?** The Task tool creates in-process subagents that can use all Claude Code tools but write output to sidechain JSONL files (not parent stdout). The external CLI processes join the team registry and handle coordination tasks that need to be visible to the backend's stream reader.

**Known issue:** The lead's stream reader (`process_stream_background`) only monitors parent stdout. Sidechain subagent activity doesn't count as "activity" for the `team_line_read_secs` timeout (default 3600s). If subagents work for >1 hour without the lead producing stdout output, the lead gets killed, losing the ability to capture subagent results.

❌ NEVER remove the external CLI process spawning — it's not redundant, it's BY DESIGN
❌ NEVER treat "0 tokens" on external CLI processes as a bug — they may be registry workers
✅ Reference: `src-tauri/manual_agent_teams_process.txt` shows the manual equivalent
✅ Debug logs: `scripts/find-debug-logs.sh -s "session title"` to find agent debug files + conversation JSONLs

## MCP Architecture
Two MCP servers — different audiences. Full disambiguation: `.claude/rules/mcp-servers.md`
```
Internal: Claude Agent → stdio → ralphx-mcp-server → HTTP :3847 → Tauri Backend
External: Third-party bot → Bearer token → ralphx-external-mcp (:3848) → HTTP :3847 → Tauri Backend
```
Plugin: `claude --plugin-dir ./ralphx-plugin --agent worker -p "Execute"` | Tool config: `.claude/rules/agent-mcp-tools.md`
**MCP server build (NON-NEGOTIABLE):** After modifying ANY source in `ralphx-plugin/ralphx-mcp-server/src/` or `ralphx-plugin/ralphx-external-mcp/src/`, rebuild the respective server. ❌ Committing without rebuilding.
**mcp_tools override semantics (NON-NEGOTIABLE):** `extends` in `ralphx.yaml`: specifying `mcp_tools` fully replaces parent (no merge) — child must list ALL tools. Omitting `mcp_tools` inherits parent's list. ❌ Assuming partial inheritance when you specify the key.
**Agent frontmatter tool fields (NON-NEGOTIABLE):** Only `tools` and `disallowedTools` are valid in agent `.md` frontmatter. ❌ `allowedTools` — silently ignored by Claude Code. Add MCP tools (e.g., `"mcp__ralphx__*"`) to the `tools` list. Note: `--allowedTools` IS valid as a CLI flag at spawn time — only invalid as frontmatter.

| Agent | MCP Tools |
|-------|-----------|
| orchestrator-ideation | *_task_proposal, *_plan_artifact |
| chat-task | update_task, add_task_note, get_task_details |
| chat-project | suggest_task, list_tasks |
| worker | get_task_context, get_artifact*, *_step, execution_complete |
| coder | get_task_context, get_artifact*, *_step (❌ no execution_complete) |
| reviewer | complete_review, get_task_context |
| merger | report_conflict, report_incomplete, get_merge_target, get_task_context, complete_merge |

## Key Principles

| # | Rule |
|---|------|
| 1 | TDD mandatory — tests FIRST |
| 1.5 | **Orchestration chain tests** — see `src-tauri/CLAUDE.md` Integration Tests section |
| 2 | Anti-AI-slop — see Design System section |
| 3 | Clean architecture — domain has no infra deps |
| 4 | Type safety — strict TS, newtype IDs in Rust |
| 5 | ❌ Fragile string comparisons — use enum variants (`matches!(err, MyError::Variant)`), error codes, or named constants for external strings |
| 6 | Full timestamps in activity log |
| 7 | Status changes → TransitionHandler ONLY. ❌ Direct DB update |
| 8 | **Zero lint/test warnings (NON-NEGOTIABLE):** Fix ALL lint warnings and test failures before completing work — including pre-existing ones. ❌ "It's pre-existing" is not an excuse. Stale warnings delay future work and compound. `src-tauri/` → cargo clippy \| broad Rust runs → `cargo nextest run --manifest-path src-tauri/Cargo.toml --lib --profile ci` \| doctests → `cargo test --manifest-path src-tauri/Cargo.toml --doc` \| pinpoint Rust runs → see `.claude/rules/rust-test-execution.md`. ❌ `cargo check` \| ❌ full `cargo test` (hang) |
| 9 | ❌ Start/stop dev server — user manages manually |
| 10 | Implementation playbook: `DEVELOPMENT.md` — read alongside CLAUDE.md files for placement, naming, recipes, and debugging. |
| 11 | New pattern → add one-liner to relevant CLAUDE.md. Pattern name + rule only. |
| 12 | Complex work → TaskCreate/TaskUpdate/TaskList (MANDATORY) → `.claude/rules/task-management.md` |
| 13 | Parallel commits → acquire `.commit-lock` before, release after. Stale = same content >30s → `.claude/rules/commit-lock.md` |
| 14 | Tauri invoke: camelCase fields. ✅ `contextId` ❌ `context_id` |
| 15 | New `.claude/rules/*.md` \| `**/CLAUDE.md` → include this maintainer note at top |
| 16 | **DbConnection (NON-NEGOTIABLE):** All SQLite repo methods MUST use `db.run(\|conn\| { ... })` via `DbConnection` for non-blocking access. ❌ Direct `conn.lock().await` / `conn.query_row()` in async methods. See `db_connection.rs`. |
| 17 | **Tokio spawn safety (NON-NEGOTIABLE):** `tokio::spawn` / `tokio::task::spawn` / `spawn_blocking` → async context ONLY. Sync constructors & Tauri setup → `std::thread::spawn` or `tauri::async_runtime::spawn`. Details: `.claude/rules/tokio-runtime-safety.md` |
| 18 | **Rust std API stability (NON-NEGOTIABLE):** Avoid unstable std APIs in production code (e.g., `is_multiple_of`). Use stable equivalents (e.g., `%`). Details: `.claude/rules/rust-stable-apis.md` |
| 19 | **Constraint bundle planning** — Ideation plans should derive repo-specific `Constraints`, `Avoid`, and `Proof Obligations` from explored architecture before verification. |
| 20 | **Mechanical extractions only (NON-NEGOTIABLE):** For large refactors/splits, move existing code with real extraction commands/scripts first (`mv`, `sed`, `awk`, scripted extraction). `apply_patch` is only for the small post-move fix-up layer, never for hand-recreating large existing bodies. Details: `.claude/rules/code-quality-standards.md` |

## Adversarial Plan Convergence (NON-NEGOTIABLE)

> Applies to: team leads, ideation team leads (`ideation-team-lead`), solo ideation orchestrators (`orchestrator-ideation`), and any agent planning non-trivial changes.

Agent limitations mean no single plan can be trusted in full. Plans proposing code changes MUST pass adversarial debate as part of the VERIFY phase before implementation begins.

**How it works:** The existing VERIFY phase (Phase 3.5/4.5) now has two layers — plan completeness (Layer 1 critic) AND implementation feasibility (Layer 2 dual-lens critic). The agent decides which layers apply: plans proposing specific code changes, file modifications, or architectural modifications → both layers. High-level plans without implementation specifics → completeness only.

| Step | What |
|------|------|
| **Layer 2 (dual-lens critic)** | Single agent with both minimal/surgical AND comprehensive/defense-in-depth lenses. Reads actual code, finds functional gaps, rates CRITICAL/HIGH/MEDIUM/LOW, attributes gap source |
| **Synthesize** | Merge findings into revised plan addressing all CRITICAL and HIGH gaps |
| **Repeat** | New critic attacks revised plan each round until convergence |
| **Converge** | `zero_blocking`: ALL CRITICAL, HIGH, and MEDIUM gaps resolved. LOW may be deferred |
| **User confirmation gate** | ❌ Implement before user confirms converged plan |

**Adversarial agent rules:** Read actual code (not summaries). Concrete scenarios only ("if X then Y breaks at line Z"). ❌ Style/preference debates. Each gap: scenario + severity + blocks implementation?

Full process details: `ralphx-plugin/agents/ideation-team-lead.md` (Phase 4.5) | `ralphx-plugin/agents/orchestrator-ideation.md` (Phase 3.5)

## Design System
`specs/DESIGN.md` | Accent: `#ff6b35` (warm orange) ❌ purple/blue | Font: SF Pro ❌ Inter | **INVOKE `/tailwind-v4-shadcn` before UI work**

Input outline removal:
```tsx
className="outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0"
style={{ boxShadow: "none", outline: "none" }}
```

## Key Features
- **Active Plan** — Project-scoped plan filtering for Graph/Kanban. Docs: `docs/features/active-plan.md` | `docs/architecture/active-plan-api.md`
- **Session Recovery** — Expired Claude session recovery with history preservation. Docs: `docs/features/session-recovery.md`
- **Plan Verification** — Automated adversarial review loop for ideation plans. Docs: `docs/features/plan-verification.md`

## Git Conventions
❌ git init/push/remotes | Prefixes: `docs:` | `feat:` | `fix:` | `chore:` | Co-author: `Co-Authored-By: Claude <MODEL> <noreply@anthropic.com>`

## Screenshot Framing
Script: `scripts/frame-screenshots.py` | Assets: `assets/` | Add new entries to `SCREENSHOTS` list in script before running.

| Step | Rule |
|------|------|
| **Inspect source** | Check for macOS system bar (date/time/Wi-Fi/battery) at top of screenshot |
| **Crop if needed** | Script auto-crops via variance detection. Manual fallback: ~50px standard display \| ~100px Retina 2x |
| **Frame** | `python3 scripts/frame-screenshots.py --single <palette-key>` |
| **Verify output** | Confirm no system bar, no personal info in `assets/framed-*.png` |

## Misc
- DB: `sqlite3 src-tauri/ralphx.db "SELECT * FROM table_name;"`
- App logs: per-launch file — dev: `logs/ralphx_YYYY-MM-DD_HH-MM-SS.log` | prod: `~/Library/Application Support/com.ralphx.app/logs/` | latest: `ls -t logs/*.log | head -1` | config: `file_logging` in ralphx.yaml / `RALPHX_FILE_LOGGING` env (default: true)
- Debug logs: `scripts/find-debug-logs.sh -a "<agent-name>" -d "YYYY-MM-DD" -v` — find Claude debug logs by agent name/date/keywords
- Slash commands: `/activate-prd <path>` — switch PRD | `/create-prd` — PRD wizard
- Claude integration docs: `ai-docs/claude-code/README.md` — lightweight local index plus official-doc stubs; fetch official Claude Code docs when current vendor behavior matters
