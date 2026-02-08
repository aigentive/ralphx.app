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

import { useEffect, type Dispatch, type SetStateAction } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useEventBus } from "@/providers/EventProvider";
import { chatKeys } from "@/hooks/useChat";
import type { ToolCall } from "@/components/Chat/ToolCallIndicator";
import type { Unsubscribe } from "@/lib/event-bus";

interface UseIntegratedChatEventsProps {
  activeConversationId: string | null;
  contextId: string | null;
  contextType: string | null;
  messagesEndRef: React.RefObject<HTMLDivElement | null>;
  setStreamingToolCalls: Dispatch<SetStateAction<ToolCall[]>>;
  setStreamingText: Dispatch<SetStateAction<string>>;
}

export function useIntegratedChatEvents({
  activeConversationId,
  contextId,
  contextType,
  messagesEndRef,
  setStreamingToolCalls,
  setStreamingText,
}: UseIntegratedChatEventsProps) {
  const bus = useEventBus();
  const queryClient = useQueryClient();

  // Subscribe to Tauri events for real-time updates
  useEffect(() => {
    const unsubscribes: Unsubscribe[] = [];

    // Unified agent:tool_call event (for merge and all contexts)
    // Dedup pattern ported from useChatPanelHandlers (Phase 41)
    unsubscribes.push(
      bus.subscribe<{
        tool_name: string;
        tool_id?: string;
        arguments: unknown;
        result?: unknown;
        conversation_id: string;
        context_id?: string;
        context_type?: string;
        diff_context?: { old_content?: string; file_path: string } | null;
      }>("agent:tool_call", (payload) => {
        const { tool_name, tool_id, arguments: args, result, conversation_id, diff_context } = payload;

        // Skip result events early — they don't add new tool calls
        if (tool_name.startsWith("result:toolu")) return;

        if (conversation_id === activeConversationId && (!contextId || payload.context_id === contextId)) {
          // Build diffContext with exactOptionalPropertyTypes compliance
          let diffContext: ToolCall["diffContext"];
          if (diff_context) {
            diffContext = { filePath: diff_context.file_path };
            if (diff_context.old_content != null) {
              diffContext.oldContent = diff_context.old_content;
            }
          }

          // Use backend tool_id for deduplication, fall back to timestamp-based ID
          const id = tool_id ?? `streaming-agent-${Date.now()}`;

          setStreamingToolCalls((prev) => {
            const existing = prev.find((tc) => tc.id === id);
            if (existing) {
              // Update existing entry (Started → Completed lifecycle)
              return prev.map((tc) => {
                if (tc.id !== id) return tc;
                const updated: ToolCall = {
                  ...tc,
                  name: tool_name,
                  arguments: args ?? tc.arguments,
                  result: result ?? tc.result,
                };
                if (diffContext) {
                  updated.diffContext = diffContext;
                }
                return updated;
              });
            }
            // New tool call — append
            const entry: ToolCall = { id, name: tool_name, arguments: args, result };
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
      bus.subscribe<{ text: string; conversation_id: string; context_id?: string; context_type?: string }>(
        "agent:chunk", (payload) => {
          if (payload.conversation_id === activeConversationId && (!contextId || payload.context_id === contextId)) {
            setStreamingText((prev) => prev + payload.text);
          }
        }
      )
    );

    // Message created events - invalidate conversation for live updates
    const handleMessageCreated = (payload: { conversation_id?: string; context_id?: string; context_type?: string }) => {
      const conversationId = payload.conversation_id;
      if (!conversationId) {
        return;
      }
      if (conversationId === activeConversationId && (!contextId || payload.context_id === contextId)) {
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(conversationId),
        });
      }
    };

    unsubscribes.push(
      bus.subscribe<{ conversation_id?: string; context_id?: string; context_type?: string }>(
        "agent:message_created",
        handleMessageCreated
      )
    );

    // Unified agent:run_completed event
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
        context_id?: string;
        context_type?: string;
      }>("agent:run_completed", (payload) => {
        const { conversation_id } = payload;
        if (conversation_id !== activeConversationId || (contextId && payload.context_id !== contextId)) return;

        setStreamingToolCalls([]);
        setStreamingText("");
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(conversation_id),
        });
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
  }, [bus, queryClient, messagesEndRef, setStreamingToolCalls, setStreamingText, activeConversationId, contextId, contextType]);
}
