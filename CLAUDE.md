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
│   └── commands/
│       └── create-prd.md       # PRD creation wizard
│
└── screenshots/                # Visual verification (if agent-browser used)
```

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
