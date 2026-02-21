# CLAUDE.md

## Project: RalphX
Native Mac GUI for autonomous AI dev: Kanban, multi-agent orchestration, ideation chat.
Full spec: `specs/plan.md` | Code quality: `.claude/rules/code-quality-standards.md`
Task detail views (Kanban UI): `.claude/rules/task-detail-views.md`
State machine: `.claude/rules/task-state-machine.md` | Git/merge: `.claude/rules/task-git-branching.md` | Agents: `.claude/rules/task-execution-agents.md`

## Structure
```
ralphx/
├─ src/                   # Frontend (React/TS) → src/CLAUDE.md
├─ src-tauri/             # Backend (Rust/Tauri) → src-tauri/CLAUDE.md
│  └─ ralphx.db           # SQLite database (dev)
├─ specs/
│  ├─ manifest.json       # Phase tracker (SOURCE OF TRUTH)
│  ├─ plan.md             # Master spec
│  └─ phases/prd_*.md     # Phase PRDs
├─ logs/activity.md       # Progress log
├─ ralphx-plugin/         # Claude plugin (agents/skills/hooks)
└─ ralphx-mcp-server/     # TS proxy → Tauri :3847
```

## MCP Architecture
```
Claude Agent → MCP Protocol → ralphx-mcp-server (TS) → HTTP :3847 → Tauri Backend
```
Plugin: `claude --plugin-dir ./ralphx-plugin --agent worker -p "Execute"`

## Agent Tool Scopes
Adding/modifying MCP tools for agents: `.claude/rules/agent-mcp-tools.md` (three-layer allowlist — all required)

| Agent | MCP Tools |
|-------|-----------|
| orchestrator-ideation | *_task_proposal, *_plan_artifact |
| chat-task | update_task, add_task_note, get_task_details |
| chat-project | suggest_task, list_tasks |
| worker | get_task_context, get_artifact*, *_step |
| coder | get_task_context, get_artifact*, *_step |
| reviewer | complete_review |
| merger | report_conflict, report_incomplete, get_merge_target, get_task_context |

## Manifest Format
```json
{ "currentPhase": N, "phases": [{ "phase": N, "prd": "path", "status": "active|pending|complete" }] }
```

## Activity Log Format
```markdown
**Last Updated:** YYYY-MM-DD HH:MM:SS | **Phase:** X | **Tasks:** N/M
### YYYY-MM-DD HH:MM:SS - Title
**What:** - bullets  **Commands:** - `commands`
```

## Design System
See `specs/DESIGN.md`
- Accent: `#ff6b35` (warm orange) — NOT purple/blue
- Font: SF Pro — NOT Inter
- **INVOKE `/tailwind-v4-shadcn` before UI work**

### Input Outline Removal Pattern
```tsx
className="outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0"
style={{ boxShadow: "none", outline: "none" }}
```

## Key Features
- **Active Plan** — Project-scoped plan filtering for Graph/Kanban. User docs: `docs/features/active-plan.md` | API docs: `docs/architecture/active-plan-api.md`
- **Session Recovery** — Automatic recovery of expired Claude sessions with conversation history preservation. User docs: `docs/features/session-recovery.md`

## Key Principles
1. TDD mandatory (tests FIRST)
2. Anti-AI-slop (no purple gradients, no Inter)
3. Clean architecture (domain has no infra deps)
4. Type safety (strict TS, newtype IDs in Rust)
5. **No fragile string comparisons** — NEVER compare error strings with `==` or `.contains()`. Use enum variants (`matches!(err, MyError::Variant)`), error codes, or shared constants. If parsing external/uncontrolled strings (CLI stderr), extract match strings to named constants with doc comments noting the source.
6. Full timestamps in activity log
7. **USE TransitionHandler for status changes** — NEVER direct DB update
8. **Lint before commit** (only for what you modified): `src-tauri/` → cargo clippy, `src/` → npm run lint. When using Claude/automation: run **only** `cargo test --lib` and **do not run** `cargo check` or full `cargo test` (they hang). `cargo test --lib` can take 5–8+ min — use **10 min timeout**: `timeout 10m cargo test --lib --manifest-path src-tauri/Cargo.toml 2>&1 | tail -40`, or a focused test (e.g. `cargo test --lib module_name`). No `--nocapture`/verbose.
9. **NEVER start/stop dev server** — User manages manually
10. **Multi-stream workflow** — Use `./ralph-streams.sh <stream>` for focused work: features (PRD+P0), refactor (P1), polish (P2/P3), verify (gaps), hygiene (backlog maintenance). See `.claude/rules/stream-*.md`
11. **Document patterns inline** — When introducing a new architectural pattern, add a one-liner to the relevant CLAUDE.md (`src/` or `src-tauri/`). Pattern name + rule only, not implementation lists.
12. **Task tools for complex work (MANDATORY)** — Use TaskCreate/TaskUpdate/TaskList for complex work. See `.claude/rules/task-management.md`
13. **Commit lock for parallel work** — Acquire `.commit-lock` before committing, release after. Stale = same content >30s. See `.claude/rules/commit-lock.md`
14. **Tauri invoke uses camelCase** — Rust command input structs use `#[serde(rename_all = "camelCase")]`. Frontend `invoke()` calls MUST pass camelCase field names (`contextId`, `sessionId`), NOT snake_case (`context_id`, `session_id`).
15. **LLM-optimized docs** — When creating `.claude/rules/*.md` or `**/CLAUDE.md` files, include this maintainer note at the top (after Required Context if present):
    ```
    > **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.
    ```

## Team Mode Rules
When delegate mode is active (TeamCreate tool available):

| Rule | Detail |
|------|--------|
| **Always managed teams** | Every agent task MUST be wrapped in a team (TeamCreate first). No standalone Task tool spawns — user needs visibility. Even single-agent tasks use a team. |
| **TDD by default** | Teammates tasked with execution write tests FIRST, or at minimum verify test coverage exists before marking complete. |
| **Lead reviews coverage** | Team lead instructs teammates to check/implement test coverage. Review for gaps before approving commits. |
| **Report test results** | Teammates report pass/fail counts in completion messages. No "done" without test evidence. |
| **Every change = tests** | Code changes without corresponding test coverage are incomplete. |

## Git Conventions
- NO: git init, push, remotes
- Commits: `docs:` | `feat:` | `fix:` | `chore:`
- Co-author: `Co-Authored-By: Claude <MODEL> <noreply@anthropic.com>` — substitute your actual model name (e.g. Opus 4.6, Sonnet 4.5)

## Slash Commands
- `/activate-prd <path>` — Switch active PRD
- `/create-prd` — Interactive PRD wizard

## Database
- **Location**: `src-tauri/ralphx.db`
- **Query**: `sqlite3 src-tauri/ralphx.db "SELECT * FROM table_name;"`

## Claude Code Docs
`docs/claude-code/`: cli-reference.md, hooks.md, settings.md, sub-agents.md, plugins.md, skills.md
