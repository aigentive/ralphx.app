# CLAUDE.md

## Project: RalphX
Native Mac GUI for autonomous AI dev: Kanban, multi-agent orchestration, ideation chat.
Full spec: `specs/plan.md`

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
| Agent | MCP Tools |
|-------|-----------|
| orchestrator-ideation | *_task_proposal, *_plan_artifact |
| chat-task | update_task, add_task_note, get_task_details |
| chat-project | suggest_task, list_tasks |
| worker | get_task_context, get_artifact*, *_step |
| reviewer | complete_review |

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
7. **Lint before commit**: `cargo clippy --all-targets --all-features -- -D warnings` + `npm run lint`
8. **NEVER start/stop dev server** — User manages manually
9. **Proactive quality improvement (MANDATORY — NEVER SKIP)** — Every task requires `refactor:` commit. Read `logs/code-quality.md` first → pick ONE item by task scope → execute → mark done → commit. If list empty, launch Explore agent to replenish. See `.claude/rules/quality-improvement.md`
10. **Document patterns inline** — When introducing a new architectural pattern, add a one-liner to the relevant CLAUDE.md (`src/` or `src-tauri/`). Pattern name + rule only, not implementation lists.
11. **Task tools for complex work (MANDATORY)** — Use TaskCreate/TaskUpdate/TaskList for complex work. See `.claude/rules/task-management.md`

## Git Conventions
- NO: git init, push, remotes
- Commits: `docs:` | `feat:` | `fix:` | `chore:`
- Co-author: `Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>`

## Slash Commands
- `/activate-prd <path>` — Switch active PRD
- `/create-prd` — Interactive PRD wizard

## Database
- **Location**: `src-tauri/ralphx.db`
- **Query**: `sqlite3 src-tauri/ralphx.db "SELECT * FROM table_name;"`

## Claude Code Docs
`docs/claude-code/`: cli-reference.md, hooks.md, settings.md, sub-agents.md, plugins.md, skills.md
