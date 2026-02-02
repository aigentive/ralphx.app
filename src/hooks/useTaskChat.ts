/**
 * useTaskChat hook - Context-aware chat for task-related conversations
 *
 * A dedicated hook that properly handles task, task_execution, and review context types.
 * Unlike useChat which always returns contextType="task" for task_detail views,
 * this hook takes the context type explicitly and fetches the correct conversations.
 *
 * This simplifies TaskChatPanel by removing 3-way branching logic for different modes.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useEffect, useCallback, useRef, useMemo } from "react";
import { chatApi, type SendAgentMessageResult, type ChatMessageResponse } from "@/api/chat";
import type { ChatConversation, AgentRun } from "@/types/chat-conversation";
import { useChatStore } from "@/stores/chatStore";
import { chatKeys, useConversation } from "./useChat";
import { useAgentEvents } from "./useAgentEvents";
import { useTaskStateTransitions } from "./useTaskStateTransitions";

/**
 * Task-specific context types
 * - task: Regular task discussion/planning
 * - task_execution: Worker execution conversation
 * - review: Review process conversation
 * - merge: Merge agent conflict resolution conversation
 */
export type TaskContextType = "task" | "task_execution" | "review" | "merge";

/**
 * Build a context key string for task-related contexts
 * Format: ${contextType}:${taskId}
 */
function buildTaskContextKey(contextType: TaskContextType, taskId: string): string {
  return `${contextType}:${taskId}`;
}

/**
 * Hook for task-specific chat functionality with context-aware messaging
 *
 * Unlike the general useChat hook, this hook:
 * - Takes the context type explicitly (not derived from view)
 * - Uses the correct context type for conversation queries
 * - Resets active conversation when context type changes
 * - Builds context keys in the format `${contextType}:${taskId}`
 * - Supports historical message filtering via historicalStatus
 *
 * @param taskId - The task ID
 * @param contextType - The context type (task, task_execution, or review)
 * @param historicalStatus - Optional status to filter messages by time period
 * @returns Object with conversations, messages, loading state, and actions
 *
 * @example
 * ```tsx
 * const {
 *   conversations,
 *   activeConversation,
 *   messages,
 *   isLoading,
 *   sendMessage,
 *   switchConversation,
 *   createConversation,
 *   contextKey,
 *   isHistoricalMode,
 * } = useTaskChat(taskId, "review", "executing");
 * ```
 */
export function useTaskChat(taskId: string, contextType: TaskContextType, historicalStatus?: string) {
  const queryClient = useQueryClient();
  const contextKey = buildTaskContextKey(contextType, taskId);
  const isHistoricalMode = !!historicalStatus;
  console.log(`[useTaskChat] taskId=${taskId}, contextType=${contextType}, contextKey=${contextKey}, historicalStatus=${historicalStatus}`);

  // Fetch state transitions for historical message filtering
  const stateTransitions = useTaskStateTransitions(isHistoricalMode ? taskId : undefined);

  const {
    activeConversationId,
    setActiveConversation,
    setAgentRunning,
  } = useChatStore();

  // Fetch conversations for this specific context type and task
  const conversations = useQuery<ChatConversation[], Error>({
    queryKey: chatKeys.conversationList(contextType, taskId),
    queryFn: async () => {
      console.log(`[useTaskChat] Fetching conversations: contextType=${contextType}, taskId=${taskId}`);
      const result = await chatApi.listConversations(contextType, taskId);
      console.log(`[useTaskChat] Fetched ${result.length} conversations`);
      return result;
    },
  });

  // Fetch active conversation with messages
  const activeConversation = useConversation(activeConversationId);

  // Fetch agent run status for the active conversation
  const agentRunStatus = useQuery<AgentRun | null, Error>({
    queryKey: chatKeys.agentRun(activeConversationId ?? ""),
    queryFn: () => {
      if (!activeConversationId) {
        return null;
      }
      return chatApi.getAgentRunStatus(activeConversationId);
    },
    enabled: !!activeConversationId,
    refetchInterval: (query) => {
      // Poll every 2 seconds if agent is running
      const agentRun = query.state.data;
      return agentRun?.status === "running" ? 2000 : false;
    },
  });

  // Subscribe to agent events for real-time updates
  useAgentEvents(activeConversationId);

  // Track previous context to detect changes
  const prevContextRef = useRef<string | null>(null);

  // Reset activeConversationId and clear stale agent state when context type or task changes
  useEffect(() => {
    const currentContext = `${contextType}:${taskId}`;
    if (prevContextRef.current !== null && prevContextRef.current !== currentContext) {
      // Context changed - clear agent state on OLD context key and reset conversation
      // This prevents stale isAgentRunning entries from previous context types
      setAgentRunning(prevContextRef.current, false);
      setActiveConversation(null);
    }
    prevContextRef.current = currentContext;
  }, [contextType, taskId, setActiveConversation, setAgentRunning]);

  // Auto-select the most recent conversation for this context
  // Use a ref to track initialization and prevent infinite loops
  const hasAutoSelectedRef = useRef(false);

  useEffect(() => {
    // Reset auto-select flag when context changes
    hasAutoSelectedRef.current = false;
  }, [contextType, taskId]);

  useEffect(() => {
    // CRITICAL: Check for stale activeConversationId FIRST, before checking hasAutoSelectedRef.
    // The activeConversationId in the store is global - it persists across context switches.
    // If it doesn't belong to the current context's conversations, it's stale and must be reset.
    if (activeConversationId && conversations.data && conversations.data.length > 0) {
      const belongsToContext = conversations.data.some(c => c.id === activeConversationId);
      if (!belongsToContext) {
        console.log(`[useTaskChat] Stale activeConversationId=${activeConversationId} not in context ${contextKey}, resetting`);
        // Reset both the ID and the flag so auto-select can run
        hasAutoSelectedRef.current = false;
        setActiveConversation(null);
        return; // Will re-run on next render with null activeConversationId
      }
    }

    // Only auto-select once per context
    if (hasAutoSelectedRef.current) {
      return;
    }

    if (!activeConversationId && conversations.data && conversations.data.length > 0) {
      // Sort by most recent activity
      const sorted = [...conversations.data].sort((a, b) => {
        const aTime = a.lastMessageAt || a.createdAt;
        const bTime = b.lastMessageAt || b.createdAt;
        return new Date(bTime).getTime() - new Date(aTime).getTime();
      });
      const mostRecent = sorted[0];

      if (mostRecent) {
        console.log(`[useTaskChat] Auto-selecting conversation ${mostRecent.id} for context ${contextKey}`);
        hasAutoSelectedRef.current = true;
        setActiveConversation(mostRecent.id);
      }
    }
  }, [activeConversationId, conversations.data, setActiveConversation, contextKey]);

  // Sync agent running state based on backend status
  const isRunning = agentRunStatus.data?.status === "running";

  useEffect(() => {
    // Only set to true based on backend status (for initial load recovery)
    // Don't set to false here - let the agent:run_completed event handle that
    if (isRunning) {
      setAgentRunning(contextKey, true);
    }
  }, [contextKey, isRunning, setAgentRunning]);

  // Unified loading state
  const isLoading =
    conversations.isLoading ||
    (activeConversation.isPending && !!activeConversationId) ||
    (!activeConversationId && conversations.data && conversations.data.length > 0) ||
    (isHistoricalMode && stateTransitions.isLoading);

  // Filter messages by historical status time period
  const filteredMessages = useMemo((): ChatMessageResponse[] => {
    const allMessages = activeConversation.data?.messages ?? [];

    // If not in historical mode, return all messages
    if (!isHistoricalMode || !historicalStatus) {
      return allMessages;
    }

    // Need state transitions to determine time range
    if (!stateTransitions.data || stateTransitions.data.length === 0) {
      return allMessages;
    }

    // Find the time range when the task was in the historical status
    // First, find when the task entered this status
    const entryTransition = stateTransitions.data.find(t => t.toStatus === historicalStatus);
    if (!entryTransition) {
      // Status never entered, show no messages
      return [];
    }

    const startTime = new Date(entryTransition.timestamp).getTime();

    // Find when the task left this status (next transition after entry)
    const transitionIndex = stateTransitions.data.findIndex(t => t.toStatus === historicalStatus);
    const exitTransition = stateTransitions.data[transitionIndex + 1];
    // If no exit transition exists, the task is still in this state - include all messages after entry
    // Use Number.MAX_SAFE_INTEGER as a stable "infinity" value for comparison
    const endTime = exitTransition ? new Date(exitTransition.timestamp).getTime() : Number.MAX_SAFE_INTEGER;

    // Filter messages within this time range
    return allMessages.filter(msg => {
      const msgTime = new Date(msg.createdAt).getTime();
      return msgTime >= startTime && msgTime <= endTime;
    });
  }, [activeConversation.data?.messages, isHistoricalMode, historicalStatus, stateTransitions.data]);

  // Send message mutation
  const sendMessage = useMutation<SendAgentMessageResult, Error, string>({
    mutationFn: async (content: string) => {
      // Set agent running immediately so subsequent messages get queued
      setAgentRunning(contextKey, true);
      return chatApi.sendAgentMessage(contextType, taskId, content);
    },
    onSuccess: (result) => {
      // Invalidate active conversation to refetch messages
      if (activeConversationId) {
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(activeConversationId),
        });
      }

      // Invalidate conversations list to update message counts
      queryClient.invalidateQueries({
        queryKey: chatKeys.conversationList(contextType, taskId),
      });

      // If this is a new conversation, set it as active
      if (result.isNewConversation) {
        setActiveConversation(result.conversationId);
      }
    },
    onError: () => {
      // Reset agent running state on error
      setAgentRunning(contextKey, false);
    },
  });

  // Switch conversation
  const switchConversation = useCallback(
    (conversationId: string) => {
      setActiveConversation(conversationId);

      // Invalidate the conversation query to ensure fresh data is fetched
      queryClient.invalidateQueries({
        queryKey: chatKeys.conversation(conversationId),
      });
    },
    [setActiveConversation, queryClient]
  );

  // Create new conversation
  const createConversation = useCallback(async () => {
    const newConversation = await chatApi.createConversation(contextType, taskId);
    setActiveConversation(newConversation.id);

    // Invalidate conversations list
    queryClient.invalidateQueries({
      queryKey: chatKeys.conversationList(contextType, taskId),
    });

    return newConversation;
  }, [contextType, taskId, setActiveConversation, queryClient]);

  return {
    // Data
    conversations,
    activeConversation,
    messages: filteredMessages,
    agentRunStatus,
    // State
    isLoading,
    isHistoricalMode,
    activeConversationId,
    contextKey,
    contextType,
    // Actions
    sendMessage,
    switchConversation,
    createConversation,
  };
}
