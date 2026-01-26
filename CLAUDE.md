# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

This repository contains the **Ralph Wiggum Loop** - an autonomous development system that runs Claude iteratively with fresh context windows until all tasks are complete. It's being used to build **RalphX**.

## Project: RalphX

**RalphX** is a native Mac app that provides a GUI for autonomous AI-driven development with:
- Task management and Kanban boards
- Multi-agent orchestration (worker, reviewer, supervisor agents)
- Ideation system with chat interface and task proposals
- Extensible workflows and methodology support (BMAD, GSD)

The complete specification is in `specs/plan.md` (9,000+ lines).

---

## Directory Structure

```
ralphx/
├── ralph.sh                    # Main loop script
├── PROMPT.md                   # Template prompt for each iteration
├── CLAUDE.md                   # This file - project guidance
├── README.md                   # Project overview
│
├── src/                        # Frontend (React/TypeScript)
│   └── CLAUDE.md               # Frontend patterns & conventions
│
├── src-tauri/                  # Backend (Rust/Tauri)
│   └── CLAUDE.md               # Backend patterns & conventions
│
├── specs/                      # Specifications
│   ├── manifest.json           # Phase tracker (source of truth for active phase)
│   ├── plan.md                 # Master plan (comprehensive specification)
│   ├── prd.md                  # Phase 0 PRD (generates phase PRDs)
│   └── phases/                 # Phase-specific PRDs
│       ├── prd_phase_01_foundation.md
│       ├── prd_phase_02_data_layer.md
│       └── ... (11 phases)
│
├── logs/                       # Logs directory
│   ├── activity.md             # Human-readable progress log (tracked in git)
│   └── iteration_N.json        # Raw Claude output per iteration (gitignored)
│
├── .claude/                    # Claude Code configuration
│   ├── settings.json           # Permissions for autonomous operation
│   └── commands/               # Slash commands for this project
│       ├── create-prd.md       # PRD creation wizard
│       └── activate-prd.md     # Switch active PRD
│
├── ralphx-plugin/              # Claude Code plugin (agents, skills, hooks)
│   ├── .claude-plugin/
│   │   └── plugin.json         # Plugin manifest
│   ├── .mcp.json               # MCP server configuration (ralphx MCP server)
│   ├── agents/                 # All agent definitions
│   │   ├── worker.md
│   │   ├── reviewer.md
│   │   ├── supervisor.md
│   │   ├── orchestrator.md
│   │   ├── deep-researcher.md
│   │   ├── qa-prep.md
│   │   ├── qa-executor.md
│   │   ├── orchestrator-ideation.md
│   │   ├── chat-task.md        # Task-focused chat agent (Phase 15)
│   │   └── chat-project.md     # Project-focused chat agent (Phase 15)
│   ├── skills/                 # All skill definitions
│   │   ├── coding-standards/
│   │   ├── testing-patterns/
│   │   ├── code-review-checklist/
│   │   ├── research-methodology/
│   │   ├── git-workflow/
│   │   ├── acceptance-criteria-writing/
│   │   ├── qa-step-generation/
│   │   ├── qa-evaluation/
│   │   ├── task-decomposition/
│   │   ├── priority-assessment/
│   │   └── dependency-analysis/
│   └── hooks/                  # Plugin hooks
│       └── hooks.json
│
├── ralphx-mcp-server/          # MCP server (TypeScript proxy to Tauri backend)
│   ├── package.json            # MCP SDK dependency
│   ├── tsconfig.json           # TypeScript config
│   └── src/
│       ├── index.ts            # MCP server entry point with tool scoping
│       ├── tauri-client.ts     # HTTP client for Tauri backend
│       ├── tools.ts            # Tool definitions (proxied to backend)
│       └── permission-handler.ts # Permission request MCP tool
│
└── screenshots/                # Visual verification (via tauri-visual-test skill)
```

---

## Plugin Architecture

RalphX uses a **Claude Code plugin** to organize all agents, skills, and hooks. This consolidates agent definitions in one place for easier discovery and management.

### Plugin Structure

The plugin lives in `ralphx-plugin/` and uses folder-based auto-discovery:
- **Agents:** `ralphx-plugin/agents/*.md` - Agent definitions with frontmatter
- **Skills:** `ralphx-plugin/skills/*/SKILL.md` - Skill definitions in directories
- **Hooks:** `ralphx-plugin/hooks/hooks.json` - Hook configurations

### Using the Plugin

When spawning agents, use the `--plugin-dir` flag:

```bash
claude --plugin-dir ./ralphx-plugin --agent worker -p "Execute the task"
```

The Rust `ClaudeCodeClient` automatically adds `--plugin-dir ./ralphx-plugin` to all spawn calls.

### Available Agents

| Agent | Role | Description |
|-------|------|-------------|
| `worker` | Worker | Executes implementation tasks |
| `reviewer` | Reviewer | Reviews code changes (has complete_review MCP tool) |
| `supervisor` | Supervisor | Monitors task execution |
| `orchestrator` | Orchestrator | Coordinates multi-step tasks |
| `deep-researcher` | Researcher | Conducts thorough research |
| `qa-prep` | QA | Generates acceptance criteria |
| `qa-executor` | QA | Executes browser tests |
| `orchestrator-ideation` | Ideation | Facilitates brainstorming (has ideation MCP tools) |
| `chat-task` | Chat | Task-focused chat (has task MCP tools) |
| `chat-project` | Chat | Project-focused chat (has project MCP tools) |

### Available Skills

| Skill | Used By | Purpose |
|-------|---------|---------|
| `coding-standards` | worker | Code quality guidelines |
| `testing-patterns` | worker | TDD patterns |
| `git-workflow` | worker | Git conventions |
| `code-review-checklist` | reviewer | Review criteria |
| `research-methodology` | deep-researcher | Research approach |
| `acceptance-criteria-writing` | qa-prep | AC generation |
| `qa-step-generation` | qa-prep | Test step creation |
| `qa-evaluation` | qa-executor | Test evaluation |
| `task-decomposition` | orchestrator-ideation | Task breakdown |
| `priority-assessment` | orchestrator-ideation | Priority scoring |
| `dependency-analysis` | orchestrator-ideation | Dependency mapping |

---

## MCP Integration (Phase 15)

RalphX uses the **Model Context Protocol (MCP)** to expose custom tools to Claude agents during chat interactions.

### Architecture

```
Claude Agent (orchestrator-ideation, chat-task, etc.)
    ↓ MCP Protocol
RalphX MCP Server (ralphx-mcp-server/)
    ↓ HTTP (localhost:3847)
Tauri Backend (existing business logic)
```

**Key design decisions:**
- **MCP server is a TypeScript proxy** - No business logic, just forwards to Tauri backend
- **All business logic stays in Rust** - MCP server calls HTTP endpoints on port 3847
- **Tool scoping via RALPHX_AGENT_TYPE** - Each agent only sees tools appropriate for its role
- **Permission bridge** - UI-based approval for non-pre-approved tools via MCP `permission_request` tool

### MCP Server

Located in `ralphx-mcp-server/`, the MCP server:
- Reads `RALPHX_AGENT_TYPE` env var (set by Tauri when spawning Claude CLI)
- Filters available tools based on agent type
- Forwards all tool calls to Tauri backend HTTP endpoints
- Implements `permission_request` MCP tool for UI-based approval flow

### Tool Scoping

| Agent Type | Tools Available |
|------------|-----------------|
| `orchestrator-ideation` | create_task_proposal, update_task_proposal, delete_task_proposal, add_proposal_dependency |
| `chat-task` | update_task, add_task_note, get_task_details |
| `chat-project` | suggest_task, list_tasks |
| `reviewer` | complete_review (submit review decision: approved/needs_changes/escalate) |
| `worker`, `supervisor`, `qa-prep`, `qa-tester` | None |

This ensures agents can only perform actions appropriate to their role (e.g., worker cannot create proposals).

### HTTP Server (Port 3847)

The Tauri backend runs an Axum HTTP server on port 3847 that exposes:
- MCP tool endpoints (POST /api/create_task_proposal, etc.)
- Permission bridge endpoints (POST /api/permission/request, GET /api/permission/await/:id, POST /api/permission/resolve)

All endpoints reuse existing service logic - no duplication.

### Permission Bridge

For tools not pre-approved in `.claude/settings.json`:
1. Agent calls `permission_request` MCP tool with tool name and arguments
2. MCP server POSTs to `/api/permission/request` → Tauri backend emits `permission:request` event
3. Frontend shows PermissionDialog with tool details
4. MCP server long-polls `/api/permission/await/:request_id` (5 min timeout)
5. User clicks Allow/Deny → frontend calls `resolve_permission_request` Tauri command
6. Backend signals waiting MCP request → returns decision to Claude
7. Claude continues (allow) or stops (deny)

This enables secure, UI-based approval for powerful tools like Write, Edit, Bash without pre-approving them globally.

---

## Codebase Documentation (Progressive Discovery)

For detailed context on tech stack, patterns, and conventions, see the dedicated CLAUDE.md files:

| File | Contents |
|------|----------|
| [`src/CLAUDE.md`](src/CLAUDE.md) | **Frontend** - React 19, TypeScript 5.8, Zustand stores, TanStack Query, Tailwind CSS, component patterns, testing with Vitest |
| [`src-tauri/CLAUDE.md`](src-tauri/CLAUDE.md) | **Backend** - Rust, Tauri 2.0, SQLite, clean architecture, repository pattern, newtype IDs, state machine, agent system |
| [`specs/DESIGN.md`](specs/DESIGN.md) | **Design System** - Color tokens, typography, spacing, shadows, component patterns, anti-AI-slop guardrails, page-specific requirements |

These files contain:
- Complete tech stack with versions
- Directory structure and organization
- Key patterns with code examples
- Coding standards and conventions
- Testing approaches and commands

---

## Design System

**All UI work must follow `specs/DESIGN.md`** - the definitive design guide for RalphX.

### Required Skill for UI/UX Work

**IMPORTANT:** When working on any UI/UX task (components, styling, layouts, theming, CSS), you MUST first invoke the `tailwind-v4-shadcn` skill:

```
/tailwind-v4-shadcn
```

This skill provides production-tested patterns for:
- Tailwind CSS v4 configuration (different from v3)
- shadcn/ui component setup and theming
- CSS variable architecture with `@theme inline`
- Dark mode implementation
- Common gotchas and fixes

**Why this matters:** Tailwind v4 has breaking changes from v3. Without this skill, you may use outdated patterns that won't work (e.g., `tailwind.config.js` is ignored in v4).

### Key Principles

- **Warm orange accent** (`#ff6b35`) - NOT purple/blue gradients
- **SF Pro font** - NOT Inter
- **Layered shadows** for depth - NOT flat surfaces
- **5% accent rule** - use sparingly for maximum impact
- **Use shadcn/ui** components from `src/components/ui/`
- **Use Lucide icons** - NOT inline SVGs

Read `specs/DESIGN.md` before any UI task.

---

## Manifest System

The `specs/manifest.json` is the **source of truth** for which phase is active. It enables automatic phase transitions without manual intervention.

### Manifest Structure

```json
{
  "project": "RalphX",
  "masterPlan": "specs/plan.md",
  "currentPhase": 0,
  "phases": [
    {
      "phase": 0,
      "name": "PRD Generation",
      "prd": "specs/prd.md",
      "status": "active",
      "description": "Generate phase-specific PRDs from master plan"
    },
    {
      "phase": 1,
      "name": "Foundation",
      "prd": "specs/phases/prd_phase_01_foundation.md",
      "status": "pending",
      "description": "Project setup, Tauri, React, TypeScript, basic types"
    },
    // ... phases 2-11
  ]
}
```

### Phase Status Values

| Status | Meaning |
|--------|---------|
| `"active"` | Currently being worked on |
| `"pending"` | Not yet started |
| `"complete"` | All tasks finished |
| `"paused"` | Temporarily stopped (via `/activate-prd`) |
| `"blocked"` | Waiting on another phase (via `/activate-prd`) |

### Automatic Transitions

When all tasks in the active PRD have `"passes": true`:
1. Current phase's status → `"complete"`
2. `currentPhase` increments
3. Next phase's status → `"active"`
4. Commit manifest changes
5. Continue with new active PRD

---

## Phase Structure

RalphX is built in 12 phases (0-11):

| Phase | Name | PRD File | Focus |
|-------|------|----------|-------|
| 0 | PRD Generation | `specs/prd.md` | Generate phase-specific PRDs from master plan |
| 1 | Foundation | `prd_phase_01_foundation.md` | Tauri, React, TypeScript, basic types |
| 2 | Data Layer | `prd_phase_02_data_layer.md` | Repository pattern, SQLite, migrations |
| 3 | State Machine | `prd_phase_03_state_machine.md` | statig, 14 internal statuses, transitions |
| 4 | Agentic Client | `prd_phase_04_agentic_client.md` | Agent abstraction, Claude Code client |
| 5 | Frontend Core | `prd_phase_05_frontend_core.md` | Zustand stores, Tauri bindings, events |
| 6 | Kanban UI | `prd_phase_06_kanban_ui.md` | TaskBoard, columns, drag-drop |
| 7 | Agent System | `prd_phase_07_agent_system.md` | Worker, reviewer, supervisor agents |
| 8 | QA System | `prd_phase_08_qa_system.md` | QA prep/testing, visual verification |
| 9 | Review & Supervision | `prd_phase_09_review_supervision.md` | Review workflow, watchdog, human-in-loop |
| 10 | Ideation | `prd_phase_10_ideation.md` | Chat interface, proposals, priority |
| 11 | Extensibility | `prd_phase_11_extensibility.md` | Workflows, methodologies, artifacts |

---

## Running the Loop

```bash
# Run until all phases complete (or max iterations reached)
./ralph.sh 200

# Example progression:
# Iterations 1-11:  Phase 0 - Creates 11 phase PRDs
# Iteration 12:     Auto-transition to Phase 1
# Iterations 12-N:  Phase 1 - Foundation implementation
# ...continues automatically through all phases
```

**Requirements:** `claude` CLI, `jq`, bash

**Termination:** Loop ends when:
- Claude outputs `<promise>COMPLETE</promise>` (all phases done), OR
- Max iterations reached

---

## Task List Format

### Planning Tasks (Phase 0)

```json
{
  "category": "planning",
  "description": "Create Phase 1 PRD: Foundation",
  "steps": [
    "Read specs/plan.md sections: Project Overview, Tech Stack, Architecture...",
    "Extract all foundation requirements...",
    "Create atomic tasks with TDD requirements...",
    "Write PRD to specs/phases/prd_phase_01_foundation.md",
    "Verify PRD tasks align with master plan"
  ],
  "output": "specs/phases/prd_phase_01_foundation.md",
  "passes": false
}
```

### Implementation Tasks (Phases 1-11)

```json
{
  "category": "setup|feature|integration|testing",
  "description": "Implement TaskRepository trait",
  "steps": [
    "Write unit tests for repository methods",
    "Implement trait with SQLite backend",
    "Run cargo test to verify"
  ],
  "passes": false
}
```

---

## Activity Log Format

The `logs/activity.md` has two parts that must be kept in sync:

### Header Section (updated after each task)

```markdown
## Current Status
**Last Updated:** YYYY-MM-DD HH:MM:SS
**Phase:** [Current phase name]
**Tasks Completed:** X / Y
**Current Task:** [Next incomplete task, or "All complete"]
```

### Log Entries (appended after each task)

All entries use full datetime stamps:

```markdown
### YYYY-MM-DD HH:MM:SS - [Title]

**What was done:**
- Item 1
- Item 2

**Commands run:**
- `command 1`
- `command 2`
```

Example:
```markdown
### 2026-01-24 05:15:00 - Project Setup

**What was done:**
- Created specs/ directory structure
- Created manifest.json for phase tracking
```

---

## Iteration Workflow

### Step 1: Determine Active PRD
Read `specs/manifest.json` → find phase with `"status": "active"` → load that PRD

### Step 2: Check for Tasks
- If tasks with `"passes": false` exist → work on next task
- If all tasks complete → handle phase transition

### Step 3: Execute Task

**For Planning Tasks (`category: "planning"`):**
1. Read relevant sections from `specs/plan.md`
2. Create phase PRD with atomic tasks
3. Verify against master plan
4. Update `"passes": true`
5. Log with full timestamp
6. Commit: `git commit -m "docs: create Phase N PRD - [name]"`

**For Implementation Tasks:**
1. Write tests FIRST (TDD mandatory)
2. Implement to make tests pass
3. Run linting/type checks
4. Update `"passes": true`
5. Log with full timestamp
6. Commit: `git commit -m "feat: [description]"`

### Step 4: Phase Transition (when PRD complete)
1. Update manifest: current phase → `"complete"`, next phase → `"active"`
2. Log phase completion
3. Commit manifest changes
4. Continue with new active PRD (or output `<promise>COMPLETE</promise>` if done)

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `ralph.sh` | Main loop - invokes `claude -p`, parses for completion signal |
| `PROMPT.md` | Template prompt - references manifest, handles both task types |
| `specs/manifest.json` | **Phase tracker** - source of truth for active phase |
| `specs/plan.md` | **Master plan** - complete RalphX specification (9,000+ lines) |
| `specs/prd.md` | Phase 0 PRD - tasks to generate phase-specific PRDs |
| `specs/phases/*.md` | Phase PRDs - generated from master plan |
| `logs/activity.md` | Progress log - timestamped entries (tracked in git) |
| `logs/iteration_*.json` | Raw Claude output (gitignored) |
| `.claude/settings.json` | Permissions for autonomous operation |

---

## Important Principles

From the master plan:

1. **TDD is mandatory** - Write tests before implementation, always
2. **Anti-AI-slop** - No purple gradients, no Inter font, no generic icon grids
3. **Clean architecture** - Domain layer has no infrastructure dependencies
4. **Type safety** - Strict TypeScript, newtype pattern in Rust
5. **Preserve details** - When creating PRDs, don't summarize - keep ALL specifics
6. **Atomic tasks** - Each task completable in one focused session
7. **Full timestamps** - Activity log entries use `YYYY-MM-DD HH:MM:SS` format
8. **Use TransitionHandler for task status changes** - NEVER update task status directly in the database. Always use `TransitionHandler` from `domain/state_machine/` to ensure entry actions (spawn workers, start reviews, emit events) are triggered. See `src-tauri/CLAUDE.md` for detailed architecture.

---

## Slash Commands

### `/activate-prd <path>`

Manually switch the active PRD. Useful for:
- Emergency course corrections
- Inserting a new phase
- Skipping ahead or going back
- Parallel work on different phases

**Usage:**
```
/activate-prd specs/phases/prd_phase_03_state_machine.md
```

**What it does:**
1. Assesses current state (active phase, tasks complete/remaining)
2. Asks what to do with currently active PRD:
   - Mark as complete
   - Mark as paused
   - Mark as blocked
   - Keep as-is
3. Asks how to set up target PRD:
   - Activate from beginning
   - Reset and activate
   - Insert as new phase (if not in manifest)
4. Updates `specs/manifest.json`
5. Logs the change with full timestamp
6. Commits the changes

### `/create-prd`

Interactive PRD creation wizard. Gathers requirements and generates a PRD with JSON task list.

---

## Claude Code CLI Reference

When working with Claude Code CLI features (agents, hooks, skills, plugins, model configuration), **always check the official documentation** in `docs/claude-code/`:

| File | Contents |
|------|----------|
| `index.txt` | Full documentation index with all available pages |
| `cli-reference.md` | CLI commands and flags (--model, --output-format, etc.) |
| `model-config.md` | Model aliases (opus, sonnet, haiku), environment variables |
| `hooks.md` | Hook events, configuration, input/output schemas |
| `settings.md` | Permissions, settings files, environment variables |
| `sub-agents.md` | Creating custom subagents |
| `plugins.md` | Plugin structure and configuration |
| `skills.md` | Skill definitions and usage |
| `headless.md` | Programmatic/SDK usage |

**Current model versions (4.5):**
- `opus` → Opus 4.5 (`claude-opus-4-5-20251101`)
- `sonnet` → Sonnet 4.5 (`claude-sonnet-4-5-20250929`)
- `haiku` → Haiku 4.5 (`claude-haiku-4-5-20251001`)

**Source:** https://code.claude.com/docs/llms.txt

---

## Git Conventions

- **Do NOT** run `git init`, change remotes, or push
- **Do** commit after each completed task
- **Commit messages:**
  - Planning: `docs: create Phase N PRD - [phase name]`
  - Features: `feat: [description]`
  - Fixes: `fix: [description]`
  - Phase transitions: `chore: complete phase N, activate phase N+1`
