/**
 * useChatRecovery — Recovery and polling effects for chat panels
 *
 * Extracted from IntegratedChatPanel to reduce component size.
 * Handles:
 * - Agent running state sync from backend
 * - Clearing stuck "running" state
 * - Polling conversation/list while agent is running
 * - Startup recovery window for agent contexts
 * - Merge watchdog polling
 */

import { useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { chatKeys } from "@/hooks/useChat";
import { taskKeys } from "@/hooks/useTasks";
import type { ContextType } from "@/types/chat-conversation";
import type { StreamingTask } from "@/types/streaming-task";
import { MERGE_STATUSES } from "@/types/status";
import { chatApi } from "@/api/chat";
import { mergeActiveStreamingTasks } from "./chat-active-state";

// ============================================================================
// Types
// ============================================================================

interface UseChatRecoveryProps {
  activeConversationId: string | null | undefined;
  storeContextKey: string;
  currentContextType: ContextType;
  /** Context ID used to key is_agent_running — bypasses activeConversationId mismatch */
  currentContextId: string;
  isHistoryMode: boolean;
  isAgentContext: boolean;
  isAgentRunning: boolean;
  /** Whether the agent is actively streaming chunks (generating state). Suppresses polling to avoid redundant refetches during streaming. */
  isGenerating: boolean;
  /** Whether active conversation belongs to current context */
  isConversationInCurrentContext: boolean;
  /** Backend agent run status */
  agentRunStatus: string | undefined;
  setStreamingTasks?: (
    updater: (prev: Map<string, StreamingTask>) => Map<string, StreamingTask>,
  ) => void;
  setAgentRunning: (contextKey: string, isRunning: boolean) => void;
  selectedTaskId: string | undefined;
  ideationSessionId: string | undefined;
  projectId: string;
  /** Effective status for merge watchdog */
  effectiveStatus: string | undefined;
}

// ============================================================================
// Hook
// ============================================================================

export function useChatRecovery({
  activeConversationId,
  storeContextKey,
  currentContextType,
  currentContextId,
  isHistoryMode,
  isAgentContext,
  isAgentRunning,
  isGenerating,
  isConversationInCurrentContext,
  agentRunStatus,
  setStreamingTasks,
  setAgentRunning,
  selectedTaskId,
  ideationSessionId,
  projectId,
  effectiveStatus,
}: UseChatRecoveryProps) {
  const queryClient = useQueryClient();
  const hydratedConversationIdRef = useRef<string | null>(null);

  useEffect(() => {
    if (!activeConversationId) {
      hydratedConversationIdRef.current = null;
      return;
    }
    if (isHistoryMode || !isAgentContext || !isConversationInCurrentContext || !setStreamingTasks) {
      return;
    }
    if (hydratedConversationIdRef.current === activeConversationId) {
      return;
    }

    hydratedConversationIdRef.current = activeConversationId;
    let cancelled = false;

    void chatApi
      .getConversationActiveState(activeConversationId)
      .then((activeState) => {
        if (cancelled || activeState.streaming_tasks.length === 0) return;
        setStreamingTasks((prev) => mergeActiveStreamingTasks(prev, activeState.streaming_tasks));
      })
      .catch(() => {
        // Best-effort recovery only. Live events remain authoritative.
      });

    return () => {
      cancelled = true;
    };
  }, [
    activeConversationId,
    isHistoryMode,
    isAgentContext,
    isConversationInCurrentContext,
    setStreamingTasks,
  ]);

  // Recovery fallback: if agent is running but events were missed, reflect it in UI
  useEffect(() => {
    if (agentRunStatus === "running" && isConversationInCurrentContext) {
      setAgentRunning(storeContextKey, true);
    }
  }, [agentRunStatus, isConversationInCurrentContext, setAgentRunning, storeContextKey]);

  // Recovery fallback: clear stuck "running" state when backend says run finished.
  // IMPORTANT: agentRunStatus reflects the DB *turn* status, not the *process*.
  // Between interactive turns, the DB shows "completed" for the finished turn
  // even though the process is still alive. Check process-level truth (IPR)
  // before clearing to avoid a race window where ChatInput takes the SEND path.
  useEffect(() => {
    if (!activeConversationId || !isConversationInCurrentContext) return;
    // Wait for agentRunStatus to resolve before clearing — prevents
    // thrashing during mount when status is still undefined (loading).
    if (agentRunStatus === undefined) return;
    if (agentRunStatus !== "running") {
      chatApi
        .isAgentRunning(currentContextType, currentContextId)
        .then((processRunning) => {
          if (!processRunning) {
            setAgentRunning(storeContextKey, false);
          }
        })
        .catch(() => {
          // If process check fails, fall back to DB truth
          setAgentRunning(storeContextKey, false);
        });
    }
  }, [activeConversationId, agentRunStatus, isConversationInCurrentContext, setAgentRunning, storeContextKey, currentContextType, currentContextId]);

  // Recovery fallback: keep conversation list fresh while agent is running
  useEffect(() => {
    if (isHistoryMode || !isAgentContext) return undefined;
    if (!isAgentRunning || !selectedTaskId) return undefined;

    const intervalId = setInterval(() => {
      queryClient.invalidateQueries({
        queryKey: chatKeys.conversationList(currentContextType, selectedTaskId),
      });
    }, 2000);

    return () => clearInterval(intervalId);
  }, [currentContextType, isAgentRunning, isAgentContext, isHistoryMode, queryClient, selectedTaskId]);

  // Live updates: poll active conversation while agent is running (store state or backend status).
  // Consolidates two previously-separate intervals that both polled the same query key.
  // Suppressed during active streaming — events (agent:chunk, agent:message_created) already
  // drive UI updates, so polling is redundant and would cause unnecessary refetches.
  useEffect(() => {
    if (!activeConversationId) return undefined;
    if (!isAgentRunning && agentRunStatus !== "running") return undefined;
    if (isGenerating) return undefined;

    const intervalId = setInterval(() => {
      queryClient.invalidateQueries({
        queryKey: chatKeys.conversation(activeConversationId),
      });
    }, 2000);

    return () => clearInterval(intervalId);
  }, [activeConversationId, isAgentRunning, isGenerating, agentRunStatus, queryClient]);

  // If a run is active but no conversation is selected, keep refreshing the list
  useEffect(() => {
    if (ideationSessionId || !selectedTaskId) return undefined;
    if (!isAgentRunning || activeConversationId) return undefined;
    if (!isAgentContext) return undefined;

    const intervalId = setInterval(() => {
      queryClient.invalidateQueries({
        queryKey: chatKeys.conversationList(currentContextType, selectedTaskId),
      });
    }, 2000);

    return () => clearInterval(intervalId);
  }, [activeConversationId, currentContextType, ideationSessionId, isAgentRunning, isAgentContext, queryClient, selectedTaskId]);

  // Reconciliation poll: 1.5s safety net while isAgentRunning is true.
  // Uses is_agent_running(contextType, contextId) rather than getAgentRunStatus(conversationId)
  // to bypass the activeConversationId mismatch that is the actual root cause of stuck state.
  // Zero overhead when no agent is running (interval is never created).
  useEffect(() => {
    if (!isAgentRunning) return undefined;

    const intervalId = setInterval(() => {
      chatApi
        .isAgentRunning(currentContextType, currentContextId)
        .then((running) => {
          if (!running) {
            setAgentRunning(storeContextKey, false);
          }
        })
        .catch(() => {
          // Silently ignore — primary signal is still Tauri events
        });
    }, 1500);

    return () => clearInterval(intervalId);
  }, [isAgentRunning, currentContextType, currentContextId, storeContextKey, setAgentRunning]);

  // Fast path: reconcile immediately when user re-focuses the app.
  // Covers the most common user-facing case: app was backgrounded/suspended during completion.
  useEffect(() => {
    if (!isAgentRunning) return undefined;

    function handleVisibilityChange() {
      if (document.visibilityState === "visible") {
        chatApi
          .isAgentRunning(currentContextType, currentContextId)
          .then((running) => {
            if (!running) {
              setAgentRunning(storeContextKey, false);
            }
          })
          .catch(() => {
            // Silently ignore
          });
      }
    }

    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () => document.removeEventListener("visibilitychange", handleVisibilityChange);
  }, [isAgentRunning, currentContextType, currentContextId, storeContextKey, setAgentRunning]);

  // Merge watchdog: keep polling task status while in merge flow
  useEffect(() => {
    if (ideationSessionId || !selectedTaskId) return undefined;
    // Poll during approved status too — bridges the gap before pending_merge
    if (!effectiveStatus || (effectiveStatus !== "approved" && !(MERGE_STATUSES as readonly string[]).includes(effectiveStatus))) return undefined;

    const intervalId = setInterval(() => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      queryClient.invalidateQueries({ queryKey: taskKeys.detail(selectedTaskId) });
    }, 2000);

    return () => clearInterval(intervalId);
  }, [effectiveStatus, ideationSessionId, projectId, queryClient, selectedTaskId]);

  // Recovery window: brief polling on startup for agent contexts
  useEffect(() => {
    if (ideationSessionId) return undefined;
    if (!selectedTaskId || !isAgentContext) return undefined;

    const intervalId = setInterval(() => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      if (selectedTaskId) {
        queryClient.invalidateQueries({ queryKey: taskKeys.detail(selectedTaskId) });
      }
      queryClient.invalidateQueries({
        queryKey: chatKeys.conversationList(currentContextType, selectedTaskId),
      });
      if (activeConversationId) {
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(activeConversationId),
        });
      }
    }, 2000);

    const timeoutId = setTimeout(() => {
      clearInterval(intervalId);
    }, 10000);

    return () => {
      clearInterval(intervalId);
      clearTimeout(timeoutId);
    };
  }, [activeConversationId, currentContextType, ideationSessionId, isAgentContext, projectId, queryClient, selectedTaskId]);
}
