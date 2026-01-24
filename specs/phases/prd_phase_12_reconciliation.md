# RalphX - Phase 12: Reconciliation

## Overview

This phase addresses architectural inconsistencies discovered during implementation. It consolidates scattered components, aligns with best practices, and ensures the codebase follows a coherent design.

## Dependencies

- All previous phases (1-11) should be complete before reconciliation
- This phase may touch code from any previous phase

## Scope

**Included:**
- Consolidate all agents/skills into the plugin architecture
- Update all references to use plugin paths
- Verify Claude Code CLI integration patterns
- Fix any other architectural inconsistencies discovered

**Excluded:**
- New features
- Performance optimizations (unless related to architecture)

---

## Issue 1: Mixed Agent/Skill Locations

### Problem

Agents and skills are scattered across two locations:
- `.claude/agents/` and `.claude/skills/` (project-level)
- `ralphx-plugin/agents/` and `ralphx-plugin/skills/` (plugin)

This creates confusion about where components belong and prevents proper control via `--plugin-dir`.

### Current State

**In `.claude/` (project-level):**
- `.claude/agents/qa-prep.md`
- `.claude/agents/qa-executor.md`
- `.claude/skills/agent-browser/`
- `.claude/skills/acceptance-criteria-writing/`
- `.claude/skills/qa-step-generation/`
- `.claude/skills/qa-evaluation/`
- (Phase 10 will add: `orchestrator-ideation.md`, `task-decomposition/`, `priority-assessment/`, `dependency-analysis/`)

**In `ralphx-plugin/`:**
- `agents/worker.md`
- `agents/reviewer.md`
- `agents/supervisor.md`
- `agents/orchestrator.md`
- `agents/deep-researcher.md`
- `skills/coding-standards/`
- `skills/testing-patterns/`
- `skills/code-review-checklist/`
- `skills/research-methodology/`
- `skills/git-workflow/`

### Solution

Consolidate everything into `ralphx-plugin/`. RalphX controls loading via `--plugin-dir ./ralphx-plugin`.

**Benefits:**
1. **Control**: `--plugin-dir` gives explicit control over what Claude sees
2. **Isolation**: User's `.claude/` stays clean
3. **Atomic loading**: All components load together
4. **Versioning**: Plugin versioned with the app
5. **`${CLAUDE_PLUGIN_ROOT}`**: Relative paths work correctly

**Target structure:**
```
ralphx-plugin/
├── .claude-plugin/
│   └── plugin.json
├── agents/
│   ├── worker.md
│   ├── reviewer.md
│   ├── supervisor.md
│   ├── orchestrator.md
│   ├── deep-researcher.md
│   ├── qa-prep.md                    # consolidated
│   ├── qa-executor.md                # consolidated
│   └── orchestrator-ideation.md      # consolidated (from Phase 10)
├── skills/
│   ├── coding-standards/
│   ├── testing-patterns/
│   ├── code-review-checklist/
│   ├── research-methodology/
│   ├── git-workflow/
│   ├── agent-browser/                # consolidated
│   ├── acceptance-criteria-writing/  # consolidated
│   ├── qa-step-generation/           # consolidated
│   ├── qa-evaluation/                # consolidated
│   ├── task-decomposition/           # consolidated (from Phase 10)
│   ├── priority-assessment/          # consolidated (from Phase 10)
│   └── dependency-analysis/          # consolidated (from Phase 10)
├── hooks/
│   └── hooks.json
└── .mcp.json
```

**Keep in `.claude/`:**
- `.claude/settings.json` - Project-level permissions (not part of plugin)

---

## Issue 2: Missing Visual Verification

### Problem

Visual verification was largely skipped during UI implementation phases. The `screenshots/` folder is empty (only `.gitkeep`), and the activity log shows only ONE visual verification attempt which partially failed.

### What Went Wrong

1. **Wrong dev command**: Used `npm run dev` (Vite-only) instead of `npm run tauri dev` (full app with backend)
2. **No screenshots captured**: Zero screenshots despite PROMPT.md having clear instructions
3. **Rationalized as "covered by unit tests"**: Agent skipped visual verification claiming unit tests were sufficient
4. **Tauri backend required**: Without Tauri, `invoke` commands fail and app shows errors

### Evidence from Activity Log

```
### 2026-01-24 15:25:00 - Visual verification of QA UI components
- Started dev server on http://localhost:1420
- Verified page renders using agent-browser (shows error without Tauri backend)
- Note: Full visual screenshots require Tauri backend running
```

### Solution

Retroactively verify all UI components from Phases 5, 6, 8, 9, 10:
1. Run `npm run tauri dev` (not `npm run dev`)
2. Wait for Tauri to compile and serve
3. Use `agent-browser` to capture screenshots
4. Fix any visual issues discovered
5. Verify anti-AI-slop compliance (no purple gradients, no Inter font, no generic icons)

### Components Requiring Visual Verification

**Phase 5 (Frontend Core):**
- Basic app shell and layout

**Phase 6 (Kanban UI):**
- TaskBoard with columns
- TaskCard component
- Drag-drop interactions
- Status badges

**Phase 8 (QA System):**
- TaskQABadge
- TaskDetailQAPanel
- QASettingsPanel

**Phase 9 (Review & Supervision):**
- Review components
- Supervisor dashboard
- Human-in-loop approval UI

**Phase 10 (Ideation):**
- ChatPanel and ChatMessage
- ProposalCard and ProposalList
- IdeationView
- PriorityBadge

---

## Issue 3: Missing Automatic Reconciliation (Auditor System)

### Problem

RalphX has no automatic way to detect and correct systemic issues in completed work. Currently:
- **Supervisor** monitors in real-time during task execution
- **Review System** checks individual tasks post-completion
- **Neither** looks backwards at patterns across multiple completed tasks

We discovered Issue 1 and Issue 2 manually by reading the activity log. This should be automatic.

### Important: RalphX is Task-Based, Not Phase-Based

RalphX core model:
```
Project → Tasks (with statuses, steps, dependencies)
```

- `wave` field exists but is nullable (for grouping parallel tasks)
- `phase_id` and `plan_id` are **methodology extensions** (BMAD, GSD), not core
- The auditor must work at the **task level**, not phase level

### Solution: Auditor Agent

A new agent that analyzes completed work and creates reconciliation tasks.

**Triggers (task-based, not phase-based):**

| Trigger | When | Use Case |
|---------|------|----------|
| `queue_empty` | All tasks approved/completed | End of work session |
| `task_count` | Every N tasks approved | Periodic checkup (e.g., every 10 tasks) |
| `terminal_state` | Task enters `failed` or `cancelled` | Post-mortem analysis |
| `manual` | User runs `/audit` | On-demand |
| `time_based` | Every X hours of execution | Long-running sessions |

**For methodologies (extensibility only):**

| Trigger | When | Use Case |
|---------|------|----------|
| `phase_complete` | All tasks in phase approved | BMAD phase transitions |
| `wave_complete` | All tasks in wave approved | GSD wave checkpoints |

### Auditor Behavior

```
1. Read activity logs for recently completed tasks
2. Read task descriptions/steps from database
3. Compare: what was required vs what was logged
4. Identify patterns:
   - Steps marked complete but not logged (e.g., "visual verification")
   - Required artifacts missing (screenshots, test files)
   - Repeated workarounds ("skipped due to...", "marked as done because...")
   - Same errors occurring across multiple tasks
5. Generate reconciliation tasks
6. Either:
   - Auto-add to queue (if autoReconcile enabled)
   - Present to user for approval
```

### Configuration

```typescript
interface AuditTrigger {
  type: 'task_count' | 'queue_empty' | 'terminal_state' | 'manual' | 'time_based';
  config: {
    taskThreshold?: number;      // For task_count (e.g., 10)
    intervalMinutes?: number;    // For time_based (e.g., 60)
    terminalStates?: string[];   // For terminal_state (e.g., ['failed', 'cancelled'])
  };
}

interface AuditConfig {
  enabled: boolean;
  triggers: AuditTrigger[];
  autoReconcile: boolean;  // Auto-create tasks or require approval
  rules: AuditRule[];
}

interface AuditRule {
  name: string;
  description: string;
  check: 'activity_pattern' | 'artifact_exists' | 'step_logged' | 'custom';
  pattern?: string;       // Regex for activity_pattern
  path?: string;          // Glob for artifact_exists
  stepKeyword?: string;   // For step_logged
  severity: 'low' | 'medium' | 'high' | 'critical';
}
```

### Default Audit Rules

```typescript
const defaultAuditRules: AuditRule[] = [
  {
    name: "visual_verification_logged",
    description: "UI tasks should have visual verification in activity log",
    check: "activity_pattern",
    pattern: "screenshot|visual verification|agent-browser",
    severity: "medium"
  },
  {
    name: "screenshots_captured",
    description: "UI tasks should produce screenshots",
    check: "artifact_exists",
    path: "screenshots/*.png",
    severity: "medium"
  },
  {
    name: "tests_created",
    description: "Tasks with TDD requirement should create test files",
    check: "artifact_exists",
    path: "**/*.test.{ts,tsx}",
    severity: "high"
  },
  {
    name: "tauri_dev_used",
    description: "UI verification should use 'tauri dev' not just 'npm run dev'",
    check: "activity_pattern",
    pattern: "tauri dev",
    severity: "low"
  }
];
```

### Integration Points

| System | How Auditor Integrates |
|--------|------------------------|
| **Supervisor** | Auditor is "retrospective supervisor" - same severity model |
| **Review System** | Auditor reviews batches of tasks, not individual tasks |
| **Activity Log** | Primary data source for pattern detection |
| **Ideation** | Audit findings can feed into ideation as proposals |
| **Methodologies** | Each methodology can define additional audit rules |

---

## Issue 4: (Placeholder for future issues)

_Add additional reconciliation issues here as they are discovered._

---

## Implementation Notes

### Claude CLI Integration Pattern

When RalphX spawns agents, use `--plugin-dir`:

```rust
// Spawn any agent with plugin loaded
fn spawn_agent(agent_name: &str, prompt: &str) -> Result<Output> {
    Command::new("claude")
        .args([
            "--plugin-dir", "./ralphx-plugin",
            "--agent", agent_name,
            "-p", prompt,
            "--output-format", "stream-json",
        ])
        .output()
}
```

### Agent Profile References

Update `AgentProfile` to not include paths (plugin handles discovery):

```typescript
// Before (wrong - hardcoded paths)
const qaPrepProfile = {
  claudeCode: {
    agentDefinition: ".claude/agents/qa-prep.md",  // hardcoded
    skills: ["acceptance-criteria-writing"],
  }
};

// After (correct - just names, plugin resolves)
const qaPrepProfile = {
  claudeCode: {
    agent: "qa-prep",  // plugin resolves via --plugin-dir
    skills: ["acceptance-criteria-writing"],  // plugin provides
  }
};
```

### Migration Steps

1. Move files from `.claude/` to `ralphx-plugin/`
2. Update `plugin.json` to include all agents/skills
3. Update Rust code to use `--plugin-dir` flag
4. Update TypeScript types to reflect new structure
5. Remove empty `.claude/agents/` and `.claude/skills/` directories
6. Keep `.claude/settings.json` for permissions

---

## Task List

```json
[
  {
    "category": "refactoring",
    "description": "Move QA agents from .claude/ to ralphx-plugin/",
    "steps": [
      "Move .claude/agents/qa-prep.md to ralphx-plugin/agents/qa-prep.md",
      "Move .claude/agents/qa-executor.md to ralphx-plugin/agents/qa-executor.md",
      "Update ralphx-plugin/.claude-plugin/plugin.json to include qa agents",
      "Verify agents are discoverable with: claude --plugin-dir ./ralphx-plugin --help",
      "Remove .claude/agents/ directory if empty"
    ],
    "passes": false
  },
  {
    "category": "refactoring",
    "description": "Move QA skills from .claude/ to ralphx-plugin/",
    "steps": [
      "Move .claude/skills/acceptance-criteria-writing/ to ralphx-plugin/skills/",
      "Move .claude/skills/qa-step-generation/ to ralphx-plugin/skills/",
      "Move .claude/skills/qa-evaluation/ to ralphx-plugin/skills/",
      "Update plugin.json skills path if needed",
      "Verify skills are discoverable"
    ],
    "passes": false
  },
  {
    "category": "refactoring",
    "description": "Move agent-browser skill to ralphx-plugin/",
    "steps": [
      "Move .claude/skills/agent-browser/ to ralphx-plugin/skills/",
      "Update any references in hooks or agents that use agent-browser",
      "Verify agent-browser commands work via plugin"
    ],
    "passes": false
  },
  {
    "category": "refactoring",
    "description": "Update Rust AgentProfile to use plugin pattern",
    "steps": [
      "Read current AgentProfile struct in src-tauri/",
      "Remove agentDefinition path field (plugin handles discovery)",
      "Add agent name field that maps to plugin agent",
      "Update all agent profile instantiations",
      "Run cargo test to verify compilation"
    ],
    "passes": false
  },
  {
    "category": "refactoring",
    "description": "Update Claude spawning to use --plugin-dir",
    "steps": [
      "Find all Command::new(\"claude\") calls in Rust code",
      "Add --plugin-dir ./ralphx-plugin to all spawn calls",
      "Update --agent flag to use simple agent names (not paths)",
      "Test spawning qa-prep agent with new flags",
      "Test spawning worker agent with new flags"
    ],
    "passes": false
  },
  {
    "category": "refactoring",
    "description": "Update TypeScript types for plugin-based agents",
    "steps": [
      "Update AgentProfile TypeScript interface",
      "Remove agentDefinition path references",
      "Update any frontend code that references agent paths",
      "Run npm run typecheck to verify"
    ],
    "passes": false
  },
  {
    "category": "refactoring",
    "description": "Consolidate Phase 10 ideation components (if created)",
    "steps": [
      "Check if .claude/agents/orchestrator-ideation.md exists",
      "If exists, move to ralphx-plugin/agents/",
      "Check if .claude/skills/task-decomposition/ exists",
      "If exists, move ideation skills to ralphx-plugin/skills/",
      "Update any references"
    ],
    "passes": false
  },
  {
    "category": "cleanup",
    "description": "Clean up .claude/ directory",
    "steps": [
      "Verify .claude/settings.json still exists (keep this)",
      "Remove .claude/agents/ directory",
      "Remove .claude/skills/ directory",
      "Verify .claude/commands/ still exists if used",
      "Run git status to confirm cleanup"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "Verify plugin integration end-to-end",
    "steps": [
      "Start the app with npm run tauri dev",
      "Create a test task",
      "Trigger QA prep flow - verify qa-prep agent spawns correctly",
      "Trigger worker execution - verify worker agent spawns correctly",
      "Check logs for --plugin-dir in Claude commands",
      "Verify no errors related to missing agents/skills"
    ],
    "passes": false
  },
  {
    "category": "documentation",
    "description": "Update documentation for plugin architecture",
    "steps": [
      "Update CLAUDE.md to reflect plugin-only architecture",
      "Update specs/plan.md agent sections to use plugin pattern",
      "Document --plugin-dir usage in README if applicable",
      "Update any PRD references to .claude/ paths"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of Kanban UI (Phase 6)",
    "steps": [
      "Run npm run tauri dev and wait for compilation",
      "agent-browser open http://localhost:1420",
      "agent-browser snapshot -i -c to analyze page structure",
      "Navigate to Kanban board view",
      "agent-browser screenshot screenshots/kanban-board-overview.png",
      "Verify TaskCard rendering - agent-browser screenshot screenshots/task-card.png",
      "Test drag-drop: agent-browser click on a task card, drag to another column",
      "agent-browser screenshot screenshots/drag-drop-interaction.png",
      "Verify status badges render correctly",
      "Check anti-AI-slop: no purple gradients, no Inter font, no generic icons",
      "agent-browser close",
      "Document findings in activity.md"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of QA UI components (Phase 8)",
    "steps": [
      "Run npm run tauri dev and wait for compilation",
      "agent-browser open http://localhost:1420",
      "Navigate to a task with QA data",
      "agent-browser screenshot screenshots/qa-badge.png",
      "Open task detail panel",
      "agent-browser screenshot screenshots/qa-detail-panel.png",
      "Verify Acceptance Criteria tab renders",
      "agent-browser screenshot screenshots/qa-acceptance-criteria-tab.png",
      "Verify Test Results tab renders",
      "agent-browser screenshot screenshots/qa-test-results-tab.png",
      "Open QA Settings panel",
      "agent-browser screenshot screenshots/qa-settings-panel.png",
      "Check anti-AI-slop compliance",
      "agent-browser close",
      "Document findings in activity.md"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of Review & Supervision UI (Phase 9)",
    "steps": [
      "Run npm run tauri dev and wait for compilation",
      "agent-browser open http://localhost:1420",
      "Navigate to review components (if accessible)",
      "agent-browser screenshot screenshots/review-panel.png",
      "Verify supervisor dashboard renders",
      "agent-browser screenshot screenshots/supervisor-dashboard.png",
      "Test human-in-loop approval UI if available",
      "agent-browser screenshot screenshots/human-approval-ui.png",
      "Check anti-AI-slop compliance",
      "agent-browser close",
      "Document findings in activity.md"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of Ideation UI (Phase 10)",
    "steps": [
      "Run npm run tauri dev and wait for compilation",
      "agent-browser open http://localhost:1420",
      "Navigate to Ideation view",
      "agent-browser screenshot screenshots/ideation-view.png",
      "Verify ChatPanel renders",
      "agent-browser screenshot screenshots/chat-panel.png",
      "Test ChatInput interaction",
      "agent-browser screenshot screenshots/chat-input.png",
      "Verify ProposalCard and ProposalList render",
      "agent-browser screenshot screenshots/proposal-list.png",
      "Verify PriorityBadge renders correctly",
      "agent-browser screenshot screenshots/priority-badge.png",
      "Check anti-AI-slop compliance",
      "agent-browser close",
      "Document findings in activity.md"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Fix visual issues discovered during verification",
    "steps": [
      "Review all screenshots captured in previous tasks",
      "Identify any visual issues (layout, styling, responsiveness)",
      "Identify any anti-AI-slop violations",
      "Fix each issue found",
      "Re-capture screenshots to verify fixes",
      "Run npm run lint and npm run typecheck",
      "Document all fixes in activity.md"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Define AuditConfig and AuditRule types",
    "steps": [
      "Create src/types/audit.ts with AuditTrigger, AuditConfig, AuditRule interfaces",
      "Define trigger types: task_count, queue_empty, terminal_state, manual, time_based",
      "Define check types: activity_pattern, artifact_exists, step_logged, custom",
      "Define severity levels: low, medium, high, critical",
      "Add Zod schemas for validation",
      "Run npm run typecheck"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Implement default audit rules",
    "steps": [
      "Create src/lib/audit/defaultRules.ts",
      "Implement visual_verification_logged rule (check activity for screenshot/agent-browser)",
      "Implement screenshots_captured rule (check screenshots/*.png exists)",
      "Implement tests_created rule (check **/*.test.{ts,tsx} exists)",
      "Implement tauri_dev_used rule (check activity for 'tauri dev')",
      "Export defaultAuditRules array",
      "Write unit tests for rule matching logic"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Implement activity log parser",
    "steps": [
      "Create src/lib/audit/activityParser.ts",
      "Parse activity.md into structured entries (timestamp, title, content)",
      "Extract task references from entries",
      "Implement pattern matching against entries",
      "Handle both header section and log entries",
      "Write unit tests for parser"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Implement artifact checker",
    "steps": [
      "Create src/lib/audit/artifactChecker.ts",
      "Implement glob-based file existence checks",
      "Check screenshots directory for PNG files",
      "Check for test files matching patterns",
      "Return structured results (found/missing, paths)",
      "Write unit tests"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Create auditor agent definition",
    "steps": [
      "Create ralphx-plugin/agents/auditor.md",
      "Define role: retrospective analysis of completed work",
      "Define inputs: activity log, task list, audit rules",
      "Define outputs: reconciliation task proposals",
      "Define tools: Read, Glob, Grep (no Write/Edit - read-only)",
      "Add to plugin.json agents list"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Implement audit trigger system",
    "steps": [
      "Create src/lib/audit/triggers.ts",
      "Implement task_count trigger (after N tasks approved)",
      "Implement queue_empty trigger (all tasks in terminal state)",
      "Implement manual trigger (user command)",
      "Implement time_based trigger (interval timer)",
      "Integrate with task state machine events",
      "Write unit tests for each trigger"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Implement reconciliation task generator",
    "steps": [
      "Create src/lib/audit/reconciliation.ts",
      "Generate task proposals from audit findings",
      "Map severity to task priority",
      "Group related findings into single tasks",
      "Format task descriptions with evidence from audit",
      "Support autoReconcile (auto-add) vs approval mode",
      "Write unit tests"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Add audit settings to project configuration",
    "steps": [
      "Add audit section to project settings schema",
      "Default: enabled=true, triggers=[queue_empty, manual], autoReconcile=false",
      "Add UI for audit settings in Settings panel",
      "Persist settings to database",
      "Write tests for settings CRUD"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Implement /audit command",
    "steps": [
      "Create ralphx-plugin/skills/audit/SKILL.md",
      "Define /audit slash command for manual trigger",
      "Show audit progress in UI",
      "Display findings summary when complete",
      "Allow user to approve/reject reconciliation tasks",
      "Document command usage"
    ],
    "passes": false
  },
  {
    "category": "auditor",
    "description": "Add methodology-aware audit rules (extensibility)",
    "steps": [
      "Extend MethodologyConfig with audit rules",
      "Add phase_complete trigger for BMAD",
      "Add wave_complete trigger for GSD",
      "Allow methodologies to define custom rules",
      "Merge methodology rules with default rules",
      "Write tests for methodology integration"
    ],
    "passes": false
  },
  {
    "category": "testing",
    "description": "End-to-end test of auditor system",
    "steps": [
      "Create test project with intentionally skipped steps",
      "Trigger audit manually",
      "Verify correct findings are detected",
      "Verify reconciliation tasks are generated",
      "Test autoReconcile mode",
      "Test approval mode",
      "Document test results in activity.md"
    ],
    "passes": false
  }
]
```
