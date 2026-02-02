/**
 * useChatPanelContext - Context management for IntegratedChatPanel
 *
 * Handles:
 * - Computing chat context based on selected task/ideation session
 * - Computing store context key for queue/agent state operations
 * - Context change effects (clearing old conversation, cache invalidation)
 * - Auto-selecting conversations in execution/review modes
 */

import { useState, useRef, useEffect, useMemo, useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useChatStore, selectActiveConversationId, getContextKey } from "@/stores/chatStore";
import { chatKeys } from "@/hooks/useChat";
import type { ChatContext } from "@/types/chat";
import type { ContextType } from "@/types/chat-conversation";
import type { ToolCall } from "@/components/Chat/ToolCallIndicator";

interface UseChatPanelContextProps {
  projectId: string;
  ideationSessionId: string | undefined;
  selectedTaskId: string | undefined;
  isExecutionMode: boolean;
  isReviewMode: boolean;
  isMergeMode: boolean;
  /** Override conversation ID for history mode - forces selection of specific conversation */
  overrideConversationId?: string | undefined;
  /** Override agent run ID for history mode - used for scroll positioning */
  overrideAgentRunId?: string | undefined;
}

interface ConversationData {
  id: string;
  lastMessageAt?: string | null;
  createdAt: string;
}

interface ConversationsQueryResult {
  data?: ConversationData[] | undefined;
  isLoading: boolean;
}

export function useChatPanelContext({
  projectId,
  ideationSessionId,
  selectedTaskId,
  isExecutionMode,
  isReviewMode,
  isMergeMode,
  overrideConversationId,
  overrideAgentRunId,
}: UseChatPanelContextProps) {
  const queryClient = useQueryClient();
  const activeConversationId = useChatStore(selectActiveConversationId);
  const setActiveConversation = useChatStore((s) => s.setActiveConversation);
  const clearMessages = useChatStore((s) => s.clearMessages);

  // Streaming tool calls - accumulated during agent execution
  const [streamingToolCalls, setStreamingToolCalls] = useState<ToolCall[]>([]);

  // Build chat context based on selected task or ideation session
  const chatContext: ChatContext = useMemo(() => {
    if (ideationSessionId) {
      return {
        view: "ideation",
        projectId,
        ideationSessionId,
      };
    }
    if (selectedTaskId) {
      return {
        view: "task_detail",
        projectId,
        selectedTaskId,
      };
    }
    return {
      view: "kanban",
      projectId,
    };
  }, [selectedTaskId, projectId, ideationSessionId]);

  // Compute store context key for queue/agent state operations
  // Uses context-aware keys: "task_execution:id", "review:id", "merge:id", or standard keys
  const storeContextKey = useMemo(() => {
    if (isMergeMode && selectedTaskId) {
      return `merge:${selectedTaskId}`;
    }
    if (isExecutionMode && selectedTaskId) {
      return `task_execution:${selectedTaskId}`;
    }
    if (isReviewMode && selectedTaskId) {
      return `review:${selectedTaskId}`;
    }
    return getContextKey(chatContext);
  }, [isMergeMode, isExecutionMode, isReviewMode, selectedTaskId, chatContext]);

  // Context key for tracking changes
  const contextKey = ideationSessionId
    ? `ideation:${ideationSessionId}`
    : selectedTaskId
      ? `${isMergeMode ? "merge" : isExecutionMode ? "execution" : isReviewMode ? "review" : "task"}:${selectedTaskId}`
      : `project:${projectId}`;

  // Initialize with empty string to ensure cleanup runs on first mount
  const prevContextKeyRef = useRef("");
  const prevContextTypeRef = useRef<{ type: string; id: string } | null>(null);

  // Auto-select tracking
  const hasAutoSelectedRef = useRef(false);

  // Track the previous context type and id for cache invalidation
  useEffect(() => {
    const currentContextType = ideationSessionId
      ? "ideation"
      : selectedTaskId
        ? (isMergeMode ? "merge" : isExecutionMode ? "task_execution" : isReviewMode ? "review" : "task")
        : "project";
    const currentContextId = ideationSessionId || selectedTaskId || projectId;
    prevContextTypeRef.current = { type: currentContextType, id: currentContextId };
  }, [selectedTaskId, isMergeMode, isExecutionMode, isReviewMode, projectId, ideationSessionId]);

  // Handle context changes
  useEffect(() => {
    if (prevContextKeyRef.current !== contextKey) {
      // Context changed - get the current conversation ID and context before clearing
      const currentConversationId = useChatStore.getState().activeConversationId;
      const oldContext = prevContextTypeRef.current;

      // Clear the active conversation immediately
      setActiveConversation(null);

      // Clear streaming tool calls
      setStreamingToolCalls([]);

      // Clear the query cache for the old conversation to prevent stale data
      if (currentConversationId) {
        queryClient.removeQueries({
          queryKey: chatKeys.conversation(currentConversationId),
        });
      }

      // Also clear the old context's conversation list to prevent initialization
      // from picking up stale conversations
      if (oldContext) {
        queryClient.removeQueries({
          queryKey: chatKeys.conversationList(oldContext.type as ContextType, oldContext.id),
        });
      }

      // Clear messages from Zustand store for the old context to free memory
      if (prevContextKeyRef.current) {
        clearMessages(prevContextKeyRef.current);
      }

      // Reset auto-select flag when context changes
      hasAutoSelectedRef.current = false;

      prevContextKeyRef.current = contextKey;
    }
  }, [contextKey, setActiveConversation, queryClient, clearMessages]);

  // Track previous override conversation ID to detect changes
  const prevOverrideConversationIdRef = useRef<string | undefined>(undefined);

  // Handle override conversation selection (for history mode)
  useEffect(() => {
    if (overrideConversationId !== prevOverrideConversationIdRef.current) {
      prevOverrideConversationIdRef.current = overrideConversationId;

      if (overrideConversationId) {
        // In history mode with a specific conversation - select it
        setActiveConversation(overrideConversationId);
        hasAutoSelectedRef.current = true; // Prevent auto-select from overriding
      }
    }
  }, [overrideConversationId, setActiveConversation]);

  // Determine current context type and ID for validation
  const currentContextType: ContextType = ideationSessionId
    ? "ideation"
    : selectedTaskId
      ? (isMergeMode ? "merge" : isExecutionMode ? "task_execution" : isReviewMode ? "review" : "task")
      : "project";
  const currentContextId = ideationSessionId || selectedTaskId || projectId;

  // Auto-select the most recent conversation for execution/review/merge modes
  const autoSelectConversation = useCallback((
    conversations: ConversationsQueryResult,
    executionLoading: boolean,
    reviewLoading: boolean,
    mergeLoading: boolean,
  ) => {
    const isLoading = isMergeMode
      ? mergeLoading
      : isExecutionMode
        ? executionLoading
        : isReviewMode
          ? reviewLoading
          : conversations.isLoading;

    // Wait for conversations to load before any validation/selection
    if (isLoading) {
      return;
    }

    // CRITICAL: Check for stale activeConversationId FIRST, before checking hasAutoSelectedRef.
    if (activeConversationId && conversations.data && conversations.data.length > 0) {
      const belongsToContext = conversations.data.some(c => c.id === activeConversationId);
      if (!belongsToContext) {
        hasAutoSelectedRef.current = false;
        setActiveConversation(null);
        return;
      }
    }

    // Reset the flag if we're in execution/review mode but have no active conversation
    if (!activeConversationId && hasAutoSelectedRef.current) {
      hasAutoSelectedRef.current = false;
    }

    // Only auto-select once per context change
    if (hasAutoSelectedRef.current) {
      return;
    }

    if (!activeConversationId && conversations.data && conversations.data.length > 0) {
      // Sort by most recent activity
      const sorted = [...conversations.data].sort((a, b) => {
        const aTime = a.lastMessageAt || a.createdAt;
        const bTime = b.lastMessageAt || b.createdAt;
        return new Date(bTime).getTime() - new Date(aTime).getTime();
      });
      const mostRecent = sorted[0];

      if (mostRecent) {
        hasAutoSelectedRef.current = true;
        setActiveConversation(mostRecent.id);
      }
    }
  }, [activeConversationId, isMergeMode, isExecutionMode, isReviewMode, ideationSessionId, selectedTaskId, contextKey, setActiveConversation]);

  return {
    chatContext,
    storeContextKey,
    contextKey,
    currentContextType,
    currentContextId,
    activeConversationId,
    streamingToolCalls,
    setStreamingToolCalls,
    autoSelectConversation,
    /** Override agent run ID for scroll positioning in history mode */
    overrideAgentRunId,
  };
}
