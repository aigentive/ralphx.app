/**
 * useIntegratedChatEvents - Event subscription logic for IntegratedChatPanel
 *
 * Handles:
 * - Tauri event listeners for tool calls
 * - Chat run completion events
 * - Execution-specific events
 * - Streaming tool call accumulation
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useEffect, useRef, type Dispatch, type SetStateAction } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useEventBus } from "@/providers/EventProvider";
import { chatKeys } from "@/hooks/useChat";
import type { ToolCall } from "@/components/Chat/ToolCallIndicator";
import type { Unsubscribe } from "@/lib/event-bus";

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
  const bus = useEventBus();
  const queryClient = useQueryClient();
  const activeConversationIdRef = useRef(activeConversationId);

  useEffect(() => {
    activeConversationIdRef.current = activeConversationId;
  }, [activeConversationId]);

  // Subscribe to Tauri events for real-time updates
  useEffect(() => {
    const unsubscribes: Unsubscribe[] = [];

    // Listen for tool calls - accumulate for streaming display and invalidate cache
    unsubscribes.push(
      bus.subscribe<{
        tool_name: string;
        arguments: unknown;
        result: unknown;
        conversation_id: string;
      }>("chat:tool_call", (payload) => {
        const { tool_name, arguments: args, result, conversation_id } = payload;
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
      })
    );

    // Listen for chat run completion - clear streaming state and refresh
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
      }>("chat:run_completed", (payload) => {
        const { conversation_id } = payload;
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
      })
    );

    // Execution-specific events
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
        tool_name: string;
        arguments: unknown;
      }>("execution:tool_call", (payload) => {
        const { tool_name, arguments: args, conversation_id } = payload;
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
      })
    );

    // Listen for execution completion - clear streaming state and refresh
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
      }>("execution:run_completed", (payload) => {
        const { conversation_id } = payload;
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
      })
    );

    return () => {
      setStreamingToolCalls([]); // Clear on cleanup to prevent context bleeding
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, queryClient, messagesEndRef, setStreamingToolCalls, activeConversationId]);
}
