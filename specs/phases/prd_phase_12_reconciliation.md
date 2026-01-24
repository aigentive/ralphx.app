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

## Issue 2: Missing UI Components and Views

### Problem

Gap analysis between the master plan (`specs/plan.md`) and current implementation reveals significant UI elements that were specified but not implemented. These gaps affect core functionality and user experience.

### Layout Gaps

| Element | Plan Spec | Current State |
|---------|-----------|---------------|
| **Project Sidebar** | Left sidebar with project list, status indicators, New Project button | Only header with hardcoded "Demo Project" |
| **Activity Navigation** | Activity view showing agent execution | View type exists but no UI |
| **Settings Navigation** | Settings view with configuration | View type exists but no UI |

### Missing Screens/Views

1. **Activity Stream View** - Real-time agent execution monitoring with expandable tool calls, search, filters
2. **Settings View** - Configuration for execution, model, review, supervisor settings, and profile management
3. **Project Creation Wizard** - Git mode selection (Local vs Worktree), folder picker, branch config
4. **Merge Workflow Dialog** - Post-completion options (merge, rebase, PR, keep, discard)
5. **Task Re-run Dialog** - Options when moving completed task back to Planned

### Missing Components

1. **Diff Viewer** - Split-view with Changes/History tabs, file tree, syntax highlighting, Web Worker support
2. **Project Sidebar** - Project list with git mode indicators, navigation items
3. **Worktree Status Indicator** - Shows "Local: main" or "Worktree: branch from base"
4. **Screenshot Gallery/Lightbox** - For QA visual verification with Expected vs Actual comparison
5. **Project Selector** - Dropdown in header replacing hardcoded project name

### Design Requirements

All components must follow the established design system:
- Warm orange accent (#ff6b35), no purple gradients
- SF Pro fonts (not Inter)
- 8pt grid spacing system
- Dark theme with 4.5:1 minimum contrast
- 150-200ms transitions

### Reference

See `specs/plan.md` for exact specifications:
- "Project Creation Wizard" section
- "Merge Workflow Dialog" section
- "Task Re-run Dialog" section
- "Diff Viewer" section
- "Minimal Essential Settings" section
- "Activity Stream" section

---

## Issue 3: Missing Visual Verification

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

## Issue 4: Missing Automatic Reconciliation (Auditor System)

> **Note:** The auditor system implementation has been moved to **Phase 15** (`specs/phases/prd_phase_15_auditor.md`). This section documents the problem and design; Phase 15 contains the implementation tasks.

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

## Issue 5: (Placeholder for future issues)

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

**IMPORTANT: Work on ONE task per iteration.** Find the first task with `"passes": false`, complete it, update `"passes": true`, commit, and stop. The Issues above are documentation - the tasks below are the actual work items.

### How to Read Tasks

**CRITICAL: Read ALL fields of a task before starting work.** Each task may contain:

| Field | Purpose |
|-------|---------|
| `description` | What the task is about (summary only) |
| `steps` | **Required actions** - follow these step by step |
| `acceptance_criteria` | **What to verify** - the task is NOT complete until all criteria pass |
| `design_quality` | **Visual standards** - for UI tasks, verify these design requirements |
| `passes` | Mark `true` only when ALL steps completed AND all criteria verified |

**For visual-verification tasks specifically:**
1. Read the `steps` to know what to capture and test
2. Read `acceptance_criteria` to know what functional requirements to check
3. Read `design_quality` to know what design standards to verify
4. Fix ANY issue found in steps 2 or 3 using `/frontend-design` skill
5. Only mark `passes: true` when everything in all three sections is satisfied

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
    "passes": true
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
    "passes": true
  },
  {
    "category": "refactoring",
    "description": "Move agent-browser skill to ralphx-plugin/",
    "steps": [
      "Move .claude/skills/agent-browser/ to ralphx-plugin/skills/",
      "Update any references in hooks or agents that use agent-browser",
      "Verify agent-browser commands work via plugin"
    ],
    "passes": true
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
    "passes": true
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
    "passes": true
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
    "passes": true
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
    "passes": true
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
    "passes": true
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
    "passes": true
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
    "passes": true
  },
  {
    "category": "ui-gaps",
    "description": "Implement Project Sidebar with project list and navigation",
    "steps": [
      "Create src/components/projects/ProjectSidebar.tsx component",
      "Add project list with status indicators (git mode, branch, dirty state)",
      "Add 'New Project' button that triggers creation wizard",
      "Add project switching functionality",
      "Implement WorktreeStatus indicator component",
      "Integrate with projectStore for state management",
      "Add navigation items: Ideation, Kanban, Activity, Settings",
      "Write unit tests for ProjectSidebar",
      "Reference specs/plan.md 'Project Sidebar' section for exact layout"
    ],
    "passes": true
  },
  {
    "category": "ui-gaps",
    "description": "Implement Activity Stream View",
    "steps": [
      "Create src/components/activity/ActivityView.tsx component",
      "Display agent thinking and actions in real-time",
      "Add expandable tool call details (inputs/outputs)",
      "Implement scrollable history with auto-scroll to new messages",
      "Add search/filter functionality by tool name or action type",
      "Connect to activityStore for message streaming",
      "Style similar to Claude Desktop execution panel",
      "Write unit tests for ActivityView",
      "Reference specs/plan.md 'Activity Stream' section"
    ],
    "passes": true
  },
  {
    "category": "ui-gaps",
    "description": "Implement Settings View with all configuration sections",
    "steps": [
      "Create src/components/settings/SettingsView.tsx component",
      "Add Execution section: max_concurrent_tasks, auto_commit, pause_on_failure",
      "Add Model section: model selection dropdown, Opus upgrade option",
      "Add Review section: ai_review_enabled, ai_review_auto_fix, require_human_review, max_fix_attempts",
      "Add Supervisor section: supervisor_enabled, loop_threshold, stuck_timeout",
      "Add Profile Management: create/edit/delete custom profiles",
      "Connect to settings store and Tauri backend for persistence",
      "Write unit tests for SettingsView",
      "Reference specs/plan.md 'Minimal Essential Settings' section for defaults"
    ],
    "passes": true
  },
  {
    "category": "ui-gaps",
    "description": "Implement Project Creation Wizard with Git Mode selection",
    "steps": [
      "Use /frontend-design skill to create a polished, professional wizard modal",
      "Create src/components/projects/ProjectCreationWizard.tsx modal",
      "Add project name input field",
      "Add folder picker with Browse button (Tauri dialog)",
      "Add Git Mode radio selector: Local vs Isolated Worktree",
      "For Worktree mode: branch name input, base branch dropdown, worktree path display",
      "Add validation and error states",
      "Connect to Tauri backend for git operations",
      "Write unit tests for ProjectCreationWizard",
      "Reference specs/plan.md 'Project Creation Wizard' section for exact ASCII layout"
    ],
    "passes": true
  },
  {
    "category": "ui-gaps",
    "description": "Implement Merge Workflow Dialog for post-completion",
    "steps": [
      "Use /frontend-design skill to create a polished, professional dialog",
      "Create src/components/projects/MergeWorkflowDialog.tsx modal",
      "Show project completion summary (commit count, branch name)",
      "Add View Diff and View Commits buttons",
      "Add radio options: Merge to main, Rebase onto main, Create PR, Keep worktree, Discard changes",
      "Connect to Tauri backend for git merge/rebase operations",
      "Handle confirmation for destructive actions (Discard)",
      "Write unit tests for MergeWorkflowDialog",
      "Reference specs/plan.md 'Merge Workflow Dialog' section for exact layout"
    ],
    "passes": true
  },
  {
    "category": "ui-gaps",
    "description": "Implement Task Re-run Dialog (Done to Planned)",
    "steps": [
      "Use /frontend-design skill to create a polished, professional dialog",
      "Create src/components/tasks/TaskRerunDialog.tsx modal",
      "Show task info and associated commit SHA",
      "Add radio options: Keep changes (recommended), Revert commit, Create new task",
      "Add warning for revert option about dependent commits",
      "Implement revert logic with conflict detection",
      "Update task run_number and previous_commit_sha in database",
      "Write unit tests for TaskRerunDialog",
      "Reference specs/plan.md 'Task Re-run Dialog' section for exact layout"
    ],
    "passes": true
  },
  {
    "category": "ui-gaps",
    "description": "Implement Diff Viewer component with Changes and History tabs",
    "steps": [
      "Use /frontend-design skill to create a polished, professional diff viewer",
      "Install @git-diff-view/react library",
      "Create src/components/diff/DiffViewer.tsx component",
      "Add Tab 1: Changes - real-time uncommitted modifications view",
      "Add Tab 2: History - commit list with diff view",
      "Add file tree on left side showing changed files",
      "Add unified diff view on right with syntax highlighting",
      "Implement collapse/expand for diff hunks",
      "Add 'Open in IDE' button using Tauri shell commands",
      "Use Web Worker for off-main-thread diff computation",
      "Write unit tests for DiffViewer",
      "Reference specs/plan.md 'Diff Viewer' section"
    ],
    "passes": true
  },
  {
    "category": "ui-gaps",
    "description": "Implement Screenshot Gallery/Lightbox for QA panel",
    "steps": [
      "Use /frontend-design skill to create a polished, professional gallery",
      "Create src/components/qa/ScreenshotGallery.tsx component",
      "Display thumbnail grid of captured screenshots",
      "Implement lightbox modal for full-size view",
      "Add navigation between screenshots (prev/next)",
      "Add Expected vs Actual comparison view for failures",
      "Integrate with TaskDetailQAPanel",
      "Write unit tests for ScreenshotGallery"
    ],
    "passes": true
  },
  {
    "category": "ui-gaps",
    "description": "Integrate Diff Viewer into Reviews Panel",
    "steps": [
      "Use /frontend-design skill for polished integration styling",
      "Update ReviewsPanel.tsx to include DiffViewer",
      "Add Changes/History tabs to review detail view",
      "Connect to git backend for real-time diff data",
      "Ensure proper loading states during diff computation",
      "Write integration tests for Reviews with DiffViewer"
    ],
    "passes": true
  },
  {
    "category": "ui-gaps",
    "description": "Replace hardcoded Project Selector with functional component",
    "steps": [
      "Use /frontend-design skill to create a polished dropdown selector",
      "Update App.tsx header to use ProjectSelector component",
      "Create src/components/projects/ProjectSelector.tsx dropdown",
      "Show current project with git mode indicator",
      "Add project list in dropdown with status badges",
      "Add 'New Project' option that opens wizard",
      "Connect to projectStore for state management",
      "Write unit tests for ProjectSelector"
    ],
    "passes": true
  },
  {
    "category": "ui-gaps",
    "description": "Add Activity and Settings navigation to app layout",
    "steps": [
      "Use /frontend-design skill for polished navigation styling",
      "Update App.tsx to include Activity view in navigation",
      "Update App.tsx to include Settings view in navigation",
      "Add keyboard shortcuts: Cmd+4 for Activity, Cmd+5 for Settings",
      "Update uiStore currentView type to include 'activity' and 'settings'",
      "Ensure proper view switching and state preservation",
      "Write integration tests for navigation"
    ],
    "passes": true
  }
]
```

---

## Note: Auditor Tasks Moved

The auditor system tasks (9 tasks + 1 e2e test) have been moved to **Phase 15** to run after the design phases.

See `specs/phases/prd_phase_15_auditor.md` for the auditor implementation.
