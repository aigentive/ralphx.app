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
import type { StreamingTask, StreamingContentBlock } from "@/types/streaming-task";
import type { Unsubscribe } from "@/lib/event-bus";

// ============================================================================
// Types
// ============================================================================

interface UseChatEventsProps {
  activeConversationId: string | null;
  contextId: string | null;
  contextType: ContextType | null;
  setStreamingToolCalls: Dispatch<SetStateAction<ToolCall[]>>;
  setStreamingContentBlocks: Dispatch<SetStateAction<StreamingContentBlock[]>>;
  setStreamingTasks: Dispatch<SetStateAction<Map<string, StreamingTask>>>;
  /** Ref to track conversation ID that's currently finalizing (between message_created and query refetch) */
  finalizingConversationRef: React.MutableRefObject<string | null>;
}

// ============================================================================
// Hook
// ============================================================================

export function useChatEvents({
  activeConversationId,
  contextId,
  contextType,
  setStreamingToolCalls,
  setStreamingContentBlocks,
  setStreamingTasks,
  finalizingConversationRef,
}: UseChatEventsProps) {
  const bus = useEventBus();
  const queryClient = useQueryClient();

  // Resolve feature flags from registry
  const config = contextType ? getContextConfig(contextType) : null;
  const supportsStreamingText = config?.supportsStreamingText ?? false;
  const supportsSubagentTasks = config?.supportsSubagentTasks ?? false;

  useEffect(() => {
    // Clear streaming state immediately when conversation changes to ensure clean slate
    // This runs BEFORE subscribing to new events, preventing stale state from previous conversation
    setStreamingToolCalls(prev => prev.length === 0 ? prev : []);
    setStreamingContentBlocks(prev => prev.length === 0 ? prev : []);
    setStreamingTasks(prev => prev.size === 0 ? prev : new Map());

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

        if (!isRelevant(payload)) return;

        // Handle result events: update existing tool calls with result payload
        if (tool_name.startsWith("result:toolu")) {
          // Extract tool_use_id from tool_name by stripping "result:" prefix
          const toolUseId = tool_name.slice(7); // "result:".length === 7

          // 1. Update matching entry in streamingToolCalls
          setStreamingToolCalls((prev) =>
            prev.map((tc) => {
              if (tc.id !== toolUseId) return tc;
              const updated: ToolCall = { ...tc };
              if (result != null) {
                updated.result = result;
              }
              return updated;
            })
          );

          // 2. Update matching entry in streamingContentBlocks
          setStreamingContentBlocks((prev) =>
            prev.map((block) => {
              if (block.type !== "tool_use" || block.toolCall.id !== toolUseId) return block;
              const updated: ToolCall = { ...block.toolCall };
              if (result != null) {
                updated.result = result;
              }
              return { type: "tool_use", toolCall: updated };
            })
          );

          // 3. Update matching entry in streamingTasks.childToolCalls
          setStreamingTasks((prev) => {
            let changed = false;
            const next = new Map(prev);
            for (const [taskId, task] of prev) {
              const childIdx = task.childToolCalls.findIndex((tc) => tc.id === toolUseId);
              if (childIdx >= 0) {
                const updatedCalls = [...task.childToolCalls];
                const existing = updatedCalls[childIdx]!;
                const updated: ToolCall = { ...existing };
                if (result != null) {
                  updated.result = result;
                }
                updatedCalls[childIdx] = updated;
                next.set(taskId, { ...task, childToolCalls: updatedCalls });
                changed = true;
              }
            }
            return changed ? next : prev;
          });

          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
          return;
        }

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

          // Push to streamingContentBlocks to preserve chronological position.
          // Task tool calls get a position-marker block { type: "task", toolUseId }
          // so they render inline at the correct position (not grouped after all text).
          // Actual task metadata is read from streamingTasks Map via toolUseId lookup.
          if (tool_name.toLowerCase() === "task") {
            setStreamingContentBlocks((prev) => {
              // Only add the marker once — deduplicate by toolUseId
              const alreadyHasMarker = prev.some((block) => block.type === "task" && block.toolUseId === id);
              if (alreadyHasMarker) return prev;
              return [...prev, { type: "task", toolUseId: id }];
            });
          } else {
            setStreamingContentBlocks((prev) => {
              const existing = prev.find((block) => block.type === "tool_use" && block.toolCall.id === id);
              if (existing) {
                // Update existing tool_use block
                return prev.map((block) => {
                  if (block.type !== "tool_use" || block.toolCall.id !== id) return block;
                  const updated: ToolCall = {
                    ...block.toolCall,
                    name: tool_name,
                    arguments: args ?? block.toolCall.arguments,
                    result: result ?? block.toolCall.result,
                  };
                  if (diffContext) {
                    updated.diffContext = diffContext;
                  }
                  return { type: "tool_use", toolCall: updated };
                });
              }
              // New tool_use block — append
              return [...prev, { type: "tool_use", toolCall: entry }];
            });
          }
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
    // Skip chunks with teammate_name — those route to teamStore via useTeamEvents
    if (supportsStreamingText) {
      unsubscribes.push(
        bus.subscribe<{ text: string; conversation_id: string; context_id?: string; context_type?: string; teammate_name?: string | null }>(
          "agent:chunk", (payload) => {
            if (payload.teammate_name) return;
            if (!isRelevant(payload)) return;
            setStreamingContentBlocks((prev) => {
              const lastBlock = prev[prev.length - 1];
              // If last block is text, append to it; otherwise create new text block
              if (lastBlock?.type === "text") {
                const updated = [...prev];
                updated[updated.length - 1] = { type: "text", text: lastBlock.text + payload.text };
                return updated;
              }
              return [...prev, { type: "text", text: payload.text }];
            });
          }
        )
      );
    }

    // ── agent:message_created ────────────────────────────────────────
    // Clear streaming state for assistant messages to prevent duplicate display.
    //
    // Atomic swap strategy: Mark conversation as "finalizing" in ref BEFORE clearing
    // streaming state. The ref persists through the query refetch window (500ms), allowing
    // ChatMessageList to continue filtering the last assistant message until the DB query
    // completes. This prevents duplicates during the timing window between clearing state
    // and query refetch completion.
    //
    // Timeline:
    // 1. Streaming active: streamingContentBlocks visible, last DB assistant message filtered
    // 2. agent:message_created fires: set finalizingConversationRef → clear streaming state → invalidate query
    // 3. Streaming state clears synchronously, but ref persists → filter still applies
    // 4. Query refetch completes: new DB message appears
    // 5. After 500ms: clear ref → filter no longer applies
    // Result: smooth swap, no duplicate or flash
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
          // Mark conversation as finalizing BEFORE clearing state
          // This ref persists through the query refetch window, keeping the filter active
          finalizingConversationRef.current = payload.conversation_id;

          setStreamingContentBlocks(prev => prev.length === 0 ? prev : []);
          setStreamingToolCalls(prev => prev.length === 0 ? prev : []);
          setStreamingTasks(prev => prev.size === 0 ? prev : new Map());

          // Clear the finalizing ref after query refetch completes (500ms delay)
          setTimeout(() => {
            if (finalizingConversationRef.current === payload.conversation_id) {
              finalizingConversationRef.current = null;
            }
          }, 500);
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

        setStreamingToolCalls(prev => prev.length === 0 ? prev : []);
        setStreamingContentBlocks(prev => prev.length === 0 ? prev : []);
        setStreamingTasks(prev => prev.size === 0 ? prev : new Map());
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

        setStreamingToolCalls(prev => prev.length === 0 ? prev : []);
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(payload.conversation_id),
        });
      })
    );

    // ── Cleanup ──────────────────────────────────────────────────────
    return () => {
      setStreamingToolCalls(prev => prev.length === 0 ? prev : []);
      setStreamingContentBlocks(prev => prev.length === 0 ? prev : []);
      setStreamingTasks(prev => prev.size === 0 ? prev : new Map());
      finalizingConversationRef.current = null;
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [
    bus, queryClient, activeConversationId, contextId, contextType,
    supportsStreamingText, supportsSubagentTasks,
    setStreamingToolCalls, setStreamingContentBlocks, setStreamingTasks,
    finalizingConversationRef,
  ]);
}
