/**
 * useIntegratedChatEvents - Event subscription logic for IntegratedChatPanel
 *
 * Handles:
 * - Unified agent:* event subscriptions (tool calls, chunks, completions)
 * - Streaming tool call accumulation
 * - Cache invalidation on message creation
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
  setStreamingText: Dispatch<SetStateAction<string>>;
}

export function useIntegratedChatEvents({
  activeConversationId,
  messagesEndRef,
  setStreamingToolCalls,
  setStreamingText,
}: UseIntegratedChatEventsProps) {
  const bus = useEventBus();
  const queryClient = useQueryClient();
  const activeConversationIdRef = useRef(activeConversationId);

  // Subscribe to Tauri events for real-time updates
  useEffect(() => {
    // Update ref synchronously at the START of this effect, before creating subscriptions.
    // This prevents a race where autoSelectConversation changes activeConversationId,
    // events arrive with the new conversation_id, but the ref still has the old value.
    activeConversationIdRef.current = activeConversationId;

    const unsubscribes: Unsubscribe[] = [];

    // Unified agent:tool_call event (for merge and all contexts)
    unsubscribes.push(
      bus.subscribe<{
        tool_name: string;
        arguments: unknown;
        result?: unknown;
        conversation_id: string;
        diff_context?: { old_content?: string; file_path: string } | null;
      }>("agent:tool_call", (payload) => {
        const { tool_name, arguments: args, result, conversation_id, diff_context } = payload;
        if (conversation_id === activeConversationIdRef.current) {
          // Build diffContext with exactOptionalPropertyTypes compliance
          let diffContext: ToolCall["diffContext"];
          if (diff_context) {
            diffContext = { filePath: diff_context.file_path };
            if (diff_context.old_content != null) {
              diffContext.oldContent = diff_context.old_content;
            }
          }
          setStreamingToolCalls((prev) => {
            const entry: ToolCall = {
              id: `streaming-agent-${Date.now()}-${prev.length}`,
              name: tool_name,
              arguments: args,
              result,
            };
            if (diffContext) {
              entry.diffContext = diffContext;
            }
            return [...prev, entry];
          });
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
      })
    );

    // Streaming text chunks - accumulate for real-time display
    unsubscribes.push(
      bus.subscribe<{ text: string; conversation_id: string }>(
        "agent:chunk", (payload) => {
          if (payload.conversation_id === activeConversationIdRef.current) {
            setStreamingText((prev) => prev + payload.text);
          }
        }
      )
    );

    // Message created events - invalidate conversation for live updates
    const handleMessageCreated = (payload: { conversation_id?: string }) => {
      const conversationId = payload.conversation_id;
      if (!conversationId) {
        return;
      }
      if (conversationId === activeConversationIdRef.current) {
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(conversationId),
        });
      }
    };

    unsubscribes.push(
      bus.subscribe<{ conversation_id?: string }>(
        "agent:message_created",
        handleMessageCreated
      )
    );

    // Unified agent:run_completed event
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
      }>("agent:run_completed", (payload) => {
        const { conversation_id } = payload;
        setStreamingToolCalls([]);
        setStreamingText("");
        if (conversation_id) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
        setTimeout(() => {
          if (messagesEndRef.current) {
            messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
          }
        }, 100);
      })
    );

    return () => {
      setStreamingToolCalls([]); // Clear on cleanup to prevent context bleeding
      setStreamingText("");
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, queryClient, messagesEndRef, setStreamingToolCalls, setStreamingText, activeConversationId]);
}
