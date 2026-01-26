/**
 * useChat hook - TanStack Query wrapper for context-aware chat
 *
 * Provides hooks for fetching and sending chat messages based on context.
 * Supports conversation management, agent run status, and real-time updates.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useEffect, useCallback } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { chatApi, type ChatMessageResponse } from "@/api/chat";
import type { ChatContext } from "@/types/chat";
import type { ChatConversation, AgentRun, ContextType } from "@/types/chat-conversation";
import { useChatStore } from "@/stores/chatStore";
import { ideationKeys } from "./useIdeation";

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

  const {
    activeConversationId,
    setActiveConversation,
    setAgentRunning,
    queuedMessages,
    processQueue,
  } = useChatStore();

  // Fetch conversations for this context
  const conversations = useConversations(context);

  // Fetch active conversation with messages
  const activeConversation = useConversation(activeConversationId);

  // Fetch agent run status
  const agentRunStatus = useAgentRunStatus(activeConversationId);

  // Update agent running state when status changes
  useEffect(() => {
    if (agentRunStatus.data) {
      setAgentRunning(agentRunStatus.data.status === "running");
    } else {
      setAgentRunning(false);
    }
  }, [agentRunStatus.data, setAgentRunning]);

  // Send message mutation
  const sendMessage = useMutation<ChatMessageResponse, Error, string>({
    mutationFn: async (content: string) => {
      return chatApi.sendContextMessage(contextType, contextId, content);
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
    },
    [setActiveConversation]
  );

  // Create new conversation
  const createConversation = useCallback(async () => {
    await createConversationMutation.mutateAsync();
  }, [createConversationMutation]);

  // Subscribe to Tauri events for real-time updates
  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    (async () => {
      // Listen for chat chunks (streaming text)
      const chunkUnlisten = await listen<{
        conversation_id: string;
        message_id: string;
        text: string;
      }>("chat:chunk", (event) => {
        const { conversation_id } = event.payload;

        // If this is for the active conversation, invalidate to refetch
        if (conversation_id === activeConversationId) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(activeConversationId),
          });
        }
      });
      unlisteners.push(chunkUnlisten);

      // Listen for tool calls
      const toolCallUnlisten = await listen<{
        conversation_id: string;
        tool_name: string;
        args: unknown;
        result: unknown;
      }>("chat:tool_call", (event) => {
        const { conversation_id } = event.payload;

        // If this is for the active conversation, invalidate to refetch
        if (conversation_id === activeConversationId) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(activeConversationId),
          });
        }
      });
      unlisteners.push(toolCallUnlisten);

      // Listen for message created
      const messageCreatedUnlisten = await listen<{
        conversation_id: string;
        message_id: string;
      }>("chat:message_created", (event) => {
        const { conversation_id } = event.payload;

        // If this is for the active conversation, invalidate to refetch
        if (conversation_id === activeConversationId) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(activeConversationId),
          });
        }
      });
      unlisteners.push(messageCreatedUnlisten);

      // Listen for run completion
      const runCompletedUnlisten = await listen<{
        conversation_id: string;
        status: string;
      }>("chat:run_completed", async (event) => {
        const { conversation_id } = event.payload;

        // Update agent running state
        setAgentRunning(false);

        // Invalidate agent run status
        if (conversation_id === activeConversationId) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.agentRun(activeConversationId),
          });

          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(activeConversationId),
          });
        }

        // Process queue: send first queued message if any
        if (queuedMessages.length > 0) {
          const firstMessage = queuedMessages[0];
          if (firstMessage) {
            // Process queue (removes first message)
            await processQueue();

            // Send the message
            try {
              await sendMessage.mutateAsync(firstMessage.content);
            } catch (error) {
              console.error("Failed to send queued message:", error);
            }
          }
        }
      });
      unlisteners.push(runCompletedUnlisten);
    })();

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [
    activeConversationId,
    queryClient,
    setAgentRunning,
    queuedMessages,
    processQueue,
    sendMessage,
  ]);

  // Initialize active conversation if none is set
  useEffect(() => {
    if (!activeConversationId && conversations.data && conversations.data.length > 0) {
      // Set the most recent conversation as active
      const mostRecent = conversations.data.sort((a, b) => {
        const aTime = a.lastMessageAt || a.createdAt;
        const bTime = b.lastMessageAt || b.createdAt;
        return new Date(bTime).getTime() - new Date(aTime).getTime();
      })[0];

      if (mostRecent) {
        setActiveConversation(mostRecent.id);
      }
    }
  }, [activeConversationId, conversations.data, setActiveConversation]);

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
  };
}
