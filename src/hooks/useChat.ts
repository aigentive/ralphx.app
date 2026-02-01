/**
 * useChat hook - TanStack Query wrapper for context-aware chat
 *
 * Provides hooks for fetching and sending chat messages based on context.
 * Supports conversation management, agent run status, and real-time updates.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useEffect, useCallback, useRef } from "react";
import { chatApi, type ChatMessageResponse, type SendAgentMessageResult } from "@/api/chat";
import type { ChatContext } from "@/types/chat";
import type { ChatConversation, AgentRun, ContextType } from "@/types/chat-conversation";
import { useChatStore } from "@/stores/chatStore";
import { ideationKeys } from "./useIdeation";
import { useAgentEvents } from "./useAgentEvents";

/**
 * Build a context key string from context type and ID
 * This matches the getContextKey format in chatStore
 */
function buildContextKey(contextType: ContextType, contextId: string): string {
  switch (contextType) {
    case "ideation":
      return `session:${contextId}`;
    case "task":
    case "task_execution":
      return `task:${contextId}`;
    case "review":
      return `review:${contextId}`;
    case "project":
      return `project:${contextId}`;
    default:
      return `project:${contextId}`;
  }
}

/**
 * Query key factory for chat
 */
export const chatKeys = {
  all: ["chat"] as const,
  messages: () => [...chatKeys.all, "messages"] as const,
  conversations: () => [...chatKeys.all, "conversations"] as const,
  conversation: (conversationId: string) =>
    [...chatKeys.conversations(), conversationId] as const,
  conversationList: (contextType: ContextType, contextId: string) =>
    [...chatKeys.conversations(), contextType, contextId] as const,
  agentRun: (conversationId: string) =>
    [...chatKeys.all, "agent-run", conversationId] as const,
  // Legacy keys for backward compatibility
  sessionMessages: (sessionId: string) =>
    [...chatKeys.messages(), "session", sessionId] as const,
  projectMessages: (projectId: string) =>
    [...chatKeys.messages(), "project", projectId] as const,
  taskMessages: (taskId: string) =>
    [...chatKeys.messages(), "task", taskId] as const,
};

/**
 * Get context type and ID from ChatContext
 *
 * NOTE: This function currently doesn't distinguish between 'task', 'task_execution', and 'review'
 * context types when view='task_detail'. Components like TaskChatPanel handle this distinction
 * by directly querying conversations with the appropriate contextType based on task state.
 */
function getContextTypeAndId(context: ChatContext): {
  contextType: ContextType;
  contextId: string;
} {
  switch (context.view) {
    case "ideation":
      if (!context.ideationSessionId) {
        throw new Error("Ideation context requires ideationSessionId");
      }
      return { contextType: "ideation", contextId: context.ideationSessionId };
    case "task_detail":
      if (!context.selectedTaskId) {
        throw new Error("Task detail context requires selectedTaskId");
      }
      // Returns 'task' contextType by default. Components should query conversations
      // with 'task_execution' or 'review' contextType directly when needed based on task state.
      return { contextType: "task", contextId: context.selectedTaskId };
    case "kanban":
      if (context.selectedTaskId) {
        return { contextType: "task", contextId: context.selectedTaskId };
      }
      return { contextType: "project", contextId: context.projectId };
    default:
      return { contextType: "project", contextId: context.projectId };
  }
}

/**
 * Hook to fetch conversations for a context
 */
export function useConversations(context: ChatContext) {
  const { contextType, contextId } = getContextTypeAndId(context);

  return useQuery<ChatConversation[], Error>({
    queryKey: chatKeys.conversationList(contextType, contextId),
    queryFn: () => chatApi.listConversations(contextType, contextId),
  });
}

/**
 * Hook to fetch a single conversation with messages
 */
export function useConversation(conversationId: string | null) {
  return useQuery<
    { conversation: ChatConversation; messages: ChatMessageResponse[] },
    Error
  >({
    queryKey: chatKeys.conversation(conversationId ?? ""),
    queryFn: () => {
      if (!conversationId) {
        throw new Error("Conversation ID is required");
      }
      return chatApi.getConversation(conversationId);
    },
    enabled: !!conversationId,
  });
}

/**
 * Hook to fetch agent run status for a conversation
 */
export function useAgentRunStatus(conversationId: string | null) {
  return useQuery<AgentRun | null, Error>({
    queryKey: chatKeys.agentRun(conversationId ?? ""),
    queryFn: () => {
      if (!conversationId) {
        return null;
      }
      return chatApi.getAgentRunStatus(conversationId);
    },
    enabled: !!conversationId,
    refetchInterval: (query) => {
      // Poll every 2 seconds if agent is running
      const agentRun = query.state.data;
      return agentRun?.status === "running" ? 2000 : false;
    },
    // Prevent excessive refetching when not polling
    staleTime: 10 * 1000, // 10 seconds
    refetchOnWindowFocus: false,
    refetchOnMount: "always", // Always check on mount for initial state
  });
}

/**
 * Hook for chat functionality with context-aware messaging
 *
 * @param context - The chat context
 * @returns Object with messages query, sendMessage mutation, and conversation management
 *
 * @example
 * ```tsx
 * const {
 *   messages,
 *   conversations,
 *   activeConversation,
 *   agentRunStatus,
 *   sendMessage,
 *   switchConversation,
 *   createConversation,
 * } = useChat({
 *   view: "ideation",
 *   projectId: "project-123",
 *   ideationSessionId: "session-123",
 * });
 * ```
 */
export function useChat(context: ChatContext) {
  const queryClient = useQueryClient();
  const { contextType, contextId } = getContextTypeAndId(context);
  const contextKey = buildContextKey(contextType, contextId);

  const {
    activeConversationId,
    setActiveConversation,
    setAgentRunning,
  } = useChatStore();

  // Fetch conversations for this context
  const conversations = useConversations(context);

  // Fetch active conversation with messages
  const activeConversation = useConversation(activeConversationId);

  // Fetch agent run status
  const agentRunStatus = useAgentRunStatus(activeConversationId);

  // Update agent running state when status changes
  // NOTE: This only sets to true on initial load (when backend shows agent is running).
  // The false state is handled by the agent:run_completed event to avoid race conditions.
  const isRunning = agentRunStatus.data?.status === "running";
  const isFailed = agentRunStatus.data?.status === "failed";
  const errorMessage = agentRunStatus.data?.errorMessage;

  useEffect(() => {
    // Only set to true based on backend status (for initial load recovery)
    // Don't set to false here - let the agent:run_completed event handle that
    if (isRunning) {
      setAgentRunning(contextKey, true);
    }
  }, [contextKey, isRunning, setAgentRunning]);

  // Show error toast when a failed run is detected (e.g., when user comes back)
  // Track which errors we've shown to avoid duplicate toasts
  const shownErrorRef = useRef<string | null>(null);
  useEffect(() => {
    if (isFailed && errorMessage && shownErrorRef.current !== agentRunStatus.data?.id) {
      // Mark this error as shown
      shownErrorRef.current = agentRunStatus.data?.id ?? null;

      // Import toast dynamically to avoid circular deps
      import("sonner").then(({ toast }) => {
        toast.error("Previous agent run failed", {
          description: errorMessage.slice(0, 200),
          duration: 10000,
        });
      });
    }
  }, [isFailed, errorMessage, agentRunStatus.data?.id]);

  // Send message mutation
  const sendMessage = useMutation<SendAgentMessageResult, Error, string>({
    mutationFn: async (content: string) => {
      // Set agent running immediately so subsequent messages get queued
      setAgentRunning(contextKey, true);
      return chatApi.sendAgentMessage(contextType, contextId, content);
    },
    onSuccess: () => {
      // Invalidate active conversation to refetch messages
      if (activeConversationId) {
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(activeConversationId),
        });
      }

      // Invalidate conversations list to update message counts
      queryClient.invalidateQueries({
        queryKey: chatKeys.conversationList(contextType, contextId),
      });

      // If in ideation context, also invalidate session data
      if (context.view === "ideation" && context.ideationSessionId) {
        queryClient.invalidateQueries({
          queryKey: ideationKeys.sessionWithData(context.ideationSessionId),
        });
      }
    },
    onError: () => {
      // Reset agent running state on error
      setAgentRunning(contextKey, false);
    },
  });

  // Create new conversation mutation
  const createConversationMutation = useMutation<ChatConversation, Error, void>(
    {
      mutationFn: async () => {
        return chatApi.createConversation(contextType, contextId);
      },
      onSuccess: (newConversation) => {
        // Set as active conversation
        setActiveConversation(newConversation.id);

        // Invalidate conversations list
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversationList(contextType, contextId),
        });
      },
    }
  );

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
    await createConversationMutation.mutateAsync();
  }, [createConversationMutation]);

  // Subscribe to agent events for real-time updates
  useAgentEvents(activeConversationId);

  // Initialize active conversation if none is set
  // Use a ref to track initialization and prevent infinite loops
  const hasInitializedRef = useRef(false);

  useEffect(() => {
    // Only initialize once per context change
    if (hasInitializedRef.current) {
      return;
    }

    if (!activeConversationId && conversations.data && conversations.data.length > 0) {
      // IMPORTANT: Create a copy before sorting to avoid mutating React Query's cached data
      const sorted = [...conversations.data].sort((a, b) => {
        const aTime = a.lastMessageAt || a.createdAt;
        const bTime = b.lastMessageAt || b.createdAt;
        return new Date(bTime).getTime() - new Date(aTime).getTime();
      });
      const mostRecent = sorted[0];

      if (mostRecent) {
        hasInitializedRef.current = true;
        setActiveConversation(mostRecent.id);
      }
    }
  }, [activeConversationId, conversations.data, setActiveConversation]);

  // Reset initialization flag when context changes
  useEffect(() => {
    hasInitializedRef.current = false;
  }, [contextType, contextId]);

  return {
    // Messages from active conversation
    messages: activeConversation,
    // All conversations for this context
    conversations,
    // Active conversation data
    activeConversation,
    // Agent run status
    agentRunStatus,
    // Mutations
    sendMessage,
    // Conversation management
    switchConversation,
    createConversation,
    // Context key for queue/agent state operations
    contextKey,
    // Context info
    contextType,
    contextId,
  };
}
