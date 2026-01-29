# Chat Architecture Refactor Plan

## Problem Statement

TaskChatPanel has become a mess of 3-way branching logic to handle task/execution/review modes. The `useChat` hook can't distinguish between context types, forcing the component to:
- Run 3 separate conversation queries
- Manually override which query to use
- Handle loading states incorrectly
- Duplicate queue logic

**Result:** Review conversations don't load reliably due to timing issues between queries and effects.

## Current Architecture (586 LOC in TaskChatPanel)

```
TaskChatPanel
  ├── useChat(context)                    # Always returns contextType="task"
  ├── useQuery("task_execution", taskId)  # Manual override for execution
  ├── useQuery("review", taskId)          # Manual override for review
  ├── useConversation(activeConversationId) # Direct fetch bypass
  │
  ├── 3-way branching: conversations = execution ? ... : review ? ... : regular
  ├── 3-way branching: activeConversation = (review || execution) ? direct : regular
  ├── 3-way branching: queueHandler = execution ? execQueue : regularQueue
  └── ... repeated 5+ times
```

## Proposed Architecture

### New Hook: `useTaskChat(taskId, contextType)`

A dedicated hook that knows about task context types:

```typescript
// src/hooks/useTaskChat.ts

export function useTaskChat(
  taskId: string,
  contextType: "task" | "task_execution" | "review"
) {
  // Single conversation list query - uses the ACTUAL context type
  const conversations = useQuery({
    queryKey: chatKeys.conversationList(contextType, taskId),
    queryFn: () => chatApi.listConversations(contextType, taskId),
  });

  // Active conversation - fetched by ID (works for any context type)
  const activeConversation = useConversation(activeConversationId);

  // Auto-initialize when context type changes
  useEffect(() => {
    // Reset selection when mode changes
  }, [contextType]);

  // Auto-select latest conversation
  useEffect(() => {
    if (!activeConversationId && conversations.data?.length) {
      setActiveConversation(conversations.data[0].id);
    }
  }, [conversations.data]);

  return {
    conversations,
    activeConversation,
    messages: activeConversation.data?.messages ?? [],
    isLoading: /* unified loading state */,
    sendMessage,
    switchConversation,
    createConversation,
    contextKey: `${contextType}:${taskId}`,
  };
}
```

### Simplified TaskChatPanel (~350 LOC)

```typescript
// src/components/tasks/TaskChatPanel.tsx

export function TaskChatPanel({ taskId, contextType, taskStatus }: Props) {
  // ONE hook call - no branching needed
  const {
    conversations,
    activeConversation,
    messages,
    isLoading,
    sendMessage,
    switchConversation,
    createConversation,
    contextKey,
  } = useTaskChat(taskId, contextType);

  // Unified queue access
  const queuedMessages = useChatStore(selectQueuedMessages(contextKey));
  const isAgentRunning = useChatStore(selectIsAgentRunning(contextKey));

  // Simple handlers - no branching
  const handleQueue = (content: string) => queueMessage(contextKey, content);
  const handleDelete = (id: string) => deleteQueuedMessage(contextKey, id);

  // ... render (same as before, but simpler)
}
```

### Unified Queue in Store

```typescript
// src/stores/chatStore.ts

interface ChatState {
  // REMOVE: executionQueuedMessages (separate system)
  // KEEP: queuedMessages with context-aware keys
  queuedMessages: Record<string, QueuedMessage[]>;
  // Key format: "task:id", "task_execution:id", "review:id"
}
```

## Implementation Steps

### Phase 1: Create useTaskChat hook (non-breaking)

**File:** `src/hooks/useTaskChat.ts` (NEW)

```typescript
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useEffect, useCallback } from "react";
import { chatApi } from "@/api/chat";
import { chatKeys, useConversation } from "./useChat";
import { useChatStore } from "@/stores/chatStore";
import { useAgentEvents } from "./useAgentEvents";
import type { ContextType } from "@/types/chat-conversation";

type TaskContextType = "task" | "task_execution" | "review";

export function useTaskChat(taskId: string, contextType: TaskContextType) {
  const queryClient = useQueryClient();
  const {
    activeConversationId,
    setActiveConversation,
    setAgentRunning,
  } = useChatStore();

  const contextKey = `${contextType}:${taskId}`;

  // Single conversation list query - correct context type
  const conversations = useQuery({
    queryKey: chatKeys.conversationList(contextType, taskId),
    queryFn: () => chatApi.listConversations(contextType, taskId),
  });

  // Active conversation by ID
  const activeConversation = useConversation(activeConversationId);

  // Agent run status
  const agentRunStatus = useQuery({
    queryKey: chatKeys.agentRun(activeConversationId ?? ""),
    queryFn: () => chatApi.getAgentRunStatus(activeConversationId!),
    enabled: !!activeConversationId,
    refetchInterval: 2000,
  });

  // Subscribe to agent events
  useAgentEvents(activeConversationId);

  // Reset activeConversationId when context type changes
  useEffect(() => {
    setActiveConversation(null);
  }, [contextType, taskId, setActiveConversation]);

  // Auto-select latest conversation for this context
  useEffect(() => {
    if (!activeConversationId && conversations.data && conversations.data.length > 0) {
      const latest = conversations.data[0];
      setActiveConversation(latest.id);
    }
  }, [activeConversationId, conversations.data, setActiveConversation]);

  // Sync agent running state
  useEffect(() => {
    const isRunning = agentRunStatus.data?.status === "running";
    setAgentRunning(contextKey, isRunning);
  }, [contextKey, agentRunStatus.data?.status, setAgentRunning]);

  // Unified loading state
  const isLoading =
    conversations.isLoading ||
    (activeConversation.isPending && !!activeConversationId) ||
    (!activeConversationId && conversations.data && conversations.data.length > 0);

  // Send message
  const sendMessage = useMutation({
    mutationFn: async (content: string) => {
      setAgentRunning(contextKey, true);
      return chatApi.sendAgentMessage(contextType, taskId, content);
    },
    onSuccess: (result) => {
      if (activeConversationId) {
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(activeConversationId),
        });
      }
      queryClient.invalidateQueries({
        queryKey: chatKeys.conversationList(contextType, taskId),
      });
    },
    onError: () => {
      setAgentRunning(contextKey, false);
    },
  });

  // Switch conversation
  const switchConversation = useCallback((conversationId: string) => {
    setActiveConversation(conversationId);
    queryClient.invalidateQueries({
      queryKey: chatKeys.conversation(conversationId),
    });
  }, [setActiveConversation, queryClient]);

  // Create new conversation
  const createConversation = useCallback(async () => {
    const newConv = await chatApi.createConversation(contextType, taskId);
    setActiveConversation(newConv.id);
    queryClient.invalidateQueries({
      queryKey: chatKeys.conversationList(contextType, taskId),
    });
    return newConv;
  }, [contextType, taskId, setActiveConversation, queryClient]);

  return {
    // Data
    conversations,
    activeConversation,
    messages: activeConversation.data?.messages ?? [],
    agentRunStatus,
    // State
    isLoading,
    activeConversationId,
    contextKey,
    // Actions
    sendMessage,
    switchConversation,
    createConversation,
  };
}
```

**Estimated:** ~120 LOC

### Phase 2: Migrate TaskChatPanel to useTaskChat

**Changes to `src/components/tasks/TaskChatPanel.tsx`:**

```diff
- import { useChat, chatKeys, useConversation } from "@/hooks/useChat";
+ import { useTaskChat } from "@/hooks/useTaskChat";
+ import { chatKeys } from "@/hooks/useChat";

- const regularChatData = useChat(context);
- const executionConversationsQuery = useQuery({...});
- const reviewConversationsQuery = useQuery({...});
- const directConversationQuery = useConversation(...);
- const conversations = isExecutionMode ? ... : isReviewMode ? ... : ...;
- const activeConversation = (isReviewMode || isExecutionMode) ? ... : ...;

+ const {
+   conversations,
+   activeConversation,
+   messages,
+   isLoading,
+   sendMessage,
+   switchConversation,
+   createConversation,
+   contextKey,
+   activeConversationId,
+ } = useTaskChat(taskId, contextType);
```

**Remove:**
- Lines 241-246: `context` memo (not needed)
- Lines 266-288: Three separate conversation queries
- Lines 290-306: activeConversation override logic
- Lines 313-320: Auto-select effect (moved to hook)
- Lines 440-452: Complex loading state logic

**Estimated removal:** ~200 LOC

### Phase 3: Unify message queues in store

**Changes to `src/stores/chatStore.ts`:**

```diff
interface ChatState {
  queuedMessages: Record<string, QueuedMessage[]>;
- executionQueuedMessages: Record<string, QueuedMessage[]>;
}

interface ChatActions {
  queueMessage: (contextKey: string, content: string) => void;
- queueExecutionMessage: (taskId: string, content: string) => void;
  deleteQueuedMessage: (contextKey: string, id: string) => void;
- deleteExecutionQueuedMessage: (taskId: string, id: string) => void;
}
```

**Update TaskChatPanel:**
```diff
- const handleQueue = useCallback((content: string) => {
-   if (isExecutionMode) {
-     queueExecutionMessage(taskId, content);
-   } else {
-     queueMessage(storeContextKey, content);
-   }
- }, [isExecutionMode, taskId, queueMessage, queueExecutionMessage, storeContextKey]);

+ const handleQueue = useCallback((content: string) => {
+   queueMessage(contextKey, content);
+ }, [contextKey, queueMessage]);
```

### Phase 4: Keep useChat for simple contexts (optional cleanup)

The existing `useChat` hook remains for:
- Ideation view
- Kanban project chat
- Any view that doesn't need context type switching

## File Changes Summary

| File | Action | LOC Change |
|------|--------|------------|
| `src/hooks/useTaskChat.ts` | CREATE | +120 |
| `src/hooks/useChat.ts` | KEEP (minor cleanup) | -20 |
| `src/components/tasks/TaskChatPanel.tsx` | REFACTOR | -200 |
| `src/stores/chatStore.ts` | SIMPLIFY | -40 |
| **Net change** | | **-140 LOC** |

## Testing Strategy

1. **Unit test useTaskChat hook** - Mock API, verify queries run with correct context type
2. **Integration test TaskChatPanel** - Verify review conversations load
3. **Manual test scenarios:**
   - Task in "reviewing" status → see active review conversation
   - Task in "review_passed" status → see review conversation, can chat
   - Task in "executing" status → see execution conversation
   - Switch between tabs → conversations load correctly

## Rollback Plan

If issues arise:
1. `useTaskChat` is additive - can delete without breaking existing code
2. TaskChatPanel changes are isolated - can revert single file
3. Queue unification can be done separately

## Timeline

| Phase | Description | Effort |
|-------|-------------|--------|
| 1 | Create useTaskChat hook | 30 min |
| 2 | Migrate TaskChatPanel | 45 min |
| 3 | Unify queues | 20 min |
| 4 | Test & fix | 30 min |
| **Total** | | ~2 hours |
