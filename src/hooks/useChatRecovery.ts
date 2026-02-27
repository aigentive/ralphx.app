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

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { chatKeys } from "@/hooks/useChat";
import { taskKeys } from "@/hooks/useTasks";
import type { ContextType } from "@/types/chat-conversation";
import { MERGE_STATUSES } from "@/types/status";
import { chatApi } from "@/api/chat";

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
  /** Whether active conversation belongs to current context */
  isConversationInCurrentContext: boolean;
  /** Backend agent run status */
  agentRunStatus: string | undefined;
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
  isConversationInCurrentContext,
  agentRunStatus,
  setAgentRunning,
  selectedTaskId,
  ideationSessionId,
  projectId,
  effectiveStatus,
}: UseChatRecoveryProps) {
  const queryClient = useQueryClient();

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
  useEffect(() => {
    if (!activeConversationId) return undefined;
    if (!isAgentRunning && agentRunStatus !== "running") return undefined;

    const intervalId = setInterval(() => {
      queryClient.invalidateQueries({
        queryKey: chatKeys.conversation(activeConversationId),
      });
    }, 2000);

    return () => clearInterval(intervalId);
  }, [activeConversationId, isAgentRunning, agentRunStatus, queryClient]);

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
    if (!effectiveStatus || !(MERGE_STATUSES as readonly string[]).includes(effectiveStatus)) return undefined;

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
