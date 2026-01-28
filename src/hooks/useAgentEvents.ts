/**
 * useAgentEvents hook - Event listener management for agent lifecycle events
 *
 * Handles real-time updates for agent runs across all contexts (ideation, task, review, project).
 * Listens to unified agent:* events and updates query cache and store state accordingly.
 */

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ChatMessageResponse } from "@/api/chat";
import type { ChatConversation, ContextType } from "@/types/chat-conversation";
import { useChatStore } from "@/stores/chatStore";
import { chatKeys } from "./useChat";

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
 * Hook to manage agent event listeners
 *
 * Subscribes to Tauri events for real-time updates of agent runs.
 * Uses unified agent:* events (Phase 5-6 consolidation).
 *
 * @param activeConversationId - The currently active conversation ID to filter events
 */
export function useAgentEvents(activeConversationId: string | null) {
  const queryClient = useQueryClient();
  const { setAgentRunning, deleteQueuedMessage } = useChatStore();

  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    (async () => {
      // NOTE: Streaming cache updates disabled per user request.
      // Instead of trying to stream text/tool calls character-by-character,
      // we show a typing indicator while the agent is running and only
      // render the final message with proper content_blocks when the run completes.
      //
      // The agent:chunk and agent:tool_call events are still emitted by the backend
      // but we don't use them to update the UI during streaming. This avoids
      // issues with mismatched tool calls/results and partial content.

      // Listen for run started - set agent running state to true
      const runStartedUnlisten = await listen<{
        run_id: string;
        context_type: string;
        context_id: string;
        conversation_id: string;
      }>("agent:run_started", (event) => {
        const { context_type, context_id: eventContextId } = event.payload;

        // Build context key from the event payload
        const eventContextKey = buildContextKey(context_type as ContextType, eventContextId);

        // Set agent as running for this context
        setAgentRunning(eventContextKey, true);
      });
      unlisteners.push(runStartedUnlisten);

      // Listen for message created - optimistically add to cache
      // Unified event: agent:message_created (replaces chat:message_created)
      const messageCreatedUnlisten = await listen<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        message_id: string;
        role: string;
        content: string;
      }>("agent:message_created", (event) => {
        const { conversation_id, message_id, role, content } = event.payload;

        // Filter by context type if needed (all contexts use the same event now)
        // If this is for the active conversation, add message to cache
        if (conversation_id === activeConversationId) {
          queryClient.setQueryData<{ conversation: ChatConversation; messages: ChatMessageResponse[] }>(
            chatKeys.conversation(activeConversationId),
            (oldData) => {
              if (!oldData) return oldData;

              // Check if message already exists
              if (oldData.messages.some(m => m.id === message_id)) {
                return oldData;
              }

              const newMessage: ChatMessageResponse = {
                id: message_id,
                conversationId: conversation_id,
                sessionId: null,
                projectId: null,
                taskId: null,
                role: role as "user" | "assistant" | "system",
                content: content || "",
                metadata: null,
                parentMessageId: null,
                createdAt: new Date().toISOString(),
                toolCalls: null,
                contentBlocks: null,
              };
              return { ...oldData, messages: [...oldData.messages, newMessage] };
            }
          );
        }
      });
      unlisteners.push(messageCreatedUnlisten);

      // Listen for run completion
      // Unified event: agent:run_completed (replaces chat:run_completed)
      const runCompletedUnlisten = await listen<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        status: string;
      }>("agent:run_completed", async (event) => {
        const { conversation_id, context_type, context_id: eventContextId } = event.payload;

        // Build context key from the event payload
        const eventContextKey = buildContextKey(context_type as ContextType, eventContextId);

        // Update agent running state for the specific context
        setAgentRunning(eventContextKey, false);

        // Invalidate agent run status
        if (conversation_id === activeConversationId) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.agentRun(activeConversationId),
          });

          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(activeConversationId),
          });
        }

        // NOTE: Queue processing is now handled by the BACKEND
        // The backend automatically processes queued messages via --resume
        // when a run completes. We listen for agent:queue_sent to update UI.
      });
      unlisteners.push(runCompletedUnlisten);

      // Listen for queue_sent - backend notifies us when it sends a queued message
      // This allows us to update the optimistic UI by removing the sent message
      // Since frontend and backend use the same ID, we can match exactly by ID
      const queueSentUnlisten = await listen<{
        message_id: string;
        conversation_id: string;
        context_type: string;
        context_id: string;
      }>("agent:queue_sent", (event) => {
        const { message_id, context_type, context_id: eventContextId } = event.payload;

        // Execution mode uses a separate queue (executionQueuedMessages keyed by taskId)
        if (context_type === "task_execution") {
          // Use direct store access for execution queue
          useChatStore.getState().deleteExecutionQueuedMessage(eventContextId, message_id);
        } else {
          // Build context key from the event payload for regular chat queue
          const eventContextKey = buildContextKey(context_type as ContextType, eventContextId);
          // Remove from frontend optimistic queue by exact ID match
          deleteQueuedMessage(eventContextKey, message_id);
        }
      });
      unlisteners.push(queueSentUnlisten);

      // Listen for agent errors
      // Unified event: agent:error
      const errorUnlisten = await listen<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        error: string;
      }>("agent:error", (event) => {
        const { conversation_id, error, context_type, context_id: eventContextId } = event.payload;

        // Build context key from the event payload
        const eventContextKey = buildContextKey(context_type as ContextType, eventContextId);

        // Update agent running state on error for the specific context
        setAgentRunning(eventContextKey, false);

        // Invalidate queries to refresh state
        if (conversation_id === activeConversationId) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.agentRun(activeConversationId),
          });
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(activeConversationId),
          });
        }

        // Log error for debugging
        console.error(`[agent:error] context=${context_type}, conversation=${conversation_id}:`, error);
      });
      unlisteners.push(errorUnlisten);
    })();

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [activeConversationId, queryClient, setAgentRunning, deleteQueuedMessage]);
}
