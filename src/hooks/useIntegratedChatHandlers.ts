/**
 * useIntegratedChatHandlers - Message handling logic for IntegratedChatPanel
 *
 * Handles:
 * - Sending messages
 * - Queueing messages (when agent is running)
 * - Editing/deleting queued messages
 * - Stopping the agent
 */

import { useCallback } from "react";
import { useChatStore } from "@/stores/chatStore";
import { chatApi, stopAgent } from "@/api/chat";
import type { ContextType } from "@/types/chat-conversation";

interface UseIntegratedChatHandlersProps {
  isExecutionMode: boolean;
  selectedTaskId: string | undefined;
  projectId: string;
  ideationSessionId: string | undefined;
  storeContextKey: string;
  sendMessage: {
    isPending: boolean;
    mutateAsync: (content: string) => Promise<unknown>;
  };
}

export function useIntegratedChatHandlers({
  isExecutionMode,
  selectedTaskId,
  projectId,
  ideationSessionId,
  storeContextKey,
  sendMessage,
}: UseIntegratedChatHandlersProps) {
  const {
    queueMessage,
    deleteQueuedMessage,
    startEditingQueuedMessage,
  } = useChatStore();

  // Get current context type and ID for queue operations
  const getQueueContext = useCallback(() => {
    const ctxType: ContextType = isExecutionMode
      ? "task_execution"
      : ideationSessionId
        ? "ideation"
        : selectedTaskId
          ? "task"
          : "project";
    const ctxId = ideationSessionId || selectedTaskId || projectId;
    return { ctxType, ctxId } as const;
  }, [isExecutionMode, ideationSessionId, selectedTaskId, projectId]);

  // Generate a unique ID for queued messages
  const generateQueuedMessageId = useCallback(() => {
    return `queued-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
  }, []);

  // Send message handler
  const handleSend = useCallback(
    async (content: string) => {
      if (!content.trim() || sendMessage.isPending) return;

      try {
        await sendMessage.mutateAsync(content);
      } catch {
        // Error is handled by the mutation
      }
    },
    [sendMessage]
  );

  // Queue message handler (when agent is running)
  // Uses backend queue API for ALL contexts so messages are properly processed
  const handleQueue = useCallback(
    async (content: string) => {
      if (!content.trim()) return;

      const { ctxType, ctxId } = getQueueContext();

      // Generate ID FIRST - this ID will be used by both frontend and backend
      const messageId = generateQueuedMessageId();

      // Add to local store immediately for optimistic UI (using the same ID)
      // storeContextKey now uses context-aware keys (e.g., "task_execution:id" for execution mode)
      queueMessage(storeContextKey, content, messageId);

      // ALSO queue to backend so it gets processed when agent completes
      try {
        await chatApi.queueAgentMessage(ctxType, ctxId, content, messageId);
      } catch {
        // Message is already in local store, which is fine - it just won't be processed by backend
      }
    },
    [queueMessage, storeContextKey, getQueueContext, generateQueuedMessageId]
  );

  // Edit last queued message - now using unified queue with context-aware keys
  const handleEditLastQueued = useCallback(
    (queuedMessages: unknown[]) => {
      const lastMessage = queuedMessages[queuedMessages.length - 1] as { id: string } | undefined;
      if (!lastMessage) return;
      startEditingQueuedMessage(storeContextKey, lastMessage.id);
    },
    [startEditingQueuedMessage, storeContextKey]
  );

  // Delete queued message handler - syncs with backend
  const handleDeleteQueuedMessage = useCallback(
    async (messageId: string) => {
      const { ctxType, ctxId } = getQueueContext();

      // Delete from local store immediately (optimistic) - unified queue with context-aware keys
      deleteQueuedMessage(storeContextKey, messageId);

      // Delete from backend using the same ID
      try {
        await chatApi.deleteQueuedAgentMessage(ctxType, ctxId, messageId);
      } catch (error) {
        console.error("Failed to delete queued message from backend:", error);
      }
    },
    [deleteQueuedMessage, getQueueContext, storeContextKey]
  );

  // Edit queued message handler - delete old and queue new (syncs with backend)
  const handleEditQueuedMessage = useCallback(
    async (messageId: string, newContent: string) => {
      const { ctxType, ctxId } = getQueueContext();

      // Delete old message from backend
      try {
        await chatApi.deleteQueuedAgentMessage(ctxType, ctxId, messageId);
      } catch (error) {
        console.error("Failed to delete old queued message:", error);
      }

      // Delete from local store - unified queue with context-aware keys
      deleteQueuedMessage(storeContextKey, messageId);

      // Generate new ID and queue the edited content
      const newMessageId = generateQueuedMessageId();

      // Add to local store first (optimistic) - unified queue with context-aware keys
      queueMessage(storeContextKey, newContent, newMessageId);

      // Queue to backend with same ID
      try {
        await chatApi.queueAgentMessage(ctxType, ctxId, newContent, newMessageId);
      } catch (error) {
        console.error("Failed to queue edited message to backend:", error);
      }
    },
    [deleteQueuedMessage, queueMessage, getQueueContext, generateQueuedMessageId, storeContextKey]
  );

  // Stop the running agent
  const handleStopAgent = useCallback(async () => {
    const ctxType: ContextType = isExecutionMode
      ? "task_execution"
      : ideationSessionId
        ? "ideation"
        : selectedTaskId
          ? "task"
          : "project";
    const ctxId = ideationSessionId || selectedTaskId || projectId;

    try {
      await stopAgent(ctxType, ctxId);
    } catch (error) {
      console.error("Failed to stop agent:", error);
    }
  }, [isExecutionMode, ideationSessionId, selectedTaskId, projectId]);

  return {
    handleSend,
    handleQueue,
    handleEditLastQueued,
    handleDeleteQueuedMessage,
    handleEditQueuedMessage,
    handleStopAgent,
  };
}
