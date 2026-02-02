# Fix: Wire Up Merge Chat Context in UI

## Problem Summary

When a task enters `Merging` state, the chat panel doesn't switch to show the merge conversation. Users can't see what the merger agent is doing.

**Current behavior:** Chat panel stays on task execution conversation
**Expected behavior:** Chat panel switches to merge conversation when task is in `pending_merge` or `merging` status

## Root Cause

`IntegratedChatPanel.tsx` only detects two modes:
- `isExecutionMode` → checks `EXECUTION_STATUSES`
- `isReviewMode` → checks `HUMAN_REVIEW_STATUSES`

Missing: `isMergeMode` → should check `MERGE_STATUSES`

## Files to Modify

1. `src/components/Chat/IntegratedChatPanel.tsx` - Add merge mode detection and query
2. `src/components/Chat/hooks/useChatPanelContext.ts` - Add merge to context type computation
3. `src/components/Chat/ConversationSelector.tsx` - Format "Merge #N" titles

## Implementation Tasks

### Task 1: Add merge mode detection and context hook support (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(chat): add merge mode detection to IntegratedChatPanel and useChatPanelContext`
**Files:** `src/components/Chat/IntegratedChatPanel.tsx`, `src/components/Chat/hooks/useChatPanelContext.ts`

> **Compilation Unit Note:** These two files must be modified together. IntegratedChatPanel passes `isMergeMode` to useChatPanelContext, so the hook must accept it in the same commit.

#### 1a. IntegratedChatPanel.tsx

**Add isMergeMode detection (after line ~90):**
```typescript
const isMergeMode = selectedTask
  ? MERGE_STATUSES.includes(selectedTask.internalStatus)
  : false;
```

**Add merge conversations query (after existing queries ~135):**
```typescript
const { data: mergeConversations } = useQuery({
  queryKey: ["chat-conversations", "merge", selectedTask?.id],
  queryFn: () => chatApi.listConversations("merge", selectedTask!.id),
  enabled: !!selectedTask?.id && isMergeMode,
});
```

**Update conversations selector (line ~138-142):**
```typescript
const conversations = isMergeMode
  ? mergeConversations
  : isReviewMode
  ? reviewConversations
  : executionConversations;
```

**Pass isMergeMode to context hook:**
```typescript
const chatContext = useChatPanelContext({
  // ... existing props
  isMergeMode,
});
```

#### 1b. useChatPanelContext.ts

**Add merge branch to currentContextType computation (~line 174-179):**
```typescript
const currentContextType: ChatContextType = isMergeMode
  ? "merge"
  : isReviewMode
  ? "review"
  : "task_execution";
```

**Update storeContextKey:**
```typescript
const storeContextKey = isMergeMode
  ? `merge:${selectedTaskId}`
  : isReviewMode
  ? `review:${selectedTaskId}`
  : `task:${selectedTaskId}`;
```

### Task 2: Add merge title formatting to ConversationSelector
**Dependencies:** Task 1
**Atomic Commit:** `feat(chat): add merge conversation title formatting`
**Files:** `src/components/Chat/ConversationSelector.tsx`

#### ConversationSelector.tsx

**Add merge title formatting (in getConversationTitle ~line 72-93):**
```typescript
case "merge":
  return `Merge #${index + 1}`;
```

## Verification

1. Create a task with a branch that will have merge conflicts
2. Approve the task → transitions to PendingMerge → Merging
3. Verify: Chat panel switches to show merge conversation
4. Verify: Can see merger agent's activity in real-time
5. Verify: Conversation selector shows "Merge #1" title

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
