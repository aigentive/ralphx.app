/**
 * useChat hook - TanStack Query wrapper for chat messages
 *
 * Provides hooks for fetching and sending chat messages based on context.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { chatApi, type ChatMessageResponse, type SendMessageInput } from "@/api/chat";
import type { ChatContext } from "@/types/chat";
import { ideationKeys } from "./useIdeation";

/**
 * Query key factory for chat
 */
export const chatKeys = {
  all: ["chat"] as const,
  messages: () => [...chatKeys.all, "messages"] as const,
  sessionMessages: (sessionId: string) =>
    [...chatKeys.messages(), "session", sessionId] as const,
  projectMessages: (projectId: string) =>
    [...chatKeys.messages(), "project", projectId] as const,
  taskMessages: (taskId: string) =>
    [...chatKeys.messages(), "task", taskId] as const,
};

/**
 * Get the appropriate query key for a chat context
 */
function getQueryKeyForContext(context: ChatContext) {
  switch (context.view) {
    case "ideation":
      if (context.ideationSessionId) {
        return chatKeys.sessionMessages(context.ideationSessionId);
      }
      return chatKeys.projectMessages(context.projectId);
    case "task_detail":
      if (context.selectedTaskId) {
        return chatKeys.taskMessages(context.selectedTaskId);
      }
      return chatKeys.projectMessages(context.projectId);
    case "kanban":
      if (context.selectedTaskId) {
        return chatKeys.taskMessages(context.selectedTaskId);
      }
      return chatKeys.projectMessages(context.projectId);
    default:
      return chatKeys.projectMessages(context.projectId);
  }
}

/**
 * Fetch messages based on context
 */
async function fetchMessagesForContext(
  context: ChatContext
): Promise<ChatMessageResponse[]> {
  switch (context.view) {
    case "ideation":
      if (context.ideationSessionId) {
        return chatApi.getSessionMessages(context.ideationSessionId);
      }
      return chatApi.getProjectMessages(context.projectId);
    case "task_detail":
      if (context.selectedTaskId) {
        return chatApi.getTaskMessages(context.selectedTaskId);
      }
      return chatApi.getProjectMessages(context.projectId);
    case "kanban":
      if (context.selectedTaskId) {
        return chatApi.getTaskMessages(context.selectedTaskId);
      }
      return chatApi.getProjectMessages(context.projectId);
    default:
      return chatApi.getProjectMessages(context.projectId);
  }
}

/**
 * Hook to fetch chat messages for a specific context
 *
 * @param context - The chat context (view, projectId, sessionId, taskId)
 * @returns TanStack Query result with messages array
 *
 * @example
 * ```tsx
 * const { data: messages, isLoading } = useChatMessages({
 *   view: "ideation",
 *   projectId: "project-123",
 *   ideationSessionId: "session-123",
 * });
 * ```
 */
export function useChatMessages(context: ChatContext) {
  return useQuery<ChatMessageResponse[], Error>({
    queryKey: getQueryKeyForContext(context),
    queryFn: () => fetchMessagesForContext(context),
  });
}

/**
 * Input for sending a message - can be just content string or full options
 */
type SendMessageMutationInput = string | { content: string } & Omit<SendMessageInput, "content">;

/**
 * Hook for chat functionality with both fetching and sending
 *
 * @param context - The chat context
 * @returns Object with messages query and sendMessage mutation
 *
 * @example
 * ```tsx
 * const { messages, sendMessage } = useChat({
 *   view: "ideation",
 *   projectId: "project-123",
 *   ideationSessionId: "session-123",
 * });
 *
 * const handleSend = async (text: string) => {
 *   await sendMessage.mutateAsync(text);
 * };
 *
 * // Or with options
 * const handleReply = async (text: string, parentId: string) => {
 *   await sendMessage.mutateAsync({
 *     content: text,
 *     parentMessageId: parentId,
 *   });
 * };
 * ```
 */
export function useChat(context: ChatContext) {
  const queryClient = useQueryClient();
  const messages = useChatMessages(context);

  const sendMessage = useMutation<ChatMessageResponse, Error, SendMessageMutationInput>({
    mutationFn: async (input) => {
      if (typeof input === "string") {
        return chatApi.sendMessageWithContext(context, input, undefined);
      }
      const { content, ...options } = input;
      return chatApi.sendMessageWithContext(context, content, options);
    },
    onSuccess: () => {
      // Invalidate messages for the current context
      queryClient.invalidateQueries({
        queryKey: getQueryKeyForContext(context),
      });

      // If in ideation context, also invalidate session data
      if (context.view === "ideation" && context.ideationSessionId) {
        queryClient.invalidateQueries({
          queryKey: ideationKeys.sessionWithData(context.ideationSessionId),
        });
      }
    },
  });

  return {
    messages,
    sendMessage,
  };
}
