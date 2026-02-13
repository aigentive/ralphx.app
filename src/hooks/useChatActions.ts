/**
 * useChatActions — Unified message handling for all chat panels
 *
 * Merges:
 * - useIntegratedChatHandlers (review mode send, ideation auto-naming, execution recovery)
 * - Action parts of useChatPanelHandlers (send, queue, stop, edit, delete)
 *
 * Uses contextType from registry instead of mode booleans.
 */

import { useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useChatStore } from "@/stores/chatStore";
import { chatApi, stopAgent } from "@/api/chat";
import { recoverTaskExecution } from "@/api/recovery";
import { chatKeys } from "@/hooks/useChat";
import { ideationApi } from "@/api/ideation";
import { logger } from "@/lib/logger";
import type { ContextType } from "@/types/chat-conversation";

// ============================================================================
// Types
// ============================================================================

interface UseChatActionsProps {
  /** Resolved context type (from registry or caller) */
  contextType: ContextType;
  /** Context entity ID (task ID, session ID, or project ID) */
  contextId: string;
  /** Store context key for queue/agent state operations */
  storeContextKey: string;
  /** Selected task ID (for execution recovery) */
  selectedTaskId: string | undefined;
  /** Ideation session ID (for auto-naming) */
  ideationSessionId: string | undefined;
  /** Send message mutation from useChat or useTaskChat */
  sendMessage: {
    isPending: boolean;
    mutateAsync: (content: string) => Promise<unknown>;
  };
  /** Current message count (for first-message detection in ideation) */
  messageCount?: number;
}

// ============================================================================
// Hook
// ============================================================================

export function useChatActions({
  contextType,
  contextId,
  storeContextKey,
  selectedTaskId,
  ideationSessionId,
  sendMessage,
  messageCount = 0,
}: UseChatActionsProps) {
  const queryClient = useQueryClient();
  const queueMessage = useChatStore((s) => s.queueMessage);
  const deleteQueuedMessage = useChatStore((s) => s.deleteQueuedMessage);
  const startEditingQueuedMessage = useChatStore((s) => s.startEditingQueuedMessage);
  const setActiveConversation = useChatStore((s) => s.setActiveConversation);
  const setAgentRunning = useChatStore((s) => s.setAgentRunning);

  // Generate a unique ID for queued messages (shared between frontend + backend)
  const generateQueuedMessageId = useCallback(() => {
    return `queued-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
  }, []);

  // ── Send ─────────────────────────────────────────────────────────
  const handleSend = useCallback(
    async (content: string) => {
      if (!content.trim() || sendMessage.isPending) return;

      // Capture first message state before sending (for auto-naming trigger)
      const isFirstIdeationMessage = ideationSessionId && messageCount === 0;

      try {
        // For review mode, send with "review" context type via direct API
        if (contextType === "review" && selectedTaskId) {
          setAgentRunning(storeContextKey, true);

          const result = await chatApi.sendAgentMessage("review", selectedTaskId, content);

          queryClient.invalidateQueries({
            queryKey: chatKeys.conversationList("review", selectedTaskId),
          });

          if (result.conversationId) {
            queryClient.invalidateQueries({
              queryKey: chatKeys.conversation(result.conversationId),
            });
            if (result.isNewConversation) {
              setActiveConversation(result.conversationId);
            }
          }
        } else {
          await sendMessage.mutateAsync(content);
        }

        // Trigger session auto-naming on first ideation message (fire-and-forget)
        if (isFirstIdeationMessage) {
          ideationApi.sessions.spawnSessionNamer(ideationSessionId, content).catch(() => {
            // Silently ignore — session namer is optional
          });
        }
      } catch {
        // Reset agent running state on error for review mode
        if (contextType === "review") {
          setAgentRunning(storeContextKey, false);
        }
      }
    },
    [sendMessage, contextType, selectedTaskId, storeContextKey, setAgentRunning, setActiveConversation, queryClient, ideationSessionId, messageCount]
  );

  // ── Queue ────────────────────────────────────────────────────────
  const handleQueue = useCallback(
    async (content: string) => {
      if (!content.trim()) return;

      const messageId = generateQueuedMessageId();

      // Add to local store immediately for optimistic UI
      queueMessage(storeContextKey, content, messageId);

      // Also queue to backend so it gets processed when agent completes
      try {
        await chatApi.queueAgentMessage(contextType, contextId, content, messageId);
      } catch {
        // Message is already in local store — it just won't be processed by backend
      }
    },
    [queueMessage, storeContextKey, contextType, contextId, generateQueuedMessageId]
  );

  // ── Stop Agent ───────────────────────────────────────────────────
  const handleStopAgent = useCallback(async () => {
    // Always attempt immediate run cancellation
    try {
      await stopAgent(contextType, contextId);
    } catch (err) {
      logger.warn("[chat] Failed to stop agent", { contextType, contextId, error: err });
    }

    // For execution mode, also run recovery so task status reconciles
    if (contextType === "task_execution" && selectedTaskId) {
      try {
        await recoverTaskExecution(selectedTaskId);
      } catch (err) {
        logger.warn("[chat] Failed to recover task execution after stop", { taskId: selectedTaskId, error: err });
      }
    }
  }, [contextType, contextId, selectedTaskId]);

  // ── Delete Queued Message ────────────────────────────────────────
  const handleDeleteQueuedMessage = useCallback(
    async (messageId: string) => {
      // Delete from local store immediately (optimistic)
      deleteQueuedMessage(storeContextKey, messageId);

      // Delete from backend using the same ID
      try {
        await chatApi.deleteQueuedAgentMessage(contextType, contextId, messageId);
      } catch {
        // Silently ignore — local state already updated
      }
    },
    [deleteQueuedMessage, storeContextKey, contextType, contextId]
  );

  // ── Edit Queued Message ──────────────────────────────────────────
  const handleEditQueuedMessage = useCallback(
    async (messageId: string, newContent: string) => {
      // Delete old message from backend
      try {
        await chatApi.deleteQueuedAgentMessage(contextType, contextId, messageId);
      } catch {
        // Silently ignore
      }

      // Delete from local store
      deleteQueuedMessage(storeContextKey, messageId);

      // Generate new ID and queue the edited content
      const newMessageId = generateQueuedMessageId();
      queueMessage(storeContextKey, newContent, newMessageId);

      // Queue to backend with same ID
      try {
        await chatApi.queueAgentMessage(contextType, contextId, newContent, newMessageId);
      } catch {
        // Silently ignore — local state already updated
      }
    },
    [deleteQueuedMessage, queueMessage, contextType, contextId, generateQueuedMessageId, storeContextKey]
  );

  // ── Edit Last Queued ─────────────────────────────────────────────
  const handleEditLastQueued = useCallback(
    (queuedMessages: Array<{ id: string }>) => {
      const lastMessage = queuedMessages[queuedMessages.length - 1];
      if (!lastMessage) return;
      startEditingQueuedMessage(storeContextKey, lastMessage.id);
    },
    [startEditingQueuedMessage, storeContextKey]
  );

  return {
    handleSend,
    handleQueue,
    handleStopAgent,
    handleDeleteQueuedMessage,
    handleEditQueuedMessage,
    handleEditLastQueued,
  };
}
