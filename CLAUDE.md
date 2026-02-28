> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# CLAUDE.md

## Priority Zero — Owner Strategy Alignment (NON-NEGOTIABLE)

Before ANY user-facing content, documentation, UI copy, or messaging work, agents MUST read:
- `@~/.ralphx/founder/founder-profile.md` — Owner vision and non-negotiables
- `@~/.ralphx/strategy/project-goal-card.md` — Messaging architecture, positioning, ICPs, competitive landscape
- `@~/.ralphx/strategy/project-metrics.md` — Verifiable project data points

These are the **owner's directives**. They override default agent judgment on messaging.

---

## Project: RalphX
Native Mac GUI for autonomous AI dev: Kanban, multi-agent orchestration, ideation chat.
Full spec: `specs/plan.md` | Code quality: `.claude/rules/code-quality-standards.md` | State machine: `.claude/rules/task-state-machine.md` | Git/merge: `.claude/rules/task-git-branching.md` | Agents: `.claude/rules/task-execution-agents.md` | Agent type map: `.claude/rules/agent-type-map.md` | Task detail views: `.claude/rules/task-detail-views.md`

## Structure
```
ralphx/
├─ src/                   # Frontend (React/TS) → src/CLAUDE.md
├─ src-tauri/             # Backend (Rust/Tauri) → src-tauri/CLAUDE.md
│  └─ ralphx.db           # SQLite (dev)
├─ ralphx-plugin/         # Claude plugin (agents/skills/hooks)
└─ ralphx-mcp-server/     # TS proxy → Tauri :3847
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

## Team Management
> Apply whenever TeamCreate is available.

**Model selection:** Default → `sonnet`. Escalate to `opus` ONLY for: deep multi-file investigation, complex architecture across modules, subtle race conditions, or when Sonnet produced insufficient results.
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
| **Audit ALL code paths** | When fixing a guard, search ALL paths to same destination. ❌ Fix one, miss another. |
| **Shared safety helpers** | Extract guard logic to shared fn — ❌ duplicate across paths. |
| **Debate before implementing** | Non-trivial fixes → spawn Alpha (minimal) vs Beta (comprehensive). |
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
```
Claude Agent → MCP Protocol → ralphx-mcp-server (TS) → HTTP :3847 → Tauri Backend
```
Plugin: `claude --plugin-dir ./ralphx-plugin --agent worker -p "Execute"` | Tool config: `.claude/rules/agent-mcp-tools.md` (three-layer allowlist)
**MCP server build (NON-NEGOTIABLE):** After modifying ANY source in `ralphx-plugin/ralphx-mcp-server/src/`, run `cd ralphx-plugin/ralphx-mcp-server && npm run build` to rebuild assets. ❌ Committing without rebuilding.

| Agent | MCP Tools |
|-------|-----------|
| orchestrator-ideation | *_task_proposal, *_plan_artifact |
| chat-task | update_task, add_task_note, get_task_details |
| chat-project | suggest_task, list_tasks |
| worker / coder | get_task_context, get_artifact*, *_step, execution_complete |
| reviewer | complete_review |
| merger | report_conflict, report_incomplete, get_merge_target, get_task_context |

## Key Principles

| # | Rule |
|---|------|
| 1 | TDD mandatory — tests FIRST |
| 1.5 | **Orchestration chain tests** — real git + real DB + MockChatService. Mock agent spawning only → verify `call_count()` & `ChatContextType::Merge`. |
| 2 | Anti-AI-slop — ❌ purple gradients, ❌ Inter font |
| 3 | Clean architecture — domain has no infra deps |
| 4 | Type safety — strict TS, newtype IDs in Rust |
| 5 | ❌ Fragile string comparisons — use enum variants (`matches!(err, MyError::Variant)`), error codes, or named constants for external strings |
| 6 | Full timestamps in activity log |
| 7 | Status changes → TransitionHandler ONLY. ❌ Direct DB update |
| 8 | **Zero lint/test warnings (NON-NEGOTIABLE):** Fix ALL lint warnings and test failures before completing work — including pre-existing ones. ❌ "It's pre-existing" is not an excuse. Stale warnings delay future work and compound. `src-tauri/` → cargo clippy \| `src/` → npm run lint. Tests: `timeout 10m cargo test --lib --manifest-path src-tauri/Cargo.toml 2>&1 \| tail -40`. ❌ `cargo check` \| ❌ full `cargo test` (hang) |
| 9 | ❌ Start/stop dev server — user manages manually |
| 10 | Multi-stream: `./ralph-streams.sh <stream>` (features/refactor/polish/verify/hygiene) → `.claude/rules/stream-*.md` |
| 11 | New pattern → add one-liner to relevant CLAUDE.md. Pattern name + rule only. |
| 12 | Complex work → TaskCreate/TaskUpdate/TaskList (MANDATORY) → `.claude/rules/task-management.md` |
| 13 | Parallel commits → acquire `.commit-lock` before, release after. Stale = same content >30s → `.claude/rules/commit-lock.md` |
| 14 | Tauri invoke: camelCase fields. ✅ `contextId` ❌ `context_id` |
| 15 | New `.claude/rules/*.md` \| `**/CLAUDE.md` → include this maintainer note at top |

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

## Team Mode Rules
When delegate mode is active (TeamCreate tool available):

| Rule | Detail |
|------|--------|
| **Always managed teams** | Every agent task MUST be wrapped in a team (TeamCreate first). No standalone Task tool spawns — user needs visibility. Even single-agent tasks use a team. |
| **TDD by default** | Teammates tasked with execution write tests FIRST, or at minimum verify test coverage exists before marking complete. |
| **Lead reviews coverage** | Team lead instructs teammates to check/implement test coverage. Review for gaps before approving commits. |
| **Report test results** | Teammates report pass/fail counts in completion messages. No "done" without test evidence. |
| **Every change = tests** | Code changes without corresponding test coverage are incomplete. |
| **Audit ALL code paths** | When fixing a bypass/guard, search for ALL code paths that reach the same destination. Fixing one path while missing another is a common regression (e.g., check_already_merged vs recover_deleted_source_branch). |
| **Shared safety helpers** | Never duplicate safety/guard logic across code paths. Extract to a shared function so all paths use the same check. |
| **Debate before implementing** | For non-trivial fixes, spawn Alpha (minimal) vs Beta (comprehensive) debate agents. This catches edge cases that single-agent implementation misses. |
| **Verify end-to-end** | After a fix, verify the user-visible behavior changed, not just the code. Stale logs/UI can make a working fix appear broken. |

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
- Claude Code docs: `docs/claude-code/`: cli-reference.md, hooks.md, settings.md, sub-agents.md, plugins.md, skills.md
