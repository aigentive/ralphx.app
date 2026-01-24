# RalphX - Phase 7: Agent System

## Overview

This phase implements the complete agent system including agent profiles, the RalphX Claude Code plugin structure, supervisor watchdog system, and integration with the agentic client abstraction from Phase 4. The agent system enables autonomous task execution with monitoring, intervention capabilities, and extensible agent behaviors.

## Dependencies

- Phase 1 (Foundation) - TypeScript types, Rust entities
- Phase 3 (State Machine) - State transitions trigger agent spawning
- Phase 4 (Agentic Client) - AgenticClient trait, ClaudeCodeClient, MockAgenticClient
- Phase 5 (Frontend Core) - Event system for supervisor alerts

## Scope

### Included
- Agent profile schema (TypeScript and Rust)
- 5 built-in agent profiles (worker, reviewer, supervisor, orchestrator, deep-researcher)
- RalphX Claude Code plugin structure (agents, skills, hooks)
- Supervisor watchdog system with pattern detection
- Event bus for agent monitoring
- Supervisor actions (log, inject, pause, kill)
- Agent profile storage in SQLite

### Excluded
- QA agents (Phase 8)
- Human review workflow (Phase 9)
- Orchestrator ideation tools (Phase 10)
- Deep researcher processes (Phase 11)

---

## Detailed Requirements

### Agent Profile Schema

Agent profiles are compositions of Claude Code native components that define how an agent behaves:

```typescript
interface AgentProfile {
  id: string;
  name: string;
  description: string;
  role: "worker" | "reviewer" | "supervisor" | "orchestrator" | "researcher";

  // Claude Code component references
  claudeCode: {
    agentDefinition: string;     // Path to .claude/agents/*.md
    skills: string[];            // Skills to inject at startup
    hooks?: HooksConfig;         // Agent-scoped hooks
    mcpServers?: string[];       // MCP servers to enable
  };

  // Execution configuration
  // Model short forms map to Claude 4.5 models:
  //   opus   → claude-opus-4-5-20251101 (Opus 4.5)
  //   sonnet → claude-sonnet-4-5-20250929 (Sonnet 4.5)
  //   haiku  → claude-haiku-4-5-20251001 (Haiku 4.5)
  execution: {
    model: "opus" | "sonnet" | "haiku";
    maxIterations: number;
    timeoutMinutes: number;
    permissionMode: "default" | "acceptEdits" | "bypassPermissions";
  };

  // Artifact I/O (deferred to Phase 11)
  io: {
    inputArtifactTypes: string[];
    outputArtifactTypes: string[];
  };

  // Behavioral flags
  behavior: {
    canSpawnSubAgents: boolean;
    autoCommit: boolean;
    autonomyLevel: "supervised" | "semi_autonomous" | "fully_autonomous";
  };
}
```

### Built-in Agent Profiles

| Profile | Role | Model | Max Iterations | Key Skills |
|---------|------|-------|----------------|------------|
| `worker` | Task execution | Sonnet | 30 | coding-standards, testing-patterns, git-workflow |
| `reviewer` | Code review | Sonnet | 10 | code-review-checklist, security-patterns |
| `supervisor` | Watchdog | Haiku | 100 | anomaly-detection, intervention-patterns |
| `orchestrator` | Planning | Opus | 50 | planning, delegation, synthesis |
| `deep-researcher` | Research | Opus | 200 | research-methodology, source-verification |

### RalphX Plugin Structure

```
ralphx-plugin/
├── .claude-plugin/
│   └── plugin.json
├── agents/
│   ├── worker.md
│   ├── reviewer.md
│   ├── supervisor.md
│   ├── orchestrator.md
│   └── deep-researcher.md
├── skills/
│   ├── coding-standards/SKILL.md
│   ├── testing-patterns/SKILL.md
│   ├── code-review-checklist/SKILL.md
│   ├── research-methodology/SKILL.md
│   └── git-workflow/SKILL.md
├── hooks/
│   └── hooks.json
└── .mcp.json
```

### plugin.json Schema

```json
{
  "name": "ralphx",
  "description": "Autonomous development loop with extensible workflows",
  "version": "1.0.0",
  "author": { "name": "RalphX" },
  "agents": "./agents/",
  "skills": "./skills/",
  "hooks": "./hooks/hooks.json",
  "mcpServers": "./.mcp.json"
}
```

### hooks.json Schema

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/scripts/lint-fix.sh",
            "timeout": 30
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "prompt",
            "prompt": "Verify task completion: check acceptance criteria and update task status"
          }
        ]
      }
    ]
  }
}
```

### Worker Agent Definition Example

`.claude/agents/worker.md`:
```markdown
---
name: ralphx-worker
description: Executes implementation tasks autonomously
tools: Read, Write, Edit, Bash, Grep, Glob, Git
permissionMode: acceptEdits
skills:
  - coding-standards
  - testing-patterns
  - git-workflow
hooks:
  PostToolUse:
    - matcher: "Write|Edit"
      hooks:
        - type: command
          command: "npm run lint:fix"
---

You are a focused developer agent executing a specific task.

## Your Mission
Complete the assigned task by:
1. Understanding requirements fully
2. Writing clean, tested code
3. Committing atomic changes

## Constraints
- Only modify files directly related to the task
- Run tests before marking complete
- Keep changes minimal and focused
```

### Skill Definition Example

`.claude/skills/coding-standards/SKILL.md`:
```markdown
---
name: coding-standards
description: Project coding standards and patterns
disable-model-invocation: true
user-invocable: false
---

## Coding Standards

### TypeScript
- Use strict mode
- Prefer const over let
- Use explicit return types on functions

### React
- Functional components only
- Use hooks for state management
- Props interfaces above component

### Testing
- Test file next to source: `Component.test.tsx`
- Use React Testing Library
- Mock external dependencies
```

### Supervisor Agent (Watchdog System)

The supervisor is an always-on monitoring system that watches task execution and intervenes when problems occur.

#### Trigger Events (Hooks)

| Event | Trigger | What Supervisor Checks |
|-------|---------|------------------------|
| `on_task_start` | Task begins execution | Validate acceptance criteria exists |
| `on_tool_call` | Every tool invocation | Detect repetition patterns (same call 3x = loop) |
| `on_error` | Tool or agent error | Analyze error, suggest fix or pause |
| `on_progress_tick` | Every 30 seconds | Check for forward progress (files changed, commits) |
| `on_token_threshold` | Token usage > 50k | Potential runaway, check if productive |
| `on_time_threshold` | Task running > 10 min | Check if stuck or legitimately complex |

#### Detection Patterns

```
Infinite Loop Detection:
- Same tool called 3+ times with identical/similar args
- Same error occurring repeatedly
- No file changes after N tool calls

Stuck Detection:
- No git diff changes for 5+ minutes
- Agent asking clarifying questions repeatedly
- High token usage with no progress

Poor Task Definition:
- Agent requests clarification multiple times
- Vague acceptance criteria (detected at task start)
```

#### Supervisor Actions

| Severity | Action |
|----------|--------|
| **Low** | Log warning, continue monitoring |
| **Medium** | Inject guidance into agent context ("Try a different approach") |
| **High** | Pause task, mark as `blocked`, notify user |
| **Critical** | Kill task, mark as `failed`, show analysis to user |

#### Supervisor Architecture

```
┌─────────────────────────────────────────────────────┐
│  Execution Loop (per task)                          │
│  ┌───────────────────────────────────────────────┐  │
│  │  Agent SDK Hooks                              │  │
│  │  - PreToolUse  → emit event                   │  │
│  │  - PostToolUse → emit event                   │  │
│  │  - OnError     → emit event                   │  │
│  └───────────────────────────────────────────────┘  │
│                        │                            │
│                        ▼                            │
│  ┌───────────────────────────────────────────────┐  │
│  │  Event Bus (lightweight, in-process)          │  │
│  └───────────────────────────────────────────────┘  │
│                        │                            │
│                        ▼                            │
│  ┌───────────────────────────────────────────────┐  │
│  │  Supervisor (triggered, feels always-on)      │  │
│  │  - Quick checks: pattern matching, timers     │  │
│  │  - Escalation: full agent call if anomaly     │  │
│  │  - Model: haiku for speed (upgrade if needed) │  │
│  └───────────────────────────────────────────────┘  │
│                        │                            │
│                        ▼                            │
│  ┌───────────────────────────────────────────────┐  │
│  │  Actions: log / inject / pause / kill         │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

#### Implementation Notes

- **Lightweight first**: Most checks are pattern matching, no LLM call
- **Escalate to agent**: Only invoke supervisor agent (Haiku) when anomaly detected
- **State tracking**: Keep rolling window of last 10 tool calls per task
- **Configurable thresholds**: User can adjust sensitivity in settings

### Custom Tools for Agent

The Agent SDK will have custom tools to interact with the database:

| Tool | Description |
|------|-------------|
| `get_next_task` | Returns the highest priority task with status `planned` |
| `update_task_status` | Updates a task's status (e.g., `planned` → `in_progress` → `completed`) |
| `log_activity` | Appends an entry to the activity log |
| `create_checkpoint` | Creates a human-in-the-loop checkpoint that pauses execution |
| `get_project_context` | Returns project metadata and recent activity for context |
| `insert_task` | Adds a new task at the correct priority position |

### Agent Profile Database Schema

```sql
CREATE TABLE agent_profiles (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  role TEXT NOT NULL,
  profile_json TEXT NOT NULL,  -- Full AgentProfile as JSON
  is_builtin BOOLEAN DEFAULT FALSE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

---

## Implementation Notes

### Key Design Decisions

1. **Agent profiles are compositions** of Claude Code native components (agents, skills, hooks, MCP servers)
2. **Supervisor is lightweight-first** - pattern matching before LLM escalation
3. **Event bus is in-process** - no external message broker needed
4. **Rolling window state** - track last 10 tool calls for pattern detection
5. **Configurable thresholds** - users can adjust supervisor sensitivity

### File Size Limits

- Agent definitions: 100 lines max
- Skill definitions: 150 lines max
- Hook configurations: 50 lines max

### Testing Strategy

- Unit tests for pattern detection algorithms
- Integration tests with MockAgenticClient
- Supervisor action tests with mock event bus

### Anti-AI-Slop Guardrails

- Agent definitions should be concise and focused
- Skills should contain only relevant standards
- No generic boilerplate in prompts

---

## Task List

```json
[
  {
    "category": "setup",
    "description": "Create RalphX plugin directory structure",
    "steps": [
      "Create ralphx-plugin/ directory in project root",
      "Create subdirectories: .claude-plugin/, agents/, skills/, hooks/",
      "Create empty placeholder files for all 5 agent definitions",
      "Create empty placeholder files for all 5 skill definitions",
      "Verify directory structure matches specification"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create plugin.json manifest",
    "steps": [
      "Write tests for plugin.json schema validation",
      "Create .claude-plugin/plugin.json with name, description, version, author",
      "Add component paths: agents, skills, hooks, mcpServers",
      "Verify JSON is valid and matches schema"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement AgentProfile Rust struct",
    "steps": [
      "Write unit tests for AgentProfile serialization/deserialization",
      "Create src-tauri/src/domain/agents/agent_profile.rs",
      "Define AgentProfile struct with all fields from schema",
      "Define AgentRole enum (worker, reviewer, supervisor, orchestrator, researcher)",
      "Define ExecutionConfig, ClaudeCodeConfig, BehaviorConfig structs",
      "Implement serde Serialize/Deserialize for all structs",
      "Run cargo test to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement AgentProfile TypeScript types",
    "steps": [
      "Write unit tests for Zod schema validation",
      "Create src/types/agent-profile.ts",
      "Define AgentProfile interface matching Rust struct",
      "Create Zod schemas for runtime validation",
      "Export all types and schemas",
      "Run npm run typecheck to verify"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create worker agent definition",
    "steps": [
      "Create ralphx-plugin/agents/worker.md",
      "Add YAML frontmatter: name, description, tools, permissionMode, skills",
      "Add PostToolUse hook for lint-fix after Write|Edit",
      "Write focused system prompt for task execution",
      "Keep file under 100 lines"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create reviewer agent definition",
    "steps": [
      "Create ralphx-plugin/agents/reviewer.md",
      "Add YAML frontmatter with code-review-checklist, security-patterns skills",
      "Set model to sonnet, maxIterations to 10",
      "Write system prompt focused on code review tasks",
      "Keep file under 100 lines"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create supervisor agent definition",
    "steps": [
      "Create ralphx-plugin/agents/supervisor.md",
      "Add YAML frontmatter with anomaly-detection, intervention-patterns skills",
      "Set model to haiku, maxIterations to 100",
      "Write system prompt for watchdog monitoring",
      "Keep file under 100 lines"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create orchestrator agent definition",
    "steps": [
      "Create ralphx-plugin/agents/orchestrator.md",
      "Add YAML frontmatter with planning, delegation, synthesis skills",
      "Set model to opus, maxIterations to 50",
      "Write system prompt for planning and coordination",
      "Keep file under 100 lines"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create deep-researcher agent definition",
    "steps": [
      "Create ralphx-plugin/agents/deep-researcher.md",
      "Add YAML frontmatter with research-methodology, source-verification skills",
      "Set model to opus, maxIterations to 200",
      "Write system prompt for deep research tasks",
      "Keep file under 100 lines"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create coding-standards skill",
    "steps": [
      "Create ralphx-plugin/skills/coding-standards/SKILL.md",
      "Add YAML frontmatter with disable-model-invocation: true",
      "Document TypeScript standards (strict mode, const, explicit types)",
      "Document React standards (functional components, hooks)",
      "Document testing standards (RTL, mocking)",
      "Keep file under 150 lines"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create testing-patterns skill",
    "steps": [
      "Create ralphx-plugin/skills/testing-patterns/SKILL.md",
      "Add YAML frontmatter",
      "Document TDD workflow (tests first)",
      "Document Rust testing patterns (cargo test)",
      "Document TypeScript testing patterns (Vitest)",
      "Keep file under 150 lines"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create code-review-checklist skill",
    "steps": [
      "Create ralphx-plugin/skills/code-review-checklist/SKILL.md",
      "Add YAML frontmatter",
      "Document code quality checks",
      "Document security checks",
      "Document performance considerations",
      "Keep file under 150 lines"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create research-methodology skill",
    "steps": [
      "Create ralphx-plugin/skills/research-methodology/SKILL.md",
      "Add YAML frontmatter",
      "Document research process (sources, verification)",
      "Document citation requirements",
      "Document synthesis patterns",
      "Keep file under 150 lines"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create git-workflow skill",
    "steps": [
      "Create ralphx-plugin/skills/git-workflow/SKILL.md",
      "Add YAML frontmatter",
      "Document commit message conventions",
      "Document branching strategy",
      "Document atomic commit principles",
      "Keep file under 150 lines"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create hooks.json configuration",
    "steps": [
      "Create ralphx-plugin/hooks/hooks.json",
      "Add PostToolUse hook for Write|Edit → lint-fix",
      "Add Stop hook for task completion verification",
      "Create hooks/scripts/lint-fix.sh placeholder",
      "Verify JSON is valid"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Create .mcp.json configuration",
    "steps": [
      "Create ralphx-plugin/.mcp.json",
      "Add empty mcpServers object (placeholder for future MCP integrations)",
      "Verify JSON is valid"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement SupervisorEvent enum in Rust",
    "steps": [
      "Write unit tests for event serialization",
      "Create src-tauri/src/domain/supervisor/mod.rs",
      "Define SupervisorEvent enum (TaskStart, ToolCall, Error, ProgressTick, TokenThreshold, TimeThreshold)",
      "Define event payload structs for each variant",
      "Implement serde traits",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement EventBus for supervisor",
    "steps": [
      "Write unit tests for event publishing and subscribing",
      "Create src-tauri/src/infrastructure/supervisor/event_bus.rs",
      "Implement EventBus struct with tokio::broadcast channel",
      "Add publish() method for emitting events",
      "Add subscribe() method for receiving events",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement pattern detection algorithms",
    "steps": [
      "Write unit tests for loop detection (3+ identical calls)",
      "Write unit tests for stuck detection (no progress for 5+ min)",
      "Create src-tauri/src/domain/supervisor/patterns.rs",
      "Implement ToolCallWindow struct (rolling window of last 10 calls)",
      "Implement detect_loop() function",
      "Implement detect_stuck() function",
      "Implement detect_poor_task_definition() function",
      "Run cargo test to verify all detection patterns"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement SupervisorAction enum",
    "steps": [
      "Write unit tests for action serialization",
      "Create SupervisorAction enum in src-tauri/src/domain/supervisor/actions.rs",
      "Define variants: Log, InjectGuidance, Pause, Kill",
      "Define Severity enum: Low, Medium, High, Critical",
      "Implement action_for_severity() function",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement Supervisor service",
    "steps": [
      "Write integration tests with mock event bus",
      "Create src-tauri/src/application/supervisor_service.rs",
      "Implement SupervisorService struct with EventBus and pattern detector",
      "Add process_event() method that applies pattern detection",
      "Add escalate_to_agent() method for anomaly analysis via Haiku",
      "Add execute_action() method for Log/InjectGuidance/Pause/Kill",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement agent_profiles table migration",
    "steps": [
      "Write integration test for migration",
      "Create migration file for agent_profiles table",
      "Define columns: id, name, role, profile_json, is_builtin, created_at",
      "Run migration",
      "Verify table created with correct schema"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement AgentProfileRepository trait",
    "steps": [
      "Write unit tests for repository methods",
      "Create src-tauri/src/domain/repositories/agent_profile_repo.rs",
      "Define AgentProfileRepository trait with CRUD methods",
      "Add get_by_role() method",
      "Add get_builtin_profiles() method",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement SqliteAgentProfileRepository",
    "steps": [
      "Write integration tests with test database",
      "Create src-tauri/src/infrastructure/sqlite/agent_profile_repo.rs",
      "Implement all repository methods",
      "Handle JSON serialization for profile_json column",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Seed built-in agent profiles",
    "steps": [
      "Write test for seeding built-in profiles",
      "Create seeding function in migration or startup",
      "Insert 5 built-in profiles (worker, reviewer, supervisor, orchestrator, deep-researcher)",
      "Set is_builtin = true for all",
      "Verify profiles are seeded correctly"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement Tauri commands for agent profiles",
    "steps": [
      "Write integration tests for Tauri commands",
      "Create get_agent_profiles command",
      "Create get_agent_profile_by_id command",
      "Create get_agent_profile_by_role command",
      "Run tauri dev to verify commands work"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement supervisor event emission in AgenticClientSpawner",
    "steps": [
      "Write integration tests for event emission",
      "Update AgenticClientSpawner to hold EventBus reference",
      "Emit TaskStart event when spawn_agent called",
      "Emit ToolCall events (hook into agent streaming)",
      "Emit Error events on agent errors",
      "Run cargo test to verify events are emitted"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement supervisor alert TypeScript types",
    "steps": [
      "Write unit tests for Zod schema validation",
      "Create src/types/supervisor.ts",
      "Define SupervisorAlert interface",
      "Define SupervisorAction type",
      "Create Zod schemas for runtime validation",
      "Run npm run typecheck to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Implement useSupervisorAlerts hook (extend from Phase 5)",
    "steps": [
      "Write unit tests for hook behavior",
      "Extend useSupervisorAlerts from Phase 5 to handle new event types",
      "Add filtering by severity level",
      "Add acknowledge functionality",
      "Run npm run test to verify"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integration test: supervisor detects infinite loop",
    "steps": [
      "Create integration test file",
      "Set up MockAgenticClient with repeating responses",
      "Emit 4 identical ToolCall events",
      "Verify supervisor detects loop pattern",
      "Verify appropriate action is taken (InjectGuidance or Pause)",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integration test: supervisor detects stuck agent",
    "steps": [
      "Create integration test file",
      "Set up MockAgenticClient",
      "Emit ProgressTick events with no git diff changes",
      "Fast-forward time by 6 minutes",
      "Verify supervisor detects stuck pattern",
      "Verify appropriate action is taken",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "integration",
    "description": "Integration test: end-to-end agent spawning with supervisor",
    "steps": [
      "Create integration test file",
      "Set up full AppState with supervisor and mock agent client",
      "Spawn a worker agent via AgenticClientSpawner",
      "Verify TaskStart event emitted",
      "Simulate tool calls and verify monitoring",
      "Run cargo test to verify"
    ],
    "passes": false
  },
  {
    "category": "feature",
    "description": "Export agent system modules",
    "steps": [
      "Update src-tauri/src/domain/mod.rs to export supervisor module",
      "Update src-tauri/src/infrastructure/mod.rs to export supervisor implementations",
      "Update src-tauri/src/lib.rs to register agent profile Tauri commands",
      "Verify all public APIs are exported",
      "Run cargo build to verify"
    ],
    "passes": false
  }
]
```
