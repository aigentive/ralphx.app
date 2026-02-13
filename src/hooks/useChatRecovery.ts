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

// ============================================================================
// Types
// ============================================================================

interface UseChatRecoveryProps {
  activeConversationId: string | null | undefined;
  storeContextKey: string;
  currentContextType: ContextType;
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

  // Recovery fallback: clear stuck "running" state when backend says run finished
  useEffect(() => {
    if (!activeConversationId || !isConversationInCurrentContext) return;
    // Wait for agentRunStatus to resolve before clearing — prevents
    // thrashing during mount when status is still undefined (loading).
    if (agentRunStatus === undefined) return;
    if (agentRunStatus !== "running") {
      setAgentRunning(storeContextKey, false);
    }
  }, [activeConversationId, agentRunStatus, isConversationInCurrentContext, setAgentRunning, storeContextKey]);

  // Recovery fallback: poll conversation while agent is running (backend status)
  useEffect(() => {
    if (!activeConversationId || agentRunStatus !== "running") return undefined;

    const intervalId = setInterval(() => {
      queryClient.invalidateQueries({
        queryKey: chatKeys.conversation(activeConversationId),
      });
    }, 2000);

    return () => clearInterval(intervalId);
  }, [activeConversationId, agentRunStatus, queryClient]);

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

  // Live updates: poll active conversation while agent is running (store state)
  useEffect(() => {
    if (!activeConversationId || !isAgentRunning) return undefined;

    const intervalId = setInterval(() => {
      queryClient.invalidateQueries({
        queryKey: chatKeys.conversation(activeConversationId),
      });
    }, 2000);

    return () => clearInterval(intervalId);
  }, [activeConversationId, isAgentRunning, queryClient]);

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
