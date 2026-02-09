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
import type { StreamingTask } from "@/types/streaming-task";
import type { Unsubscribe } from "@/lib/event-bus";

interface UseIntegratedChatEventsProps {
  activeConversationId: string | null;
  contextId: string | null;
  contextType: string | null;
  setStreamingToolCalls: Dispatch<SetStateAction<ToolCall[]>>;
  setStreamingText: Dispatch<SetStateAction<string>>;
  setStreamingTasks: Dispatch<SetStateAction<Map<string, StreamingTask>>>;
}

export function useIntegratedChatEvents({
  activeConversationId,
  contextId,
  contextType,
  setStreamingToolCalls,
  setStreamingText,
  setStreamingTasks,
}: UseIntegratedChatEventsProps) {
  const bus = useEventBus();
  const queryClient = useQueryClient();

  // Subscribe to Tauri events for real-time updates
  useEffect(() => {
    const unsubscribes: Unsubscribe[] = [];

    // Unified agent:tool_call event (for merge and all contexts)
    // Dedup pattern ported from useChatPanelHandlers (Phase 41)
    // Routes tool calls by parentToolUseId: child → task's childToolCalls, parent → streamingToolCalls
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
        parent_tool_use_id?: string | null;
      }>("agent:tool_call", (payload) => {
        const { tool_name, tool_id, arguments: args, result, conversation_id, diff_context, parent_tool_use_id } = payload;

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

          const entry: ToolCall = { id, name: tool_name, arguments: args };
          if (result != null) {
            entry.result = result;
          }
          if (diffContext) {
            entry.diffContext = diffContext;
          }

          // Route to parent task's childToolCalls if this is a subagent tool call
          if (parent_tool_use_id) {
            setStreamingTasks((prev) => {
              const task = prev.get(parent_tool_use_id);
              if (!task) return prev; // No matching task — ignore (task_started may not have arrived yet)
              const next = new Map(prev);
              const existingIdx = task.childToolCalls.findIndex((tc) => tc.id === id);
              if (existingIdx >= 0) {
                // Update existing (Started → Completed lifecycle)
                const updatedCalls = [...task.childToolCalls];
                const existing = updatedCalls[existingIdx]!;
                const updated: ToolCall = {
                  ...existing,
                  name: tool_name,
                  arguments: args ?? existing.arguments,
                };
                if (result != null) {
                  updated.result = result;
                } else if (existing.result != null) {
                  updated.result = existing.result;
                }
                if (diffContext) {
                  updated.diffContext = diffContext;
                }
                updatedCalls[existingIdx] = updated;
                next.set(parent_tool_use_id, { ...task, childToolCalls: updatedCalls });
              } else {
                // New child tool call — append
                next.set(parent_tool_use_id, {
                  ...task,
                  childToolCalls: [...task.childToolCalls, entry],
                });
              }
              return next;
            });
          } else {
            // Parent-level tool call — route to streamingToolCalls (existing behavior)
            setStreamingToolCalls((prev) => {
              const existing = prev.find((tc) => tc.id === id);
              if (existing) {
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
              return [...prev, entry];
            });
          }

          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
      })
    );

    // Agent task (subagent) started — create new StreamingTask entry
    unsubscribes.push(
      bus.subscribe<{
        tool_use_id: string;
        description?: string;
        subagent_type?: string;
        model?: string;
        conversation_id: string;
        context_id?: string;
        context_type?: string;
      }>("agent:task_started", (payload) => {
        if (payload.conversation_id === activeConversationId && (!contextId || payload.context_id === contextId)) {
          setStreamingTasks((prev) => {
            const next = new Map(prev);
            next.set(payload.tool_use_id, {
              toolUseId: payload.tool_use_id,
              description: payload.description ?? "",
              subagentType: payload.subagent_type ?? "unknown",
              model: payload.model ?? "unknown",
              status: "running",
              startedAt: Date.now(),
              childToolCalls: [],
            });
            return next;
          });
        }
      })
    );

    // Agent task (subagent) completed — update task with stats
    unsubscribes.push(
      bus.subscribe<{
        tool_use_id: string;
        agent_id?: string;
        total_duration_ms?: number;
        total_tokens?: number;
        total_tool_use_count?: number;
        conversation_id: string;
        context_id?: string;
        context_type?: string;
      }>("agent:task_completed", (payload) => {
        if (payload.conversation_id === activeConversationId && (!contextId || payload.context_id === contextId)) {
          setStreamingTasks((prev) => {
            const task = prev.get(payload.tool_use_id);
            if (!task) return prev;
            const next = new Map(prev);
            const updated: StreamingTask = {
              ...task,
              status: "completed",
              completedAt: Date.now(),
            };
            if (payload.agent_id != null) {
              updated.agentId = payload.agent_id;
            }
            if (payload.total_duration_ms != null) {
              updated.totalDurationMs = payload.total_duration_ms;
            }
            if (payload.total_tokens != null) {
              updated.totalTokens = payload.total_tokens;
            }
            if (payload.total_tool_use_count != null) {
              updated.totalToolUseCount = payload.total_tool_use_count;
            }
            next.set(payload.tool_use_id, updated);
            return next;
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
        setStreamingTasks(new Map());
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(conversation_id),
        });
      })
    );

    return () => {
      setStreamingToolCalls([]); // Clear on cleanup to prevent context bleeding
      setStreamingText("");
      setStreamingTasks(new Map());
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, queryClient, setStreamingToolCalls, setStreamingText, setStreamingTasks, activeConversationId, contextId, contextType]);
}
