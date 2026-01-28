/**
 * useIntegratedChatEvents - Event subscription logic for IntegratedChatPanel
 *
 * Handles:
 * - Tauri event listeners for tool calls
 * - Chat run completion events
 * - Execution-specific events
 * - Streaming tool call accumulation
 */

import { useEffect, useRef, type Dispatch, type SetStateAction } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import { chatKeys } from "@/hooks/useChat";
import type { ToolCall } from "@/components/Chat/ToolCallIndicator";

interface UseIntegratedChatEventsProps {
  activeConversationId: string | null;
  messagesEndRef: React.RefObject<HTMLDivElement | null>;
  setStreamingToolCalls: Dispatch<SetStateAction<ToolCall[]>>;
}

export function useIntegratedChatEvents({
  activeConversationId,
  messagesEndRef,
  setStreamingToolCalls,
}: UseIntegratedChatEventsProps) {
  const queryClient = useQueryClient();
  const activeConversationIdRef = useRef(activeConversationId);

  useEffect(() => {
    activeConversationIdRef.current = activeConversationId;
  }, [activeConversationId]);

  // Subscribe to Tauri events for real-time updates
  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    (async () => {
      // Listen for tool calls - accumulate for streaming display and invalidate cache
      const toolCallUnlisten = await listen<{
        tool_name: string;
        arguments: unknown;
        result: unknown;
        conversation_id: string;
      }>("chat:tool_call", (event) => {
        const { tool_name, arguments: args, result, conversation_id } = event.payload;
        // Only show for active conversation
        if (conversation_id === activeConversationIdRef.current) {
          setStreamingToolCalls((prev) => [
            ...prev,
            {
              id: `streaming-${Date.now()}-${prev.length}`,
              name: tool_name,
              arguments: args,
              result,
            },
          ]);
          // Invalidate cache to pick up any new messages from backend
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
      });
      unlisteners.push(toolCallUnlisten);

      // Listen for chat run completion - clear streaming state and refresh
      const runCompletedUnlisten = await listen<{
        conversation_id: string;
      }>("chat:run_completed", (event) => {
        const { conversation_id } = event.payload;
        // Clear streaming tool calls
        setStreamingToolCalls([]);
        // Invalidate cache to get final messages
        if (conversation_id) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
        // Scroll to bottom after a short delay to let messages render
        setTimeout(() => {
          if (messagesEndRef.current) {
            messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
          }
        }, 100);
      });
      unlisteners.push(runCompletedUnlisten);

      // Execution-specific events
      const execToolCallUnlisten = await listen<{
        conversation_id: string;
        tool_name: string;
        arguments: unknown;
      }>("execution:tool_call", (event) => {
        const { tool_name, arguments: args, conversation_id } = event.payload;
        // Only show for active conversation
        if (conversation_id === activeConversationIdRef.current) {
          setStreamingToolCalls((prev) => [
            ...prev,
            {
              id: `streaming-exec-${Date.now()}-${prev.length}`,
              name: tool_name,
              arguments: args,
            },
          ]);
          // Invalidate cache to pick up any new messages from backend
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
      });
      unlisteners.push(execToolCallUnlisten);

      // Listen for execution completion - clear streaming state and refresh
      const execCompletedUnlisten = await listen<{
        conversation_id: string;
      }>("execution:run_completed", (event) => {
        const { conversation_id } = event.payload;
        // Clear streaming tool calls
        setStreamingToolCalls([]);
        // Invalidate cache to get final messages
        if (conversation_id) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
        // Scroll to bottom after a short delay to let messages render
        setTimeout(() => {
          if (messagesEndRef.current) {
            messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
          }
        }, 100);
      });
      unlisteners.push(execCompletedUnlisten);
    })();

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [queryClient, messagesEndRef, setStreamingToolCalls]);
}
