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
    "passes": false
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
    "passes": false
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
    "passes": false
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
    "passes": false
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
    "passes": false
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
    "passes": false
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
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of Project Sidebar and Navigation",
    "steps": [
      "Use /agent-browser skill to visually verify Project Sidebar and navigation",
      "Capture screenshots of: sidebar with project list, project selector dropdown, worktree status indicator",
      "Test navigation between views (Kanban, Ideation, Activity, Settings, Extensibility)",
      "Test keyboard shortcuts Cmd+1 through Cmd+5",
      "Use /frontend-design skill to fix any visual issues identified during verification",
      "Run npm run lint and npm run typecheck after fixes",
      "Document findings and fixes in activity.md"
    ],
    "acceptance_criteria": [
      "Project Sidebar is positioned on the left with appropriate width (200-280px)",
      "Project list shows project names with git mode indicators clearly",
      "WorktreeStatus shows 'Local: branch' or 'Worktree: branch from base' format",
      "New Project button is prominent and accessible",
      "Active project is visually distinguished from others",
      "Navigation items (Ideation, Kanban, Activity, Settings) are clearly labeled",
      "Active navigation item has clear visual indicator",
      "Keyboard shortcuts work and views switch correctly",
      "Sidebar doesn't overflow or cause horizontal scroll",
      "No purple gradients, no Inter font, no generic icon grids (anti-AI-slop)"
    ],
    "design_quality": [
      "Typography: Project names are readable, git status is secondary",
      "Typography: Navigation labels use appropriate weight (medium/semibold)",
      "Spacing: Projects have adequate vertical spacing in list",
      "Spacing: Navigation items have consistent padding",
      "Colors: Active states use warm orange accent (#ff6b35)",
      "Colors: Git mode indicators are subtle but informative",
      "Depth: Sidebar has subtle separation from main content",
      "Borders: Clean dividers between sections",
      "Interactions: Hover states on projects and nav items are smooth",
      "Polish: Icons (if any) are consistent style",
      "Overall: Feels like a professional sidebar navigation"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of Activity Stream View",
    "steps": [
      "Use /agent-browser skill to visually verify Activity Stream View",
      "Capture screenshots of: activity view overview, expanded tool call, search/filter UI",
      "Test scrolling behavior with many activity entries",
      "Test expand/collapse of tool call details",
      "Use /frontend-design skill to fix any visual issues identified during verification",
      "Run npm run lint and npm run typecheck after fixes",
      "Document findings and fixes in activity.md"
    ],
    "acceptance_criteria": [
      "Activity Stream fills available viewport height",
      "Agent thinking and actions are clearly displayed",
      "Tool calls are distinguishable from regular messages",
      "Expand/collapse for tool call details works smoothly",
      "Input/output of tool calls are formatted readably",
      "Search/filter functionality is accessible and visible",
      "Auto-scroll to new messages works correctly",
      "Timestamps are visible and formatted appropriately",
      "Empty state shows helpful message when no activity",
      "No purple gradients, no Inter font, no generic icon grids (anti-AI-slop)"
    ],
    "design_quality": [
      "Typography: Activity text is readable size (14-16px)",
      "Typography: Tool names are distinguishable (monospace or bold)",
      "Typography: Timestamps use smaller, muted text",
      "Spacing: Activity entries have consistent vertical rhythm",
      "Spacing: Expanded details have adequate padding",
      "Colors: Different message types have subtle color coding",
      "Colors: Tool calls have distinct background or border",
      "Depth: Expanded sections feel nested appropriately",
      "Borders: Entry separators are subtle",
      "Interactions: Scroll is smooth, expand/collapse animates",
      "Polish: Long content doesn't break layout",
      "Overall: Feels like Claude Desktop's execution panel"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of Settings View",
    "steps": [
      "Use /agent-browser skill to visually verify Settings View",
      "Capture screenshots of: settings overview, each section (Execution, Model, Review, Supervisor)",
      "Test form interactions (toggles, dropdowns, number inputs)",
      "Verify profile management UI",
      "Use /frontend-design skill to fix any visual issues identified during verification",
      "Run npm run lint and npm run typecheck after fixes",
      "Document findings and fixes in activity.md"
    ],
    "acceptance_criteria": [
      "Settings View has clear section organization",
      "Each setting has descriptive label and helper text where needed",
      "Toggle switches clearly show on/off state",
      "Number inputs have appropriate min/max constraints",
      "Dropdown selectors show current value and expand properly",
      "Model selection dropdown shows available options",
      "Profile management allows create/edit/delete",
      "Save/cancel actions are clear (if applicable)",
      "Form validation errors display inline",
      "No purple gradients, no Inter font, no generic icon grids (anti-AI-slop)"
    ],
    "design_quality": [
      "Typography: Section headers establish clear hierarchy",
      "Typography: Setting labels are readable, descriptions are secondary",
      "Spacing: Sections have adequate separation",
      "Spacing: Form elements have consistent spacing",
      "Spacing: Labels and inputs have clear visual association",
      "Colors: Toggle states use appropriate success/neutral colors",
      "Colors: Section backgrounds provide subtle grouping",
      "Depth: Cards/sections have subtle elevation",
      "Borders: Form inputs have consistent border treatment",
      "Interactions: Toggles animate smoothly",
      "Interactions: Dropdowns open/close without jank",
      "Polish: All inputs have proper focus states",
      "Overall: Feels like a well-organized settings panel"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of Project Creation Wizard and Merge Dialog",
    "steps": [
      "Use /agent-browser skill to visually verify Project dialogs",
      "Capture screenshots of: Project Creation Wizard, Git Mode selection, Merge Workflow Dialog",
      "Test folder picker interaction (Tauri dialog)",
      "Test radio button selection for Git Mode and Merge options",
      "Use /frontend-design skill to fix any visual issues identified during verification",
      "Run npm run lint and npm run typecheck after fixes",
      "Document findings and fixes in activity.md"
    ],
    "acceptance_criteria": [
      "Project Creation Wizard opens as modal with proper backdrop",
      "Project name input is clearly labeled and focused on open",
      "Folder picker button triggers Tauri file dialog",
      "Git Mode radio buttons clearly show Local vs Worktree options",
      "Worktree-specific fields appear when Worktree is selected",
      "Branch name auto-generates with ralphx/ prefix",
      "Base branch dropdown shows available branches",
      "Merge Workflow Dialog shows completion summary clearly",
      "View Diff and View Commits buttons are accessible",
      "Merge options are clearly explained with radio buttons",
      "Cancel and primary action buttons are properly positioned",
      "No purple gradients, no Inter font, no generic icon grids (anti-AI-slop)"
    ],
    "design_quality": [
      "Typography: Modal titles are prominent",
      "Typography: Option descriptions are readable",
      "Typography: Warning text (⚠️) stands out appropriately",
      "Spacing: Form fields have adequate vertical spacing",
      "Spacing: Modal has generous padding",
      "Colors: Primary action button uses warm orange accent",
      "Colors: Warning indicators use amber/yellow",
      "Depth: Modal has proper elevation over backdrop",
      "Borders: Radio buttons and inputs have consistent styling",
      "Interactions: Radio selection is responsive",
      "Interactions: Modal open/close animates smoothly",
      "Polish: Tab order is logical for keyboard navigation",
      "Overall: Wizard feels guided and professional"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of Task Re-run Dialog",
    "steps": [
      "Use /agent-browser skill to visually verify Task Re-run Dialog",
      "Capture screenshots of: dialog with all three options, warning state for revert",
      "Test radio button selection between options",
      "Verify warning displays when dependent commits exist",
      "Use /frontend-design skill to fix any visual issues identified during verification",
      "Run npm run lint and npm run typecheck after fixes",
      "Document findings and fixes in activity.md"
    ],
    "acceptance_criteria": [
      "Dialog opens when dragging Done task back to Planned",
      "Task info and commit SHA are clearly displayed",
      "Three options are clearly presented as radio buttons",
      "Recommended option is visually indicated",
      "Revert option shows warning about dependent commits",
      "Warning text is clearly visible and alarming",
      "Cancel and Confirm buttons are properly positioned",
      "Dialog closes on cancel without side effects",
      "No purple gradients, no Inter font, no generic icon grids (anti-AI-slop)"
    ],
    "design_quality": [
      "Typography: Task title is prominent in header",
      "Typography: Commit SHA uses monospace font",
      "Typography: Option descriptions are scannable",
      "Spacing: Options have adequate separation",
      "Spacing: Warning has breathing room",
      "Colors: Recommended option has subtle highlight",
      "Colors: Warning uses amber/yellow appropriately",
      "Depth: Dialog has proper modal elevation",
      "Borders: Radio buttons are clearly styled",
      "Interactions: Selection is immediate and clear",
      "Polish: Focus management is correct",
      "Overall: Dialog communicates consequences clearly"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of Diff Viewer component",
    "steps": [
      "Use /agent-browser skill to visually verify Diff Viewer",
      "Capture screenshots of: Changes tab, History tab, file tree, diff view",
      "Test tab switching between Changes and History",
      "Test file selection in tree view",
      "Test collapse/expand of diff hunks",
      "Use /frontend-design skill to fix any visual issues identified during verification",
      "Run npm run lint and npm run typecheck after fixes",
      "Document findings and fixes in activity.md"
    ],
    "acceptance_criteria": [
      "Diff Viewer has two tabs: Changes and History",
      "File tree shows changed files with status indicators",
      "Selecting a file shows its diff on the right",
      "Unified diff displays with proper +/- indicators",
      "Added lines are visually distinct (green-ish)",
      "Removed lines are visually distinct (red-ish)",
      "Context lines are neutral colored",
      "Diff hunks can collapse/expand",
      "Syntax highlighting works for common languages",
      "Open in IDE button is accessible",
      "History tab shows commit list on left",
      "Selecting commit shows its diff",
      "No purple gradients, no Inter font, no generic icon grids (anti-AI-slop)"
    ],
    "design_quality": [
      "Typography: File names are readable in tree",
      "Typography: Code uses monospace font (JetBrains Mono)",
      "Typography: Line numbers are subtle",
      "Spacing: File tree items have adequate spacing",
      "Spacing: Diff has comfortable line height",
      "Colors: Add/remove colors are intuitive but not harsh",
      "Colors: Syntax highlighting is readable on dark background",
      "Depth: Selected file has clear indicator",
      "Borders: Panels have subtle separators",
      "Interactions: Tab switching is instant",
      "Interactions: Collapse/expand animates smoothly",
      "Polish: Large diffs don't cause lag (virtual scrolling)",
      "Overall: Feels like a professional diff tool"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of Screenshot Gallery in QA Panel",
    "steps": [
      "Use /agent-browser skill to visually verify Screenshot Gallery",
      "Capture screenshots of: thumbnail grid, lightbox view, navigation, expected vs actual comparison",
      "Test clicking thumbnail to open lightbox",
      "Test navigation between screenshots in lightbox",
      "Test expected vs actual comparison view",
      "Use /frontend-design skill to fix any visual issues identified during verification",
      "Run npm run lint and npm run typecheck after fixes",
      "Document findings and fixes in activity.md"
    ],
    "acceptance_criteria": [
      "Screenshot Gallery shows thumbnails in a grid layout",
      "Thumbnails are appropriately sized and don't overflow",
      "Clicking thumbnail opens lightbox with full-size image",
      "Lightbox has prev/next navigation",
      "Close button is clearly visible in lightbox",
      "Expected vs Actual comparison shows both images",
      "Differences are highlighted or clearly labeled",
      "Empty state when no screenshots exist",
      "Gallery integrates properly in TaskDetailQAPanel",
      "No purple gradients, no Inter font, no generic icon grids (anti-AI-slop)"
    ],
    "design_quality": [
      "Typography: Screenshot names/labels are readable",
      "Typography: Comparison labels (Expected/Actual) are clear",
      "Spacing: Thumbnails have consistent gaps",
      "Spacing: Lightbox has adequate padding",
      "Colors: Thumbnail borders are subtle",
      "Colors: Lightbox backdrop is appropriately dark",
      "Depth: Lightbox has proper elevation",
      "Borders: Thumbnails have consistent corner radius",
      "Interactions: Thumbnail hover shows intent",
      "Interactions: Lightbox open/close is smooth",
      "Interactions: Navigation is intuitive",
      "Polish: Images load without jarring layout shifts",
      "Overall: Feels like a professional image gallery"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of Kanban UI (Phase 6)",
    "steps": [
      "Use /agent-browser skill to visually verify the Kanban board",
      "Capture screenshots of: board overview, individual task cards, drag-drop interaction",
      "Use /frontend-design skill to fix any visual issues identified during verification",
      "Run npm run lint and npm run typecheck after fixes",
      "Document findings and fixes in activity.md"
    ],
    "acceptance_criteria": [
      "TaskBoard fills available viewport height (no unnecessary scrolling at page level)",
      "Columns have consistent widths and proper spacing between them",
      "TaskCard has clear visual hierarchy: title prominent, description secondary, metadata subtle",
      "TaskCard hover state is visible but not distracting",
      "Status badges are legible with sufficient contrast against background",
      "Column headers are clearly visible and aligned with column content",
      "Drag handle or drag affordance is visually clear on TaskCard",
      "During drag: dragged card has visual feedback (shadow, opacity change, or elevation)",
      "Drop targets highlight when dragging over them",
      "Empty columns show appropriate empty state (not just blank)",
      "No purple gradients, no Inter font, no generic icon grids (anti-AI-slop)"
    ],
    "design_quality": [
      "Typography: Font weights create clear hierarchy (bold headers, regular body, light metadata)",
      "Typography: Line heights are comfortable for reading (1.4-1.6 for body text)",
      "Typography: Text sizes follow a consistent scale (not arbitrary pixel values)",
      "Spacing: Consistent spacing rhythm throughout (8px base or similar system)",
      "Spacing: Cards have adequate padding - content doesn't feel cramped",
      "Spacing: Negative space is used intentionally to group related elements",
      "Colors: Cohesive, limited palette (not random colors for each element)",
      "Colors: Sufficient contrast ratios (WCAG AA minimum: 4.5:1 for text)",
      "Colors: Status colors are intuitive (success=green-ish, error=red-ish, warning=amber-ish)",
      "Depth: Shadows are subtle and consistent (same light source direction)",
      "Depth: Elevation changes are meaningful (higher = more important/interactive)",
      "Borders: Consistent border radius throughout (not mixing sharp and rounded)",
      "Borders: Border colors are subtle, not harsh black lines",
      "Interactions: Hover/focus states have smooth transitions (150-200ms)",
      "Interactions: No jarring color jumps on state changes",
      "Polish: No orphaned pixels, misaligned elements, or visual glitches",
      "Polish: Icons are consistent style (all outline OR all filled, not mixed)",
      "Overall: Looks like a professional tool, not a prototype or homework project"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of QA UI components (Phase 8)",
    "steps": [
      "Use /agent-browser skill to visually verify QA components",
      "Capture screenshots of: QA badge, detail panel, acceptance criteria tab, test results tab, settings panel",
      "Use /frontend-design skill to fix any visual issues identified during verification",
      "Run npm run lint and npm run typecheck after fixes",
      "Document findings and fixes in activity.md"
    ],
    "acceptance_criteria": [
      "TaskQABadge is compact and doesn't dominate the TaskCard layout",
      "QA badge color/icon clearly indicates pass/fail/pending state",
      "TaskDetailQAPanel opens as a slide-over or modal without layout shift",
      "Detail panel has proper header with close button in expected position (top-right)",
      "Acceptance Criteria tab shows criteria as a scannable list, not a wall of text",
      "Each criterion has clear pass/fail/untested indicator",
      "Test Results tab shows results with timestamps and clear status indicators",
      "Test result details are expandable/collapsible if verbose",
      "QASettingsPanel form inputs are properly labeled and aligned",
      "Settings panel has clear save/cancel actions",
      "Error states (if any) are visually distinct and helpful",
      "No purple gradients, no Inter font, no generic icon grids (anti-AI-slop)"
    ],
    "design_quality": [
      "Typography: Tab labels are appropriately weighted (medium/semibold, not bold or light)",
      "Typography: Result text is scannable - key info (pass/fail) jumps out immediately",
      "Spacing: Panel has generous padding - doesn't feel like a cramped dialog",
      "Spacing: List items have consistent vertical rhythm",
      "Spacing: Form labels and inputs have clear visual association (proximity)",
      "Colors: Pass/fail/pending use distinct but harmonious colors from same palette",
      "Colors: Background colors for states are subtle tints, not harsh saturated colors",
      "Depth: Panel has appropriate elevation over underlying content (shadow or backdrop)",
      "Depth: Active tab is clearly distinguished from inactive tabs",
      "Borders: Tabs have subtle indicator for selected state (underline or background)",
      "Borders: Form inputs have clear but not heavy borders",
      "Interactions: Tab switches are instant or have quick fade transition",
      "Interactions: Expandable sections animate smoothly (not instant show/hide)",
      "Polish: Close button has adequate hit target (min 44x44px)",
      "Polish: Form validation errors appear inline, not as jarring alerts",
      "Overall: Feels like a well-crafted settings/detail panel from a premium app"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of Review & Supervision UI (Phase 9)",
    "steps": [
      "Use /agent-browser skill to visually verify Review and Supervisor components",
      "Capture screenshots of: review panel, supervisor dashboard, human approval UI",
      "Use /frontend-design skill to fix any visual issues identified during verification",
      "Run npm run lint and npm run typecheck after fixes",
      "Document findings and fixes in activity.md"
    ],
    "acceptance_criteria": [
      "Review panel clearly shows what is being reviewed (task context visible)",
      "Review comments/feedback are displayed in a readable format",
      "Approve/Reject/Request Changes buttons are clearly distinguishable",
      "Button states (enabled/disabled/loading) are visually distinct",
      "Supervisor dashboard shows agent status at a glance",
      "Active tasks are visually distinguished from queued/completed",
      "Alerts or issues are highlighted with appropriate urgency colors",
      "Human-in-loop approval UI clearly explains what action is needed",
      "Approval dialog has clear confirm/cancel actions",
      "Loading states don't cause layout shifts",
      "No purple gradients, no Inter font, no generic icon grids (anti-AI-slop)"
    ],
    "design_quality": [
      "Typography: Dashboard headers establish clear information hierarchy",
      "Typography: Status text is concise and scannable (not verbose sentences)",
      "Typography: Button labels are action-oriented and clear (not vague)",
      "Spacing: Dashboard cards/sections have consistent gaps",
      "Spacing: Button groups have appropriate spacing (not cramped, not too spread)",
      "Spacing: Alert messages have adequate breathing room",
      "Colors: Primary action (Approve) is visually prominent",
      "Colors: Destructive action (Reject) uses appropriate warning color",
      "Colors: Dashboard uses color sparingly - not a rainbow of status colors",
      "Colors: Urgency levels are clearly differentiated (critical vs warning vs info)",
      "Depth: Cards have subtle shadows establishing visual grouping",
      "Depth: Modal dialogs have proper backdrop and elevation",
      "Borders: Cards have consistent corner radius",
      "Borders: Section dividers are subtle (hairline or spacing, not heavy rules)",
      "Interactions: Buttons have clear hover and active states",
      "Interactions: Loading spinners are appropriately sized (not too large or small)",
      "Polish: Empty dashboard state is helpful (not just 'No data')",
      "Polish: Agent status indicators use consistent visual language (dots, badges, etc.)",
      "Overall: Feels like a command center - professional, focused, actionable"
    ],
    "passes": false
  },
  {
    "category": "visual-verification",
    "description": "Visual verification of Ideation UI (Phase 10)",
    "steps": [
      "Use /agent-browser skill to visually verify Ideation components",
      "Capture screenshots of: ideation view, chat panel, chat input, proposal list, priority badges",
      "Use /frontend-design skill to fix any visual issues identified during verification",
      "Run npm run lint and npm run typecheck after fixes",
      "Document findings and fixes in activity.md"
    ],
    "acceptance_criteria": [
      "ChatPanel fills the available height of its container (chat should not be tiny)",
      "Messages are clearly attributed (user vs assistant visually distinct)",
      "Message timestamps are subtle but readable",
      "Chat history scrolls properly with newest messages visible",
      "ChatInput is positioned at bottom and stays fixed during scroll",
      "ChatInput has clear send button with appropriate disabled state when empty",
      "ChatInput expands gracefully for multi-line input (or has clear affordance)",
      "ProposalCard shows proposal title, description preview, and status",
      "ProposalList is scrollable if many proposals exist",
      "PriorityBadge colors are meaningful and consistent (high=urgent, low=subtle)",
      "Priority badges don't overwhelm the card content",
      "IdeationView layout balances chat and proposals appropriately",
      "Empty states (no messages, no proposals) are helpful, not blank",
      "No purple gradients, no Inter font, no generic icon grids (anti-AI-slop)"
    ],
    "design_quality": [
      "Typography: Message text is comfortable reading size (14-16px)",
      "Typography: User vs assistant messages have subtle typographic distinction",
      "Typography: Proposal titles are scannable (semibold, not all caps)",
      "Typography: Timestamps and metadata use smaller, lighter text",
      "Spacing: Messages have adequate vertical spacing (not cramped chat bubbles)",
      "Spacing: Chat input has comfortable padding for typing",
      "Spacing: Proposal cards have balanced internal spacing",
      "Spacing: Two-column layout (if used) has appropriate gutter",
      "Colors: User messages and assistant messages have distinct but harmonious backgrounds",
      "Colors: Chat background provides subtle contrast from surrounding UI",
      "Colors: Priority colors follow intuitive urgency scale",
      "Colors: Selected/active proposal has clear visual indication",
      "Depth: Chat bubbles have subtle depth (soft shadow or border)",
      "Depth: Proposal cards have hover elevation change",
      "Borders: Message bubbles have appropriate rounding (not too sharp, not too pill-shaped)",
      "Borders: Cards have consistent corner radius matching rest of app",
      "Interactions: Send button has satisfying pressed state",
      "Interactions: Scroll behavior is smooth (no janky jumps)",
      "Interactions: New message appears with subtle animation",
      "Polish: Typing indicator (if present) is subtle and non-distracting",
      "Polish: Long messages don't break layout (proper text wrapping)",
      "Polish: Code blocks in messages (if supported) are properly styled",
      "Overall: Chat feels modern and responsive like Slack/Discord, not like 2010 IRC"
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
