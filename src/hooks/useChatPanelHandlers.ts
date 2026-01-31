/**
 * useChatPanelHandlers - Event handlers and queue management for ChatPanel
 *
 * Extracted from ChatPanel.tsx to reduce component size.
 * Contains: error handling, agent control, message queue operations, and Tauri event subscriptions.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import type { UseMutationResult } from "@tanstack/react-query";
import { chatApi, stopAgent, type SendAgentMessageResult } from "@/api/chat";
import { chatKeys } from "@/hooks/useChat";
import { useChatStore } from "@/stores/chatStore";
import type { ChatContext } from "@/types/chat";
import type { ToolCall } from "@/components/Chat/ToolCallIndicator";
import { toast } from "sonner";

interface UseChatPanelHandlersProps {
  context: ChatContext;
  isExecutionMode: boolean;
  contextKey: string;
  activeConversationId: string | null;
  sendMessage: UseMutationResult<SendAgentMessageResult, Error, string, unknown>;
  queuedMessages: Array<{ id: string; content: string }>;
  messagesEndRef: React.RefObject<HTMLDivElement | null>;
}

export function useChatPanelHandlers({
  context,
  isExecutionMode,
  contextKey,
  activeConversationId,
  sendMessage,
  queuedMessages,
  messagesEndRef,
}: UseChatPanelHandlersProps) {
  const queryClient = useQueryClient();
  const chatStore = useChatStore();
  const {
    queueMessage,
    deleteQueuedMessage,
    startEditingQueuedMessage,
  } = chatStore;

  // Streaming tool calls - accumulated during agent execution
  const [streamingToolCalls, setStreamingToolCalls] = useState<ToolCall[]>([]);
  // Ref for activeConversationId so event listeners always have current value
  const activeConversationIdRef = useRef(activeConversationId);

  useEffect(() => {
    activeConversationIdRef.current = activeConversationId;
  }, [activeConversationId]);

  // Unified error handler for chat operations
  const logError = useCallback((operation: string, error: unknown, showToast = false) => {
    console.error(`ChatPanel - ${operation}:`, error);
    if (showToast) {
      toast.error(`Failed to ${operation.toLowerCase()}. Please try again.`);
    }
  }, []);

  // Stop the running agent
  const handleStopAgent = useCallback(async () => {
    const ctxType = isExecutionMode
      ? "task_execution"
      : context.view === "ideation"
        ? "ideation"
        : context.view === "task_detail"
          ? "task"
          : "project";
    const ctxId = context.view === "ideation" && context.ideationSessionId
      ? context.ideationSessionId
      : context.selectedTaskId || context.projectId;

    try {
      await stopAgent(ctxType, ctxId);
      // Clear streaming tool calls when agent is stopped
      setStreamingToolCalls([]);
    } catch (error) {
      logError("stop agent", error, true);
    }
  }, [isExecutionMode, context, logError]);

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

  // Get current context type and ID for queue operations
  const getQueueContext = useCallback(() => {
    const ctxType = isExecutionMode
      ? "task_execution"
      : context.view === "ideation"
        ? "ideation"
        : context.view === "task_detail"
          ? "task"
          : "project";
    const ctxId = context.view === "ideation" && context.ideationSessionId
      ? context.ideationSessionId
      : context.selectedTaskId || context.projectId;
    return { ctxType, ctxId } as const;
  }, [isExecutionMode, context]);

  // Generate a unique ID for queued messages
  const generateQueuedMessageId = useCallback(() => {
    return `queued-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
  }, []);

  // Queue message handler (when agent is running)
  // Uses backend queue API for ALL contexts so messages are properly processed
  const handleQueue = useCallback(
    async (content: string) => {
      if (!content.trim()) return;

      const { ctxType, ctxId } = getQueueContext();

      // Generate ID FIRST - this ID will be used by both frontend and backend
      const messageId = generateQueuedMessageId();

      // Add to local store immediately for optimistic UI (using the same ID)
      // contextKey now uses context-aware keys (e.g., "task_execution:id" for execution mode)
      queueMessage(contextKey, content, messageId);

      try {
        // Queue via backend API with the same ID
        await chatApi.queueAgentMessage(ctxType, ctxId, content, messageId);
      } catch (error) {
        logError("queue message to backend", error);
        // Message is already in local store, which is fine - it just won't be processed by backend
        // User can delete and re-queue if needed
      }
    },
    [queueMessage, getQueueContext, generateQueuedMessageId, contextKey, logError]
  );

  // Delete queued message handler - syncs with backend
  // Both frontend and backend use the same ID, so we can delete directly by ID
  const handleDeleteQueuedMessage = useCallback(
    async (messageId: string) => {
      const { ctxType, ctxId } = getQueueContext();

      // Delete from local store immediately (optimistic) - unified queue with context-aware keys
      deleteQueuedMessage(contextKey, messageId);

      // Delete from backend using the same ID
      try {
        await chatApi.deleteQueuedAgentMessage(ctxType, ctxId, messageId);
      } catch (error) {
        logError("delete queued message from backend", error);
        // Message already removed from local store, which is fine
      }
    },
    [deleteQueuedMessage, getQueueContext, contextKey, logError]
  );

  // Edit queued message handler - delete old and queue new
  // Both frontend and backend use the same ID, so we can operate directly by ID
  const handleEditQueuedMessage = useCallback(
    async (messageId: string, newContent: string) => {
      const { ctxType, ctxId } = getQueueContext();

      // Delete old message from backend
      try {
        await chatApi.deleteQueuedAgentMessage(ctxType, ctxId, messageId);
      } catch (error) {
        logError("delete old queued message", error);
      }

      // Delete from local store - unified queue with context-aware keys
      deleteQueuedMessage(contextKey, messageId);

      // Generate new ID and queue the edited content
      const newMessageId = generateQueuedMessageId();

      // Add to local store first (optimistic) - unified queue with context-aware keys
      queueMessage(contextKey, newContent, newMessageId);

      // Queue to backend with same ID
      try {
        await chatApi.queueAgentMessage(ctxType, ctxId, newContent, newMessageId);
      } catch (error) {
        logError("queue edited message to backend", error);
        // Message is already in local store
      }
    },
    [deleteQueuedMessage, queueMessage, getQueueContext, generateQueuedMessageId, contextKey, logError]
  );

  // Edit last queued message - now using unified queue with context-aware keys
  const handleEditLastQueued = useCallback(() => {
    const lastMessage = queuedMessages[queuedMessages.length - 1];
    if (!lastMessage) return;
    startEditingQueuedMessage(contextKey, lastMessage.id);
  }, [queuedMessages, startEditingQueuedMessage, contextKey]);

  // Subscribe to Tauri events for real-time updates (only on mount)
  // Using unified agent:* events (Phase 5-6 consolidation)
  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    (async () => {
      // Listen for tool calls - accumulate for streaming display and invalidate cache
      // Unified event: agent:tool_call (replaces chat:tool_call and execution:tool_call)
      const toolCallUnlisten = await listen<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        tool_name: string;
        tool_id?: string;
        arguments: unknown;
        result: unknown;
      }>("agent:tool_call", (event) => {
        const { tool_name, tool_id, arguments: args, result, conversation_id } = event.payload;
        // Only show for active conversation
        if (conversation_id === activeConversationIdRef.current) {
          // Use backend tool_id for deduplication, fall back to timestamp-based ID if null
          const id = tool_id ?? `streaming-${Date.now()}`;

          setStreamingToolCalls((prev) => {
            // Check if tool call already exists (deduplicate by tool_id)
            const existing = prev.find((tc) => tc.id === id);

            if (existing) {
              // Update existing entry with new data (started → completed → result lifecycle)
              return prev.map((tc) =>
                tc.id === id
                  ? {
                      ...tc,
                      name: tool_name,
                      arguments: args ?? tc.arguments,
                      result: result ?? tc.result,
                    }
                  : tc
              );
            }

            // New tool call - append
            return [
              ...prev,
              {
                id,
                name: tool_name,
                arguments: args,
                result,
              },
            ];
          });
          // Invalidate cache to pick up any new messages from backend
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
      });
      unlisteners.push(toolCallUnlisten);

      // Listen for run completion - clear streaming state and refresh
      // Unified event: agent:run_completed (replaces chat:run_completed and execution:run_completed)
      const runCompletedUnlisten = await listen<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        status: string;
      }>("agent:run_completed", (event) => {
        const { conversation_id } = event.payload;
        // Clear streaming tool calls
        setStreamingToolCalls([]);
        // Invalidate cache to get final messages
        if (conversation_id) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
        // Force scroll to bottom after completion
        setTimeout(() => {
          if (messagesEndRef.current) {
            messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
          }
        }, 100);
      });
      unlisteners.push(runCompletedUnlisten);

      // Listen for agent errors - clear streaming state
      // Unified event: agent:error
      const errorUnlisten = await listen<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        error: string;
      }>("agent:error", (event) => {
        const { conversation_id, error, context_type } = event.payload;
        logError(`agent error (context=${context_type}, conversation=${conversation_id})`, error);
        // Clear streaming tool calls on error
        setStreamingToolCalls([]);
        // Invalidate cache
        if (conversation_id) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
      });
      unlisteners.push(errorUnlisten);

      // Listen for run started - for progress tracking
      // Unified event: agent:run_started
      const runStartedUnlisten = await listen<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        agent_run_id: string;
      }>("agent:run_started", (event) => {
        const { conversation_id } = event.payload;
        // Invalidate agent run status to pick up new run
        if (conversation_id) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.agentRun(conversation_id),
          });
        }
      });
      unlisteners.push(runStartedUnlisten);

      // Listen for queue_sent - backend notifies when it sends a queued message
      // This updates the optimistic UI for queued messages
      // Since frontend and backend use the same ID, we can match exactly by ID
      const queueSentUnlisten = await listen<{
        message_id: string;
        conversation_id: string;
        context_type: string;
        context_id: string;
      }>("agent:queue_sent", (event) => {
        const { message_id, context_type, context_id } = event.payload;

        // Build context key from event payload - unified queue with context-aware keys
        const eventContextKey = context_type === "ideation"
          ? `session:${context_id}`
          : context_type === "task"
            ? `task:${context_id}`
            : context_type === "task_execution"
              ? `task_execution:${context_id}`
              : context_type === "review"
                ? `review:${context_id}`
                : `project:${context_id}`;
        useChatStore.getState().deleteQueuedMessage(eventContextKey, message_id);
      });
      unlisteners.push(queueSentUnlisten);
    })();

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [queryClient, logError, messagesEndRef]);

  return {
    streamingToolCalls,
    handleSend,
    handleQueue,
    handleStopAgent,
    handleDeleteQueuedMessage,
    handleEditQueuedMessage,
    handleEditLastQueued,
  };
}
