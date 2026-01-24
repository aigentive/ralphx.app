# RalphX - Phase PRD Generation

## Overview

This PRD orchestrates the creation of detailed, phase-specific PRDs from the master plan (`@specs/plan.md`). Each phase PRD will contain atomic tasks that can be executed by the Ralph loop.

**Master Plan Reference:** @specs/plan.md

## Phase Structure

The RalphX implementation is divided into 11 phases, each producing a dedicated PRD in `specs/phases/`:

| Phase | Name | PRD File | Focus |
|-------|------|----------|-------|
| 1 | Foundation | `prd_phase_01_foundation.md` | Project setup, Tauri, basic types |
| 2 | Data Layer | `prd_phase_02_data_layer.md` | Repository pattern, SQLite, migrations |
| 3 | State Machine | `prd_phase_03_state_machine.md` | statig, transitions, side effects |
| 4 | Agentic Client | `prd_phase_04_agentic_client.md` | Agent abstraction, Claude client |
| 5 | Frontend Core | `prd_phase_05_frontend_core.md` | React, Zustand, Tauri bindings |
| 6 | Kanban UI | `prd_phase_06_kanban_ui.md` | TaskBoard, drag-drop, cards |
| 7 | Agent System | `prd_phase_07_agent_system.md` | Agents, skills, hooks, plugin |
| 8 | QA System | `prd_phase_08_qa_system.md` | QA prep/testing, visual verification |
| 9 | Review & Supervision | `prd_phase_09_review_supervision.md` | Review, watchdog, human-in-loop |
| 10 | Ideation System | `prd_phase_10_ideation.md` | Chat, ideation, proposals, priority |
| 11 | Extensibility | `prd_phase_11_extensibility.md` | Workflows, methodologies, artifacts |

## Instructions for PRD Generation

When creating a phase PRD:

1. **Read the master plan** (`specs/plan.md`) thoroughly for the relevant sections
2. **Extract all implementation details** for that phase
3. **Create atomic tasks** - each task should be completable in one focused session
4. **Include TDD requirements** - tests must be written before implementation
5. **Add acceptance criteria** - clear, verifiable conditions for completion
6. **Preserve dependencies** - note which tasks depend on previous phases
7. **Use the standard PRD format** with JSON task list at the end

### PRD Template

Each phase PRD should follow this structure:

```markdown
# RalphX - Phase N: [Phase Name]

## Overview
[Brief description of what this phase accomplishes]

## Dependencies
- Phase N-1 must be complete
- [Other dependencies]

## Scope
[What's included and explicitly excluded]

## Detailed Requirements
[Extracted from master plan - all relevant sections]

## Implementation Notes
[Key decisions, patterns to follow, gotchas]

## Task List

<!-- JSON task list at end -->
```

---

## Task List

```json
[
  {
    "category": "validation",
    "description": "Validate and finalize phase structure against master plan",
    "steps": [
      "Read the ENTIRE specs/plan.md thoroughly - all 9000+ lines",
      "List every major component/system described in the plan",
      "Cross-reference against the 11 proposed phases - identify gaps or overlaps",
      "Analyze dependencies: which components depend on others being built first?",
      "Consider Ralph loop constraints: serial execution, one task at a time, fresh context each iteration",
      "Identify if any phase is too large (should be split) or too small (should be merged)",
      "Check if phase ordering respects dependencies (can't build UI before data layer, etc.)",
      "Look for components in plan that aren't covered by any phase",
      "Look for phases that don't have corresponding content in plan",
      "If changes needed: update this PRD's phase table and task list",
      "If changes needed: update specs/manifest.json with correct phases",
      "Document your analysis and any changes made in the activity log",
      "Create a dependency graph showing phase relationships"
    ],
    "output": "Updated specs/prd.md and specs/manifest.json (if changes needed), dependency analysis in activity log",
    "passes": true
  },
  {
    "category": "planning",
    "description": "Create Phase 1 PRD: Foundation",
    "steps": [
      "Read specs/plan.md sections: Project Overview, Tech Stack, Architecture, Data Model (core tables only)",
      "Extract all foundation requirements: Tauri setup, React setup, TypeScript config, database schema basics",
      "Create atomic tasks for: project scaffolding, dependency installation, basic types, core entities",
      "Include TDD setup tasks: testing frameworks, test utilities",
      "Write PRD to specs/phases/prd_phase_01_foundation.md",
      "Verify PRD tasks align with master plan - cross-reference each task"
    ],
    "output": "specs/phases/prd_phase_01_foundation.md",
    "passes": false
  },
  {
    "category": "planning",
    "description": "Create Phase 2 PRD: Data Layer",
    "steps": [
      "Read specs/plan.md sections: Repository Pattern Architecture, SQLite Implementation, In-Memory Implementation",
      "Extract repository traits, CRUD operations, state persistence patterns",
      "Create atomic tasks for: repository traits, SQLite repos, memory repos, migrations",
      "Include testing tasks with mock repositories",
      "Write PRD to specs/phases/prd_phase_02_data_layer.md",
      "Verify PRD tasks cover all repository methods from plan"
    ],
    "output": "specs/phases/prd_phase_02_data_layer.md",
    "passes": false
  },
  {
    "category": "planning",
    "description": "Create Phase 3 PRD: State Machine",
    "steps": [
      "Read specs/plan.md sections: Internal Status State Machine, State Machine Definition, statig implementation",
      "Extract all 14 internal statuses, transitions, guards, side effects",
      "Create atomic tasks for: state enum, events, statig macros, transition logic, SQLite integration",
      "Include tasks for state serialization and audit logging",
      "Write PRD to specs/phases/prd_phase_03_state_machine.md",
      "Verify all transitions from the state diagram are covered"
    ],
    "output": "specs/phases/prd_phase_03_state_machine.md",
    "passes": false
  },
  {
    "category": "planning",
    "description": "Create Phase 4 PRD: Agentic Client",
    "steps": [
      "Read specs/plan.md sections: Agentic Client Abstraction Layer, Claude Code Implementation, Mock Client",
      "Extract AgenticClient trait, ClaudeCodeClient, MockAgenticClient implementations",
      "Create atomic tasks for: trait definition, spawn/stop methods, streaming, capabilities",
      "Include cost-optimized testing patterns from plan",
      "Write PRD to specs/phases/prd_phase_04_agentic_client.md",
      "Verify all trait methods and client implementations are covered"
    ],
    "output": "specs/phases/prd_phase_04_agentic_client.md",
    "passes": false
  },
  {
    "category": "planning",
    "description": "Create Phase 5 PRD: Frontend Core",
    "steps": [
      "Read specs/plan.md sections: TypeScript Frontend Best Practices, Real-Time Events, Module Organization",
      "Extract Zod schemas, Tauri invoke wrappers, Zustand stores, event handling",
      "Create atomic tasks for: type definitions, API wrappers, stores, event listeners",
      "Include strict TypeScript configuration from plan",
      "Write PRD to specs/phases/prd_phase_05_frontend_core.md",
      "Verify all type schemas and store patterns are covered"
    ],
    "output": "specs/phases/prd_phase_05_frontend_core.md",
    "passes": false
  },
  {
    "category": "planning",
    "description": "Create Phase 6 PRD: Kanban UI",
    "steps": [
      "Read specs/plan.md sections: UI Components, TaskBoard, Design System",
      "Extract component hierarchy, drag-drop implementation, styling patterns",
      "Create atomic tasks for: TaskBoard, Column, TaskCard, drag-drop, status badges",
      "Include design system tokens (colors, typography, spacing) from plan",
      "Apply anti-AI-slop guardrails in task requirements",
      "Write PRD to specs/phases/prd_phase_06_kanban_ui.md",
      "Verify all UI components and design tokens are covered"
    ],
    "output": "specs/phases/prd_phase_06_kanban_ui.md",
    "passes": false
  },
  {
    "category": "planning",
    "description": "Create Phase 7 PRD: Agent System",
    "steps": [
      "Read specs/plan.md sections: Agent Profiles, Claude Code Agent Definition, RalphX Plugin Structure",
      "Extract agent profile schema, built-in profiles (worker, reviewer, supervisor, orchestrator)",
      "Create atomic tasks for: profile schema, agent definitions, skills, hooks, plugin.json",
      "Include Claude Code integration patterns",
      "Write PRD to specs/phases/prd_phase_07_agent_system.md",
      "Verify all agent profiles and plugin components are covered"
    ],
    "output": "specs/phases/prd_phase_07_agent_system.md",
    "passes": false
  },
  {
    "category": "planning",
    "description": "Create Phase 8 PRD: QA System",
    "steps": [
      "Read specs/plan.md sections: Built-in QA System, Visual Verification Layer, TDD requirements",
      "Extract QA prep agent, QA executor agent, agent-browser integration",
      "Create atomic tasks for: QA schema, QA agents, browser testing, screenshot capture",
      "Include two-phase QA flow (prep parallel with execution, then testing)",
      "Write PRD to specs/phases/prd_phase_08_qa_system.md",
      "Verify complete QA workflow is covered"
    ],
    "output": "specs/phases/prd_phase_08_qa_system.md",
    "passes": false
  },
  {
    "category": "planning",
    "description": "Create Phase 9 PRD: Review & Supervision",
    "steps": [
      "Read specs/plan.md sections: Review System, Supervisor Agent, Human-in-the-Loop Features",
      "Extract reviewer agent, watchdog patterns, loop detection, intervention protocols",
      "Create atomic tasks for: review workflow, supervisor monitoring, stuck detection, human approval UI",
      "Include AskUserQuestion handling from plan",
      "Write PRD to specs/phases/prd_phase_09_review_supervision.md",
      "Verify all review and supervision patterns are covered"
    ],
    "output": "specs/phases/prd_phase_09_review_supervision.md",
    "passes": false
  },
  {
    "category": "planning",
    "description": "Create Phase 10 PRD: Ideation System",
    "steps": [
      "Read specs/plan.md sections: Chat & Ideation System, Ideation View, Priority Assessment, Orchestrator Tools",
      "Extract chat panel, ideation sessions, task proposals, priority algorithm, apply workflow",
      "Create atomic tasks for: chat UI, ideation view, proposal cards, priority scoring, apply modal",
      "Include all 12 orchestrator tools for ideation",
      "Write PRD to specs/phases/prd_phase_10_ideation.md",
      "Verify complete ideation workflow is covered"
    ],
    "output": "specs/phases/prd_phase_10_ideation.md",
    "passes": false
  },
  {
    "category": "planning",
    "description": "Create Phase 11 PRD: Extensibility",
    "steps": [
      "Read specs/plan.md sections: Custom Workflow Schemas, Methodology Support, Artifact System, Deep Research Loops",
      "Extract workflow schema, BMAD/GSD integrations, artifact types/buckets/flows",
      "Create atomic tasks for: workflow CRUD, methodology loading, artifact storage, research processes",
      "Include extensibility database schema from plan",
      "Write PRD to specs/phases/prd_phase_11_extensibility.md",
      "Verify all extension points are covered"
    ],
    "output": "specs/phases/prd_phase_11_extensibility.md",
    "passes": false
  }
]
```
