# RalphX - Phase 31: Ideation Performance Optimization

## Overview

When navigating through many sessions with large conversations, the ideation section becomes laggy. This phase addresses four root causes: no virtualization for message lists, missing memoization causing cascading re-renders, memory leaks from uncleaned Zustand stores, and inefficient selector patterns.

This is a pure frontend optimization phase with no backend changes. The changes primarily affect the Chat components, Ideation components, and Zustand stores.

**Reference Plan:**
- `specs/plans/performance_optimization_ideation.md` - Detailed implementation plan with code examples for virtualization, memoization, memory management, and file extractions

## Goals

1. Eliminate scroll jank in large conversations via virtualization
2. Prevent cascading re-renders with React.memo and useCallback
3. Fix memory leaks with context cleanup and LRU eviction
4. Stabilize selectors to prevent reference inequality re-renders
5. Extract oversized files to meet code quality standards

## Dependencies

### Phase 30 (Ideation Artifacts Fix) - Required

| Dependency | Why Needed |
|------------|------------|
| Ideation UI | Performance optimizations target existing ideation components |
| Chat components | Virtualization and memoization applied to chat message lists |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/performance_optimization_ideation.md`
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
- Tasks 1, 2, 3 can run in parallel (no dependencies)
- Task 4 requires Task 2
- Task 5 requires Task 3
- Task 6 requires Task 2
- Task 7 requires Tasks 1-6

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/performance_optimization_ideation.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Add virtualization to message lists with react-virtuoso",
    "plan_section": "Phase 1: Add Virtualization to Message Lists",
    "blocking": [7],
    "blockedBy": [],
    "atomic_commit": "feat(chat): add virtualization with react-virtuoso",
    "steps": [
      "Read specs/plans/performance_optimization_ideation.md section 'Phase 1'",
      "Install react-virtuoso: npm install react-virtuoso",
      "Update IntegratedChatPanel.tsx to use <Virtuoso> instead of .map()",
      "Update ChatMessages.tsx to use <Virtuoso> for message rendering",
      "Update useIntegratedChatScroll.ts to use Virtuoso's scroll API",
      "Test with a session containing 100+ messages",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat): add virtualization with react-virtuoso"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Memoize chat and ideation components to prevent cascading re-renders",
    "plan_section": "Phase 2: Memoize Components",
    "blocking": [4, 6, 7],
    "blockedBy": [],
    "atomic_commit": "feat(chat): memoize message and proposal components",
    "steps": [
      "Read specs/plans/performance_optimization_ideation.md section 'Phase 2'",
      "Wrap MessageItem with React.memo + custom equality function",
      "Wrap ToolCallIndicator with React.memo",
      "Wrap ProposalCard with React.memo",
      "Add useCallback for handlers in ProposalList",
      "Test that components don't re-render on unrelated state changes",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat): memoize message and proposal components"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Fix memory leaks with context cleanup and LRU eviction",
    "plan_section": "Phase 3: Fix Memory Leaks",
    "blocking": [5, 7],
    "blockedBy": [],
    "atomic_commit": "fix(stores): add cleanup and LRU eviction for memory management",
    "steps": [
      "Read specs/plans/performance_optimization_ideation.md section 'Phase 3'",
      "Add clearMessages action to chatStore.ts",
      "Add context cleanup on switch in IntegratedChatPanel.tsx",
      "Add session cleanup on archive/delete in App.tsx",
      "Add LRU eviction (MAX_CACHED_SESSIONS=20) to ideationStore.ts",
      "Test by switching between 10+ sessions and checking memory in DevTools",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(stores): add cleanup and LRU eviction for memory management"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Move JSON parsing to API layer to prevent redundant parsing",
    "plan_section": "Phase 4: Optimize JSON Parsing",
    "blocking": [7],
    "blockedBy": [2],
    "atomic_commit": "refactor(chat): move JSON parsing to API layer",
    "steps": [
      "Read specs/plans/performance_optimization_ideation.md section 'Phase 4'",
      "Add parseContentBlocks and parseToolCalls functions to src/api/chat.ts",
      "Update getConversation to parse at fetch time",
      "Remove redundant useMemo parsing from MessageItem.tsx",
      "Verify messages display correctly with pre-parsed data",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(chat): move JSON parsing to API layer"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Fix selector stability with memoized empty arrays",
    "plan_section": "Phase 5: Fix Selector Stability",
    "blocking": [7],
    "blockedBy": [3],
    "atomic_commit": "fix(stores): stabilize selectors with memoized empty arrays",
    "steps": [
      "Read specs/plans/performance_optimization_ideation.md section 'Phase 5'",
      "Add EMPTY_ARRAY constant to chatStore.ts",
      "Update selectMessagesForContext to use EMPTY_ARRAY instead of []",
      "Review other selectors for similar reference inequality issues",
      "Test that components don't re-render when selecting empty contexts",
      "Run npm run lint && npm run typecheck",
      "Commit: fix(stores): stabilize selectors with memoized empty arrays"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Extract markdown components to module-level constant",
    "plan_section": "Phase 6: Memoize Markdown Components",
    "blocking": [7],
    "blockedBy": [2],
    "atomic_commit": "refactor(chat): extract markdown components to module constant",
    "steps": [
      "Read specs/plans/performance_optimization_ideation.md section 'Phase 6'",
      "Move MARKDOWN_COMPONENTS config outside MessageItem component",
      "Update ReactMarkdown usage to reference the constant",
      "Verify markdown rendering still works correctly",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(chat): extract markdown components to module constant"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Extract oversized files to meet LOC limits",
    "plan_section": "Phase 7: Refactor Oversized Files",
    "blocking": [],
    "blockedBy": [1, 2, 3, 4, 5, 6],
    "atomic_commit": "refactor(chat): extract MessageItem and IntegratedChatPanel components",
    "steps": [
      "Read specs/plans/performance_optimization_ideation.md section 'Phase 7'",
      "Extract MessageContent.tsx from MessageItem.tsx",
      "Extract useMessageParsing.ts hook to src/components/Chat/hooks/",
      "Extract messageUtils.ts helpers",
      "Extract ChatHeader.tsx from IntegratedChatPanel.tsx",
      "Extract ChatMessageList.tsx (virtualized wrapper)",
      "Extract useChatPanelState.ts hook",
      "Verify all extractions compile: npm run typecheck",
      "Verify MessageItem.tsx is ~200 LOC and IntegratedChatPanel.tsx is ~300 LOC",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(chat): extract MessageItem and IntegratedChatPanel components"
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
| **react-virtuoso over react-window** | Better support for dynamic heights which is essential for chat messages with variable content |
| **LRU eviction with max 20 sessions** | Balances memory usage with UX - users rarely need more than 20 recent sessions in memory |
| **Module-level markdown components** | Prevents object recreation on every render without complex memoization |
| **Parse at API layer** | Single point of parsing eliminates redundant useMemo calls in render path |
| **Memoized empty array constant** | Prevents reference inequality causing unnecessary re-renders |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] Chat message rendering with virtualization
- [ ] Memoized components don't re-render on unrelated changes
- [ ] Memory cleanup on context switch

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Create/select a session with 100+ messages - no scroll jank
- [ ] Rapidly switch between 5+ sessions - memory stabilizes
- [ ] Monitor memory in DevTools - should not grow unbounded
- [ ] Check for scroll jank in message list

### Performance Metrics
```bash
# Open Chrome DevTools > Performance
# Record while switching sessions
# Look for:
# - Long tasks > 50ms
# - Layout thrashing
# - Excessive JS heap growth
```

### Memory Check
```javascript
// In DevTools console after switching 10 sessions:
console.log(Object.keys(useChatStore.getState().messages).length); // Should be ~1-3
console.log(Object.keys(useIdeationStore.getState().sessions).length); // Should be ≤20
```

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Virtuoso component receives data prop and renders items
- [ ] React.memo wrapped components receive correct props
- [ ] LRU eviction triggers when session count exceeds 20
- [ ] Context cleanup triggers on session switch

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
