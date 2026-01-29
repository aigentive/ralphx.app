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
import { useEffect, useCallback, useRef } from "react";
import { chatApi, type SendAgentMessageResult } from "@/api/chat";
import type { ChatConversation, AgentRun } from "@/types/chat-conversation";
import { useChatStore } from "@/stores/chatStore";
import { chatKeys, useConversation } from "./useChat";
import { useAgentEvents } from "./useAgentEvents";

/**
 * Task-specific context types
 * - task: Regular task discussion/planning
 * - task_execution: Worker execution conversation
 * - review: Review process conversation
 */
export type TaskContextType = "task" | "task_execution" | "review";

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
 *
 * @param taskId - The task ID
 * @param contextType - The context type (task, task_execution, or review)
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
 * } = useTaskChat(taskId, "review");
 * ```
 */
export function useTaskChat(taskId: string, contextType: TaskContextType) {
  const queryClient = useQueryClient();
  const contextKey = buildTaskContextKey(contextType, taskId);

  const {
    activeConversationId,
    setActiveConversation,
    setAgentRunning,
  } = useChatStore();

  // Fetch conversations for this specific context type and task
  const conversations = useQuery<ChatConversation[], Error>({
    queryKey: chatKeys.conversationList(contextType, taskId),
    queryFn: () => chatApi.listConversations(contextType, taskId),
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

  // Reset activeConversationId when context type or task changes
  useEffect(() => {
    const currentContext = `${contextType}:${taskId}`;
    if (prevContextRef.current !== null && prevContextRef.current !== currentContext) {
      // Context changed, reset active conversation
      setActiveConversation(null);
    }
    prevContextRef.current = currentContext;
  }, [contextType, taskId, setActiveConversation]);

  // Auto-select the most recent conversation for this context
  // Use a ref to track initialization and prevent infinite loops
  const hasAutoSelectedRef = useRef(false);

  useEffect(() => {
    // Reset auto-select flag when context changes
    hasAutoSelectedRef.current = false;
  }, [contextType, taskId]);

  useEffect(() => {
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
        hasAutoSelectedRef.current = true;
        setActiveConversation(mostRecent.id);
      }
    }
  }, [activeConversationId, conversations.data, setActiveConversation]);

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
    (!activeConversationId && conversations.data && conversations.data.length > 0);

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
    messages: activeConversation.data?.messages ?? [],
    agentRunStatus,
    // State
    isLoading,
    activeConversationId,
    contextKey,
    contextType,
    // Actions
    sendMessage,
    switchConversation,
    createConversation,
  };
}
