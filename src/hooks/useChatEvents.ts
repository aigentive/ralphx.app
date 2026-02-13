/**
 * useChatEvents — Unified event subscription for all chat panels
 *
 * Merges:
 * - useIntegratedChatEvents (streaming text, subagent routing, diff views)
 * - Event handling from useChatPanelHandlers (tool calls, run lifecycle, queue)
 *
 * Uses registry feature flags to conditionally enable subscriptions:
 * - supportsStreamingText → agent:chunk
 * - supportsSubagentTasks → agent:task_started/completed, parent_tool_use_id routing
 * - supportsDiffViews → diff_context on tool calls
 *
 * The hook subscribes to events that supplement useAgentEvents (which handles
 * the core lifecycle: run_started, message_created, run_completed, queue_sent,
 * stopped, error, session_recovered). This hook adds streaming UI features.
 */

import { useEffect, type Dispatch, type SetStateAction } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useEventBus } from "@/providers/EventProvider";
import { chatKeys } from "@/hooks/useChat";
import { getContextConfig } from "@/lib/chat-context-registry";
import type { ContextType } from "@/types/chat-conversation";
import type { ToolCall } from "@/components/Chat/ToolCallIndicator";
import type { StreamingTask } from "@/types/streaming-task";
import type { Unsubscribe } from "@/lib/event-bus";

// ============================================================================
// Types
// ============================================================================

interface UseChatEventsProps {
  activeConversationId: string | null;
  contextId: string | null;
  contextType: ContextType | null;
  setStreamingToolCalls: Dispatch<SetStateAction<ToolCall[]>>;
  setStreamingText: Dispatch<SetStateAction<string>>;
  setStreamingTasks: Dispatch<SetStateAction<Map<string, StreamingTask>>>;
}

// ============================================================================
// Hook
// ============================================================================

export function useChatEvents({
  activeConversationId,
  contextId,
  contextType,
  setStreamingToolCalls,
  setStreamingText,
  setStreamingTasks,
}: UseChatEventsProps) {
  const bus = useEventBus();
  const queryClient = useQueryClient();

  // Resolve feature flags from registry
  const config = contextType ? getContextConfig(contextType) : null;
  const supportsStreamingText = config?.supportsStreamingText ?? false;
  const supportsSubagentTasks = config?.supportsSubagentTasks ?? false;

  useEffect(() => {
    const unsubscribes: Unsubscribe[] = [];

    // Helper: check if event matches current context
    const isRelevant = (payload: { conversation_id?: string; context_id?: string }) =>
      payload.conversation_id === activeConversationId &&
      (!contextId || payload.context_id === contextId);

    // ── agent:tool_call ──────────────────────────────────────────────
    // Handles tool call accumulation for streaming display.
    // Routes child tool calls to parent task when supportsSubagentTasks is enabled.
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

        // Skip result events — they don't add new tool calls
        if (tool_name.startsWith("result:toolu")) return;

        if (!isRelevant(payload)) return;

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
        if (supportsSubagentTasks && parent_tool_use_id) {
          setStreamingTasks((prev) => {
            const task = prev.get(parent_tool_use_id);
            if (!task) return prev;
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
          // Parent-level tool call — route to streamingToolCalls
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
      })
    );

    // ── agent:task_started (subagent) ────────────────────────────────
    if (supportsSubagentTasks) {
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
          if (!isRelevant(payload)) return;
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
        })
      );
    }

    // ── agent:task_completed (subagent) ──────────────────────────────
    if (supportsSubagentTasks) {
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
          if (!isRelevant(payload)) return;
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
        })
      );
    }

    // ── agent:chunk (streaming text) ─────────────────────────────────
    if (supportsStreamingText) {
      unsubscribes.push(
        bus.subscribe<{ text: string; conversation_id: string; context_id?: string; context_type?: string }>(
          "agent:chunk", (payload) => {
            if (!isRelevant(payload)) return;
            setStreamingText((prev) => prev + payload.text);
          }
        )
      );
    }

    // ── agent:message_created ────────────────────────────────────────
    // Clear streaming state for assistant messages to prevent duplicate display
    unsubscribes.push(
      bus.subscribe<{
        conversation_id?: string;
        context_id?: string;
        context_type?: string;
        role?: string;
      }>("agent:message_created", (payload) => {
        if (!payload.conversation_id) return;
        if (!isRelevant(payload)) return;

        if (payload.role === "assistant") {
          setStreamingText("");
          setStreamingToolCalls([]);
          setStreamingTasks(new Map());
        }

        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(payload.conversation_id),
        });
      })
    );

    // ── agent:run_completed ──────────────────────────────────────────
    // Clear all streaming state on run completion
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
        context_id?: string;
        context_type?: string;
      }>("agent:run_completed", (payload) => {
        if (!isRelevant(payload)) return;

        setStreamingToolCalls([]);
        setStreamingText("");
        setStreamingTasks(new Map());
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(payload.conversation_id),
        });
      })
    );

    // ── agent:error ──────────────────────────────────────────────────
    // Clear streaming state on error
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
        context_id?: string;
        context_type?: string;
        error: string;
      }>("agent:error", (payload) => {
        if (!payload.conversation_id) return;

        setStreamingToolCalls([]);
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(payload.conversation_id),
        });
      })
    );

    // ── Cleanup ──────────────────────────────────────────────────────
    return () => {
      setStreamingToolCalls([]);
      setStreamingText("");
      setStreamingTasks(new Map());
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [
    bus, queryClient, activeConversationId, contextId, contextType,
    supportsStreamingText, supportsSubagentTasks,
    setStreamingToolCalls, setStreamingText, setStreamingTasks,
  ]);
}
