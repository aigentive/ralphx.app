# CLAUDE.md

## Project: RalphX
Native Mac GUI for autonomous AI dev: Kanban, multi-agent orchestration, ideation chat.
Full spec: `specs/plan.md` | Code quality: @.claude/rules/code-quality-standards.md
Task detail views (Kanban UI): @.claude/rules/task-detail-views.md
Git modes, state machine, agents, merges: @.claude/rules/task-execution-git-workflows.md

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
Adding/modifying MCP tools for agents: @.claude/rules/agent-mcp-tools.md (three-layer allowlist — all required)

| Agent | MCP Tools |
|-------|-----------|
| orchestrator-ideation | *_task_proposal, *_plan_artifact |
| chat-task | update_task, add_task_note, get_task_details |
| chat-project | suggest_task, list_tasks |
| worker | get_task_context, get_artifact*, *_step |
| reviewer | complete_review |
| merger | complete_merge, report_conflict, report_incomplete, get_merge_target, get_task_context |

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

## Key Principles
1. TDD mandatory (tests FIRST)
2. Anti-AI-slop (no purple gradients, no Inter)
3. Clean architecture (domain has no infra deps)
4. Type safety (strict TS, newtype IDs in Rust)
5. Full timestamps in activity log
6. **USE TransitionHandler for status changes** — NEVER direct DB update
7. **Lint before commit** (only for what you modified): `src-tauri/` → cargo clippy, `src/` → npm run lint
8. **NEVER start/stop dev server** — User manages manually
9. **Multi-stream workflow** — Use `./ralph-streams.sh <stream>` for focused work: features (PRD+P0), refactor (P1), polish (P2/P3), verify (gaps), hygiene (backlog maintenance). See `.claude/rules/stream-*.md`
10. **Document patterns inline** — When introducing a new architectural pattern, add a one-liner to the relevant CLAUDE.md (`src/` or `src-tauri/`). Pattern name + rule only, not implementation lists.
11. **Task tools for complex work (MANDATORY)** — Use TaskCreate/TaskUpdate/TaskList for complex work. See `.claude/rules/task-management.md`
12. **Commit lock for parallel work** — Acquire `.commit-lock` before committing, release after. Stale = same content >30s. See `.claude/rules/commit-lock.md`
13. **LLM-optimized docs** — When creating `.claude/rules/*.md` or `**/CLAUDE.md` files, include this maintainer note at the top (after Required Context if present):
    ```
    > **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.
    ```

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
