# RalphX - Phase 44: Fix Chat Context Bleeding

## Overview

When switching between chat contexts (task → ideation → project → review) while a message is streaming, stale UI elements from the old conversation appear in the new context. This includes the stop button and streaming tool call bubbles bleeding through context switches due to a race condition between context cleanup effects and asynchronous Tauri event listeners.

The fix is minimal: add `activeConversationId` to the event listener effect dependency arrays so that listeners are re-subscribed on context change, ensuring old events are properly ignored.

**Reference Plan:**
- `specs/plans/fix_chat_context_bleeding_on_rapid_context_switches.md` - Detailed root cause analysis and implementation steps

## Goals

1. Prevent streaming tool call bubbles from old conversations appearing in new contexts
2. Prevent stop button from old contexts appearing after switching
3. Ensure messages are received correctly in the new context after a switch

## Dependencies

### Phase 42 (Welcome Screen Redesign) - None Required

This phase operates on independent chat hook infrastructure and has no dependencies on Phase 42.

| Dependency | Why Needed |
|------------|------------|
| None | This is a bug fix to existing hooks |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_chat_context_bleeding_on_rapid_context_switches.md`
2. Understand the race condition and why adding to deps fixes it
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol:**

Reference: `.claude/rules/commit-lock.md`

1. Establish project root: `PROJECT_ROOT="$(git rev-parse --show-toplevel)"`
2. Acquire lock before `git add` (see commit-lock.md § Protocol)
3. Stage and commit using `git -C "$PROJECT_ROOT"`
4. Release lock after commit: `rm -f "$PROJECT_ROOT/.commit-lock"`

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Both tasks in this phase have no dependencies and can run in parallel
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_chat_context_bleeding_on_rapid_context_switches.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Fix useIntegratedChatEvents event listener dependency array",
    "plan_section": "Step 1: Fix useIntegratedChatEvents.ts",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(hooks): add activeConversationId to event listener deps in useIntegratedChatEvents",
    "steps": [
      "Read specs/plans/fix_chat_context_bleeding_on_rapid_context_switches.md section 'Step 1'",
      "Open src/hooks/useIntegratedChatEvents.ts",
      "Add `activeConversationId` to the event listener useEffect dependency array (line ~140)",
      "Add `setStreamingToolCalls([])` to the cleanup function (before the unlisteners cleanup)",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(hooks): add activeConversationId to event listener deps in useIntegratedChatEvents"
    ],
    "passes": false
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Fix useChatPanelHandlers event listener dependency array",
    "plan_section": "Step 2: Fix useChatPanelHandlers.ts",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "fix(hooks): add activeConversationId to event listener deps in useChatPanelHandlers",
    "steps": [
      "Read specs/plans/fix_chat_context_bleeding_on_rapid_context_switches.md section 'Step 2'",
      "Open src/hooks/useChatPanelHandlers.ts",
      "Add `activeConversationId` to the event listener useEffect dependency array (line ~366)",
      "Add `setStreamingToolCalls([])` to the cleanup function (before the unlisteners cleanup)",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(hooks): add activeConversationId to event listener deps in useChatPanelHandlers"
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
| **Add conversationId to effect deps** | Forces re-subscription on context switch, ensuring old listeners are cleaned up before new ones are created. This is the minimal fix that addresses the root cause. |
| **Clear streaming state in cleanup** | Ensures any accumulated tool calls from the old context are cleared when switching, preventing stale data from persisting across the re-subscription cycle. |
| **No additional state management** | The existing ref pattern is correct; the bug was only in the missing dependency. Adding complexity would be unnecessary. |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] No regressions in existing chat tests

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Open task A with an active agent streaming
- [ ] Quickly switch to ideation session
- [ ] Verify: No stop button appears in ideation
- [ ] Verify: No tool call bubbles appear in ideation
- [ ] Verify: New context loads correctly

### Test Scenarios
- [ ] Task → Ideation (while agent streaming) - no bleeding
- [ ] Ideation → Task (while agent streaming) - no bleeding
- [ ] Task (execution mode) → Task (regular chat) (same task) - no bleeding
- [ ] Task → Project chat (while agent streaming) - no bleeding

### Regression Checks
- [ ] Tool calls still appear correctly in the active conversation
- [ ] Stop button still works in the correct context
- [ ] Messages are still received properly
- [ ] Run completion still clears streaming state

### Wiring Verification

**For each modified hook, verify the effect lifecycle:**

- [ ] `activeConversationId` is in the dependency array
- [ ] Cleanup function calls `setStreamingToolCalls([])`
- [ ] Cleanup function calls all unlisteners
- [ ] Effect re-runs when conversation changes

**Common failure modes to check:**
- [ ] No missing dependencies in useEffect (ESLint exhaustive-deps should pass)
- [ ] No accidental early returns before cleanup registration

See `.claude/rules/gap-verification.md` for full verification workflow.
