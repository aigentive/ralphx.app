# CLAUDE.md (COMPACT)

## Project: RalphX
Native Mac GUI for autonomous AI dev: Kanban, multi-agent orchestration, ideation chat, extensible workflows (BMAD/GSD).
Full spec: `specs/plan.md` (9k+ lines)

## Structure
```
ralphx/
├─ ralph.sh, PROMPT.md          # Loop script + template
├─ src/                         # Frontend (React/TS) → src/CLAUDE.md
├─ src-tauri/                   # Backend (Rust/Tauri) → src-tauri/CLAUDE.md
│  └─ ralphx.db                 # SQLite database (dev mode)
├─ specs/
│  ├─ manifest.json             # Phase tracker (SOURCE OF TRUTH)
│  ├─ plan.md                   # Master spec
│  ├─ prd.md                    # Phase 0 PRD
│  └─ phases/prd_phase_*.md     # Phase PRDs (1-11)
├─ logs/activity.md             # Progress (git-tracked)
├─ .claude/settings.json        # Permissions
├─ ralphx-plugin/               # Claude plugin (agents/skills/hooks)
│  ├─ agents/*.md               # worker|reviewer|supervisor|orchestrator|deep-researcher|
│  │                            # qa-prep|qa-executor|orchestrator-ideation|chat-task|chat-project
│  └─ skills/*/SKILL.md
└─ ralphx-mcp-server/           # TS proxy → Tauri backend (port 3847)
```

## Plugin Usage
```bash
claude --plugin-dir ./ralphx-plugin --agent worker -p "Execute"
```
Agent tool scopes (via RALPHX_AGENT_TYPE env):
| Agent | MCP Tools |
|-------|-----------|
| orchestrator-ideation | create/update/delete_task_proposal, add_proposal_dependency, *_plan_artifact, link_proposals_to_plan |
| chat-task | update_task, add_task_note, get_task_details |
| chat-project | suggest_task, list_tasks |
| reviewer | complete_review |
| worker | get_task_context, get_artifact*, search_project_artifacts |
| supervisor/qa-* | None |

## MCP Architecture
```
Claude Agent → MCP Protocol → ralphx-mcp-server (TS)
                                    ↓ HTTP :3847
                              Tauri Backend (Rust logic)
```
Permission bridge: `permission_request` MCP tool → long-poll `/api/permission/await/:id` → UI dialog → resolve

## Manifest (specs/manifest.json)
```json
{ "currentPhase": N, "phases": [{ "phase": N, "prd": "path", "status": "active|pending|complete|paused|blocked" }] }
```
Auto-transition: all tasks `passes:true` → current→complete, next→active, increment currentPhase

## 12 Phases (0-11)
0:PRD-Gen | 1:Foundation | 2:DataLayer | 3:StateMachine | 4:AgenticClient | 5:FrontendCore
6:KanbanUI | 7:AgentSystem | 8:QASystem | 9:Review&Supervision | 10:Ideation | 11:Extensibility

## Loop (`./ralph.sh 200`)
```pseudocode
WHILE iterations < max:
  phase = manifest.phases.find(status=="active")
  prd = load(phase.prd)
  task = prd.tasks.find(passes==false)
  IF task:
    IF task.category=="planning": create_phase_prd()
    ELSE: TDD_implement()  # tests FIRST
    task.passes = true
    commit()
  ELIF all_complete:
    IF last_phase: output("<promise>COMPLETE</promise>")
    ELSE: transition_to_next_phase()
```

## Task JSON Format
```json
// Planning: { "category":"planning", "description":"...", "steps":[], "output":"path", "passes":false }
// Impl: { "category":"setup|feature|integration|testing", "description":"...", "steps":[], "passes":false }
```

## Activity Log (`logs/activity.md`)
Header: `**Last Updated:** YYYY-MM-DD HH:MM:SS | **Phase:** X | **Tasks:** N/M | **Current:** desc`
Entries: `### YYYY-MM-DD HH:MM:SS - Title\n**What:**\n- ...\n**Commands:**\n- \`...\``

## Design System → specs/DESIGN.md
- Accent: `#ff6b35` (warm orange) — NOT purple/blue
- Font: SF Pro — NOT Inter
- Shadows: layered depth — NOT flat
- 5% accent rule | Use shadcn/ui + Lucide icons
- **INVOKE `/tailwind-v4-shadcn` skill before UI work** (v4 ≠ v3)

### Removing Input Outlines (IMPORTANT)
Browser default focus outlines require BOTH Tailwind classes AND inline styles to fully remove:
```tsx
// Tailwind classes (all required):
className="outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0 focus:border-0"
// Inline styles (also required):
style={{ boxShadow: "none", outline: "none" }}
```
Reference: `src/components/Chat/ChatInput.tsx` textarea styling

## Git Conventions
- NO: git init, push, remotes
- Commit msgs: `docs:` (PRD) | `feat:` | `fix:` | `chore:` (phase transition)
- Co-author: `Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>`

## Key Principles
1. TDD mandatory (tests FIRST)
2. Anti-AI-slop (no purple gradients, no Inter)
3. Clean architecture (domain has no infra deps)
4. Type safety (strict TS, newtype IDs in Rust)
5. Atomic tasks (completable in one session)
6. Full timestamps in activity log
7. **USE TransitionHandler for status changes** — NEVER direct DB update
8. **Lint before commit**: `cargo clippy --all-targets --all-features -- -D warnings` + `npm run lint`
9. **NEVER start/stop dev server** — User manages dev server manually. Only touch it if explicitly asked.

## Slash Commands
- `/activate-prd <path>` — Switch active PRD (updates manifest, logs, commits)
- `/create-prd` — Interactive PRD wizard

## Database
- **Location**: `src-tauri/ralphx.db` (SQLite, dev mode)
- **Query**: `sqlite3 src-tauri/ralphx.db "SELECT * FROM table_name;"`
- **Key tables**: projects, tasks, ideation_sessions, task_proposals, chat_messages, artifacts, task_steps

## Claude Code Docs
`docs/claude-code/`: cli-reference.md, model-config.md, hooks.md, settings.md, sub-agents.md, plugins.md, skills.md
Models (4.5): opus→claude-opus-4-5-20251101 | sonnet→claude-sonnet-4-5-20250929 | haiku→claude-haiku-4-5-20251001
