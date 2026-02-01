# RalphX - Phase 65: Activity Screen UX Improvement

## Overview

This phase improves the Activity screen user experience by fixing scroll behavior, adding semantic rendering for tool calls and results, and improving visual hierarchy. Currently, tool calls and results display raw JSON which is difficult to read, and history mode auto-scrolls to the bottom instead of staying at the top where the newest events are.

**Reference Plan:**
- `specs/plans/activity_screen_ux_improvement.md` - Detailed implementation plan with code snippets and visual mockups

## Goals

1. Fix scroll behavior so history mode stays at top (newest events), live mode auto-scrolls
2. Add semantic rendering for tool calls with clean names and formatted arguments
3. Add semantic rendering for tool results with human-readable previews
4. Add markdown support for text messages using existing markdown components
5. Improve visual hierarchy and reduce badge noise

## Dependencies

### Phase 64 (Link Conversation IDs to Task State History) - Required

| Dependency | Why Needed |
|------------|------------|
| Activity events infrastructure | Phase 48/52/57 built the activity system this phase enhances |
| ActivityView component | Base component being modified |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/activity_screen_ux_improvement.md`
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

**Parallel Execution:** Tasks 1, 2, and 4 have no dependencies and can run concurrently.

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/activity_screen_ux_improvement.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Fix scroll behavior: disable auto-scroll in history mode, keep it for live mode",
    "plan_section": "Task 1: Fix Scroll Behavior",
    "blocking": [5],
    "blockedBy": [],
    "atomic_commit": "fix(activity): disable auto-scroll in history mode",
    "steps": [
      "Read specs/plans/activity_screen_ux_improvement.md section 'Task 1: Fix Scroll Behavior'",
      "Modify autoScroll state initialization to be false for historical mode",
      "Update useEffect to only auto-scroll in live mode when user wants to follow",
      "Keep 'Scroll to latest' button for manual navigation",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(activity): disable auto-scroll in history mode"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Add semantic tool call rendering with clean names and formatted arguments",
    "plan_section": "Task 2: Semantic Tool Call Rendering",
    "blocking": [3, 5],
    "blockedBy": [],
    "atomic_commit": "feat(activity): add semantic tool call rendering with clean names",
    "steps": [
      "Read specs/plans/activity_screen_ux_improvement.md section 'Task 2: Semantic Tool Call Rendering'",
      "Add cleanToolName() helper to ActivityView.utils.ts to strip mcp__ralphx__ prefix",
      "Update ActivityMessage.tsx tool_call case to show clean name + formatted args",
      "Add collapsible 'Raw JSON' section for debugging",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): add semantic tool call rendering with clean names"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Add semantic tool result rendering with human-readable preview",
    "plan_section": "Task 3: Semantic Tool Result Rendering",
    "blocking": [5],
    "blockedBy": [2],
    "atomic_commit": "feat(activity): add semantic tool result rendering with preview",
    "steps": [
      "Read specs/plans/activity_screen_ux_improvement.md section 'Task 3: Semantic Tool Result Rendering'",
      "Add generateResultPreview() helper to ActivityView.utils.ts",
      "Update ActivityMessage.tsx tool_result case to show preview + success/error indicator",
      "Add expandable section for full syntax-highlighted JSON",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): add semantic tool result rendering with preview"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Add markdown rendering for text messages using existing markdown components",
    "plan_section": "Task 4: Markdown Support for Text Messages",
    "blocking": [5],
    "blockedBy": [],
    "atomic_commit": "feat(activity): add markdown rendering for text messages",
    "steps": [
      "Read specs/plans/activity_screen_ux_improvement.md section 'Task 4: Markdown Support for Text Messages'",
      "Update ActivityMessage.tsx text case to use ReactMarkdown with remarkGfm",
      "Reuse markdownComponents from @/components/Chat/MessageItem.markdown",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): add markdown rendering for text messages"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Visual cleanup: reduce badge noise, improve whitespace, consistent card styling",
    "plan_section": "Task 5: Visual Cleanup",
    "blocking": [],
    "blockedBy": [1, 2, 3, 4],
    "atomic_commit": "refactor(activity): improve visual hierarchy and reduce badge noise",
    "steps": [
      "Read specs/plans/activity_screen_ux_improvement.md section 'Task 5: Visual Cleanup'",
      "Reduce inline badge noise in ActivityMessage.tsx",
      "Improve whitespace and padding",
      "Right-align timestamps with subtle color",
      "Ensure consistent card styling across event types",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(activity): improve visual hierarchy and reduce badge noise"
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
| **Strip `mcp__ralphx__` prefix** | Clean tool names improve readability without losing information |
| **Reuse existing markdownComponents** | Maintains consistency with chat UI, avoids duplication |
| **Collapsible raw JSON** | Preserves debugging capability while improving default experience |
| **Mode-aware scroll** | History shows newest first (no scroll), live follows events (scroll) |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] ActivityView renders without errors
- [ ] Tool call messages show clean names
- [ ] Tool result messages show previews

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] **History scroll**: Load Activity → History mode → Verify stays at top, no auto-scroll
- [ ] **Live scroll**: Generate events → Verify new events appear at top without jarring scroll
- [ ] **Tool call display**: Expand tool call → Verify clean name + formatted arguments
- [ ] **Tool result display**: Expand result → Verify preview + expandable full JSON
- [ ] **Markdown rendering**: Send message with markdown → Verify proper rendering
- [ ] **Visual comparison**: Screenshot before/after for readability improvement

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] cleanToolName() is called for tool_call messages
- [ ] generateResultPreview() is called for tool_result messages
- [ ] ReactMarkdown is used for text messages
- [ ] Collapsible raw JSON works in both tool_call and tool_result

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
