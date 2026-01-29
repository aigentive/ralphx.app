# RalphX - Phase 27: Chat Architecture Refactor

## Overview

TaskChatPanel has grown to 586 LOC with complex 3-way branching logic to handle task/execution/review modes. The current `useChat` hook can't distinguish between context types, forcing the component to run 3 separate conversation queries, manually override which to use, and duplicate queue logic. This results in review conversations not loading reliably due to timing issues.

This phase introduces a dedicated `useTaskChat` hook that properly handles context types, simplifying TaskChatPanel by ~200 LOC and fixing the review conversation loading issues.

**Reference Plan:**
- `specs/plans/chat-architecture-refactor.md` - Detailed architecture and implementation plan

## Goals

1. Create `useTaskChat` hook that correctly fetches conversations by context type
2. Simplify TaskChatPanel from 586 LOC to ~350 LOC by removing branching logic
3. Unify message queues to use context-aware keys instead of separate execution queue
4. Fix review conversation loading reliability

## Dependencies

### Phase 26 (Auto-Scheduler for Ready Tasks) - Required

| Dependency | Why Needed |
|------------|------------|
| Chat system | Base chat infrastructure with useChat, chatApi, stores |
| Task execution flow | TaskChatPanel, context types, conversation management |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/chat-architecture-refactor.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/chat-architecture-refactor.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "frontend",
    "description": "Create useTaskChat hook with context-aware conversation fetching",
    "plan_section": "Phase 1: Create useTaskChat hook",
    "steps": [
      "Read specs/plans/chat-architecture-refactor.md section 'Phase 1'",
      "Create src/hooks/useTaskChat.ts with the hook implementation",
      "Hook should accept taskId and contextType (task | task_execution | review)",
      "Use correct context type for conversation list query",
      "Handle auto-selection of latest conversation",
      "Sync agent running state with context key",
      "Export from hooks index if one exists",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat): add useTaskChat hook for context-aware conversations"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Migrate TaskChatPanel to use useTaskChat hook",
    "plan_section": "Phase 2: Migrate TaskChatPanel to useTaskChat",
    "steps": [
      "Read specs/plans/chat-architecture-refactor.md section 'Phase 2'",
      "Replace useChat + 3 separate queries with single useTaskChat call",
      "Remove context memo (lines ~241-246)",
      "Remove three separate conversation queries (lines ~266-288)",
      "Remove activeConversation override logic (lines ~290-306)",
      "Remove auto-select effect that moved to hook (lines ~313-320)",
      "Remove complex loading state logic (lines ~440-452)",
      "Update handlers to use contextKey from hook",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(chat): migrate TaskChatPanel to useTaskChat hook"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Unify message queues in chatStore",
    "plan_section": "Phase 3: Unify message queues in store",
    "steps": [
      "Read specs/plans/chat-architecture-refactor.md section 'Phase 3'",
      "Remove executionQueuedMessages from ChatState",
      "Remove queueExecutionMessage and deleteExecutionQueuedMessage actions",
      "Update queueMessage to use context-aware keys (task:id, task_execution:id, review:id)",
      "Update TaskChatPanel queue handlers to use unified system",
      "Update any other components using execution queue",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(chat): unify message queues with context-aware keys"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Clean up useChat and verify all contexts work",
    "plan_section": "Phase 4: Keep useChat for simple contexts",
    "steps": [
      "Read specs/plans/chat-architecture-refactor.md section 'Phase 4'",
      "Review useChat usage - keep for ideation and project chat",
      "Remove any dead code from useChat that was moved to useTaskChat",
      "Ensure chatKeys are exported for external use if needed",
      "Verify TaskChatPanel works in all modes (task, execution, review)",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(chat): clean up useChat, verify all chat contexts"
    ],
    "passes": true
  }
]
```

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Dedicated useTaskChat hook** | Separates task-specific context handling from generic useChat, avoiding 3-way branching in components |
| **Context-aware queue keys** | Uses `${contextType}:${taskId}` format instead of separate queue systems, simplifying state management |
| **Keep useChat for simple contexts** | Ideation and project chat don't need context type switching, so useChat remains appropriate |
| **Hook handles auto-selection** | Moving auto-select logic to hook ensures consistent behavior and removes component complexity |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] useTaskChat hook tests pass
- [ ] TaskChatPanel component tests pass

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Task in "reviewing" status shows active review conversation
- [ ] Task in "review_passed" status shows review conversation with chat enabled
- [ ] Task in "executing" status shows execution conversation
- [ ] Switching between task/execution/review modes loads correct conversations
- [ ] Message queue works correctly in all context types
- [ ] Ideation chat still works (uses useChat)
- [ ] Project/Kanban chat still works (uses useChat)

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] useTaskChat hook is imported and used in TaskChatPanel
- [ ] Conversation queries use correct context type parameter
- [ ] Queue actions use context-aware keys
- [ ] Agent running state syncs with correct context key

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
