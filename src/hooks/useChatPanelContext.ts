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
import { buildStoreKey } from "@/lib/chat-context-registry";
import { chatKeys } from "@/hooks/useChat";
import type { ChatContext } from "@/types/chat";
import type { ContextType } from "@/types/chat-conversation";
import type { ToolCall } from "@/components/Chat/ToolCallIndicator";
import type { StreamingTask, StreamingContentBlock } from "@/types/streaming-task";

interface UseChatPanelContextProps {
  projectId: string;
  ideationSessionId: string | undefined;
  selectedTaskId: string | undefined;
  isExecutionMode: boolean;
  isReviewMode: boolean;
  isMergeMode: boolean;
  isHistoryMode: boolean;
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
  isHistoryMode,
  overrideConversationId,
  overrideAgentRunId,
}: UseChatPanelContextProps) {
  const queryClient = useQueryClient();
  const activeConversationId = useChatStore(selectActiveConversationId);
  const setActiveConversation = useChatStore((s) => s.setActiveConversation);
  const clearMessages = useChatStore((s) => s.clearMessages);
  const setAgentRunning = useChatStore((s) => s.setAgentRunning);
  const setSending = useChatStore((s) => s.setSending);

  // Streaming tool calls - accumulated during agent execution
  const [streamingToolCalls, setStreamingToolCalls] = useState<ToolCall[]>([]);

  // Streaming content blocks - text and tool calls interleaved in order for real-time display
  const [streamingContentBlocks, setStreamingContentBlocks] = useState<StreamingContentBlock[]>([]);

  // Streaming tasks - subagent Task tool calls during agent execution
  const [streamingTasks, setStreamingTasks] = useState<Map<string, StreamingTask>>(new Map());

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
  // Uses context-aware keys via registry: "task_execution:id", "review:id", "merge:id", or standard keys
  const storeContextKey = useMemo(() => {
    if (isMergeMode && selectedTaskId) {
      return buildStoreKey("merge", selectedTaskId);
    }
    if (isExecutionMode && selectedTaskId) {
      return buildStoreKey("task_execution", selectedTaskId);
    }
    if (isReviewMode && selectedTaskId) {
      return buildStoreKey("review", selectedTaskId);
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

  // Handle context changes
  useEffect(() => {
    if (prevContextKeyRef.current !== contextKey) {
      // Context changed - capture OLD context from refs BEFORE updating them
      const currentConversationId = useChatStore.getState().activeConversationId;
      const oldContext = prevContextTypeRef.current;

      // DO NOT clear active conversation here - let autoSelectConversation handle
      // the transition atomically to avoid transient empty state

      // Clear streaming state (functional updaters to avoid new-ref re-renders when already empty)
      setStreamingToolCalls(prev => prev.length === 0 ? prev : []);
      setStreamingContentBlocks(prev => prev.length === 0 ? prev : []);
      setStreamingTasks(prev => prev.size === 0 ? prev : new Map());

      // Cancel and remove the old conversation's agent run query to prevent
      // stale cached data from triggering recovery effects in the new context
      if (currentConversationId) {
        queryClient.cancelQueries({
          queryKey: chatKeys.agentRun(currentConversationId),
        });
        queryClient.removeQueries({
          queryKey: chatKeys.agentRun(currentConversationId),
        });
      }

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

      // Clear agent running and sending state for the NEW context to prevent stale state
      // from the previous context leaking (e.g., spinner showing in idle session)
      setAgentRunning(storeContextKey, false);
      setSending(storeContextKey, false);

      // Reset auto-select flag when context changes
      hasAutoSelectedRef.current = false;

      // Update refs with NEW context AFTER cleanup
      prevContextKeyRef.current = contextKey;
      const newContextType = ideationSessionId
        ? "ideation"
        : selectedTaskId
          ? (isMergeMode ? "merge" : isExecutionMode ? "task_execution" : isReviewMode ? "review" : "task")
          : "project";
      const newContextId = ideationSessionId || selectedTaskId || projectId;
      prevContextTypeRef.current = { type: newContextType, id: newContextId };
    }
  }, [contextKey, storeContextKey, setActiveConversation, queryClient, clearMessages, setAgentRunning, setSending, ideationSessionId, selectedTaskId, projectId, isMergeMode, isExecutionMode, isReviewMode]);

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
  ) => {
    const isAgentContext = isMergeMode || isExecutionMode || isReviewMode;

    // Wait for conversations to load before any validation/selection
    if (conversations.isLoading) {
      return;
    }

    // In history mode with an explicit conversation override, skip auto-selection.
    // But if no override is provided (e.g., 'approved' transition has no conversation_id),
    // allow auto-selection to pick the most recent review conversation.
    if (isHistoryMode && overrideConversationId) {
      return;
    }

    // Read activeConversationId from store snapshot at call-time (not from closure)
    // to avoid including it in useCallback deps — which would cause self-invalidation
    // when autoSelectConversation calls setActiveConversation.
    const currentActiveId = useChatStore.getState().activeConversationId;

    // CRITICAL: Check for stale activeConversationId FIRST, before checking hasAutoSelectedRef.
    // If current conversation is stale but new context has conversations, directly select replacement.
    if (currentActiveId && conversations.data) {
      const belongsToContext = conversations.data.length > 0
        ? conversations.data.some(c => c.id === currentActiveId)
        : false;

      if (!belongsToContext) {
        // Current conversation is stale
        if (conversations.data.length > 0) {
          // New context has conversations - directly select most recent
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
        } else {
          // Empty list — conversation may be freshly created and list hasn't
          // refetched yet. Don't clear; isConversationInCurrentContext guard
          // prevents wrong-context messages, and auto-select will run when
          // the list populates.
          return;
        }
        return;
      }
    }

    if (isAgentContext && conversations.data && conversations.data.length > 0) {
      const sorted = [...conversations.data].sort((a, b) => {
        const aTime = a.lastMessageAt || a.createdAt;
        const bTime = b.lastMessageAt || b.createdAt;
        return new Date(bTime).getTime() - new Date(aTime).getTime();
      });
      const mostRecent = sorted[0];
      if (mostRecent && mostRecent.id !== currentActiveId) {
        hasAutoSelectedRef.current = true;
        setActiveConversation(mostRecent.id);
        return;
      }
    }

    // Reset the flag if we're in execution/review mode but have no active conversation
    if (!currentActiveId && hasAutoSelectedRef.current) {
      hasAutoSelectedRef.current = false;
    }

    // Only auto-select once per context change
    if (hasAutoSelectedRef.current) {
      return;
    }

    if (!currentActiveId && conversations.data && conversations.data.length > 0) {
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
  }, [isMergeMode, isExecutionMode, isReviewMode, isHistoryMode, overrideConversationId, setActiveConversation]);

  return {
    chatContext,
    storeContextKey,
    contextKey,
    currentContextType,
    currentContextId,
    activeConversationId,
    streamingToolCalls,
    setStreamingToolCalls,
    streamingContentBlocks,
    setStreamingContentBlocks,
    streamingTasks,
    setStreamingTasks,
    autoSelectConversation,
    /** Override agent run ID for scroll positioning in history mode */
    overrideAgentRunId,
  };
}
