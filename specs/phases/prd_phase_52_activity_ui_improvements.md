# RalphX - Phase 52: Activity Screen UI Improvements

## Overview

This phase improves the Activity Screen with smarter content rendering, context display, and role filtering. The existing ActivityView.tsx (974 lines) is refactored into focused sub-components to meet the 400-line limit.

Key improvements:
- **Smart content rendering**: Tool results display as formatted JSON, thinking blocks render as markdown
- **Context display**: Each event shows its source (task/session) with clickable navigation links
- **Role filter**: UI control for filtering by agent/system/user roles (backend already supports)
- **Safe JSON parsing**: Prevents crashes from malformed data

**Reference Plan:**
- `specs/plans/activity_screen_ui_improvements.md` - Detailed implementation with component structure and code snippets

## Goals

1. Fix JSON display issues (escaped strings in tool results/call arguments)
2. Add context/source display showing where events originated (task vs session)
3. Add role filter UI (agent/system/user)
4. Refactor ActivityView.tsx from 974 lines to ~300 lines with extracted sub-components

## Dependencies

### Phase 48 (Activity Screen Enhancement) - Required

| Dependency | Why Needed |
|------------|------------|
| Activity events table | Stores persistent events with task_id, session_id, role, metadata |
| Filtering infrastructure | Type filter, status filter, search already implemented |
| Infinite scroll | Cursor-based pagination with TanStack Query |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/activity_screen_ui_improvements.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/activity_screen_ui_improvements.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Extract sub-components from ActivityView.tsx",
    "plan_section": "Task 1: Extract Sub-Components",
    "blocking": [2, 3, 4, 5],
    "blockedBy": [],
    "atomic_commit": "refactor(activity): extract sub-components from ActivityView",
    "steps": [
      "Read specs/plans/activity_screen_ui_improvements.md section 'Task 1: Extract Sub-Components'",
      "Create src/components/activity/ActivityView.types.ts with UnifiedActivityMessage, ViewMode, filter types",
      "Create src/components/activity/ActivityView.utils.ts with getMessageColor, getMessageIcon, highlightJSON, formatTimestamp",
      "Create src/components/activity/ActivityMessage.tsx for message display",
      "Create src/components/activity/ActivityFilters.tsx with ViewModeToggle, FilterTabs, StatusFilter, SearchBar",
      "Create src/components/activity/ActivityContext.tsx as placeholder (will be populated in Task 2)",
      "Create src/components/activity/index.ts for re-exports",
      "Modify src/components/activity/ActivityView.tsx to import from new files and reduce to ~300 lines",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(activity): extract sub-components from ActivityView"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Add safe JSON parsing utility to prevent crashes from malformed data",
    "plan_section": "Task 5: Safe JSON Parsing",
    "blocking": [5],
    "blockedBy": [1],
    "atomic_commit": "feat(activity): add safe JSON parsing utility",
    "steps": [
      "Read specs/plans/activity_screen_ui_improvements.md section 'Task 5: Safe JSON Parsing'",
      "Add safeJsonParse function to src/components/activity/ActivityView.utils.ts",
      "Update toUnifiedMessage() in ActivityView.tsx to use safeJsonParse for metadata",
      "Update ActivityMessage.tsx to use safeJsonParse for content parsing",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): add safe JSON parsing utility"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Add context/source display with role badge showing event origin",
    "plan_section": "Task 2: Add Context/Source Display",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(activity): add context/source display with role badge",
    "steps": [
      "Read specs/plans/activity_screen_ui_improvements.md section 'Task 2: Add Context/Source Display'",
      "Implement ActivityContext component with task/session icon, label, clickable link",
      "Add role badge (Agent/System/User) to ActivityContext",
      "Ensure role field exists in UnifiedActivityMessage type (add if missing)",
      "Integrate ActivityContext into ActivityMessage header below type/status/timestamp row",
      "Test navigation links work correctly",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): add context/source display with role badge"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Add role filter UI control for filtering by agent/system/user",
    "plan_section": "Task 3: Add Role Filter UI",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(activity): add role filter UI",
    "steps": [
      "Read specs/plans/activity_screen_ui_improvements.md section 'Task 3: Add Role Filter UI'",
      "Add RoleFilter component to ActivityFilters.tsx (pill selector: All/Agent/System/User)",
      "Make it multi-select like the existing type filter",
      "Wire RoleFilter to historicalFilter.roles state in ActivityView.tsx",
      "Verify backend already supports role filtering",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): add role filter UI"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Add smart content rendering for tool results, tool calls, and thinking blocks",
    "plan_section": "Task 4: Smart Content Rendering",
    "blocking": [],
    "blockedBy": [1, 2],
    "atomic_commit": "feat(activity): add smart content rendering for tool results and thinking blocks",
    "steps": [
      "Read specs/plans/activity_screen_ui_improvements.md section 'Task 4: Smart Content Rendering'",
      "For tool_result events: Parse content as JSON, display with syntax highlighting in collapsible block",
      "For tool_call events: Show tool name badge, render arguments from metadata as formatted JSON",
      "For thinking events: Render as markdown using react-markdown + remark-gfm, reuse markdownComponents from Chat",
      "For text/error events: Keep as plain text with whitespace preserved",
      "Add fallback to plain text when JSON parse fails (using safeJsonParse from Task 2)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): add smart content rendering for tool results and thinking blocks"
    ],
    "passes": false
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start (inverse of blocking)
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Extract components first** | Creates clean foundation before adding features, prevents making the large file even larger |
| **Reuse existing markdown components** | react-markdown and remark-gfm already installed, markdownComponents exist in Chat |
| **Safe JSON parsing utility** | Prevents UI crashes from malformed backend data, essential for robustness |
| **Role badge in context display** | Combines source and role in one visual element for clarity |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Navigate to Activity screen with existing data
- [ ] Context display: Each event shows source (Task/Session) with clickable link
- [ ] Role display: Agent/System/User badge on each event
- [ ] Role filter: Can filter by role (All/Agent/System/User)
- [ ] Tool results: Show as formatted JSON (not escaped strings with `\n`, `\"`)
- [ ] Tool calls: Show tool name + formatted arguments
- [ ] Thinking blocks: Render as markdown (headers, code blocks, lists)
- [ ] Status filter: Still works correctly
- [ ] Infinite scroll: Still works correctly
- [ ] No crashes: Malformed JSON doesn't break UI

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] ActivityContext component is rendered in ActivityMessage
- [ ] Navigation links in ActivityContext correctly navigate to task/session
- [ ] RoleFilter component is rendered in filter bar
- [ ] Role filter state changes update the query and filter results
- [ ] Smart content rendering activates based on event type

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
