/**
 * IntegratedChatPanel - Context-aware chat panel for split-screen layout
 *
 * This is a refactored version of ChatPanel that:
 * - Is part of the layout, not fixed positioned
 * - Has collapsed state (thin bar with expand button)
 * - Supports context switching based on selected task
 * - No slide animations (instant show/hide)
 *
 * Design spec: specs/design/refined-studio-patterns.md
 */

import { useState, useRef, useEffect, useMemo } from "react";
import { useChat, chatKeys } from "@/hooks/useChat";
import { useChatStore, selectQueuedMessages, selectIsAgentRunning, selectActiveConversationId, getContextKey } from "@/stores/chatStore";
import { useUiStore } from "@/stores/uiStore";
import { useTasks } from "@/hooks/useTasks";
import type { ChatContext } from "@/types/chat";
import type { ContextType } from "@/types/chat-conversation";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { chatApi } from "@/api/chat";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { PanelRightClose, Loader2 } from "lucide-react";
import { ConversationSelector } from "./ConversationSelector";
import { QueuedMessageList } from "./QueuedMessageList";
import { ChatInput } from "./ChatInput";
import { type ToolCall } from "./ToolCallIndicator";
import { StreamingToolIndicator } from "./StreamingToolIndicator";
import { MessageItem } from "./MessageItem";
import {
  TypingIndicator,
  EmptyState,
  LoadingState,
  WorkerExecutingIndicator,
  FailedRunBanner,
  ContextIndicator,
  CollapsedPanel,
  animationStyles,
} from "./IntegratedChatPanel.components";
import { useIntegratedChatScroll } from "@/hooks/useIntegratedChatScroll";
import { useIntegratedChatHandlers } from "@/hooks/useIntegratedChatHandlers";
import { useIntegratedChatEvents } from "@/hooks/useIntegratedChatEvents";

// ============================================================================
// Main Component
// ============================================================================

interface IntegratedChatPanelProps {
  /** Project ID for context */
  projectId: string;
  /** Optional ideation session ID - when set, uses ideation context */
  ideationSessionId?: string;
  /** Custom empty state component */
  emptyState?: React.ReactNode;
  /** Always show helper text under input */
  showHelperTextAlways?: boolean;
  /** Custom class for input container */
  inputContainerClassName?: string;
  /** Whether to show the collapse button (default: true) */
  showCollapseButton?: boolean;
  /** Custom header content to replace default context indicator */
  headerContent?: React.ReactNode;
}

export function IntegratedChatPanel({
  projectId,
  ideationSessionId,
  emptyState,
  showHelperTextAlways = false,
  inputContainerClassName,
  showCollapseButton = true,
  headerContent,
}: IntegratedChatPanelProps) {
  const queryClient = useQueryClient();
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const chatCollapsed = useUiStore((s) => s.chatCollapsed);
  const setChatCollapsed = useUiStore((s) => s.setChatCollapsed);

  const activeConversationId = useChatStore(selectActiveConversationId);

  // Get task data from React Query (useTasks) which has full task data
  // Note: We can't use useTaskStore because it only has partial task data from events
  const { data: tasks = [] } = useTasks(projectId);
  const selectedTask = useMemo(
    () => selectedTaskId ? tasks.find((t) => t.id === selectedTaskId) : undefined,
    [tasks, selectedTaskId]
  );

  // Execution states: worker agent is running
  const executionStatuses = ["executing", "re_executing", "qa_refining", "qa_testing", "qa_passed", "qa_failed"];
  const isExecutionMode = selectedTask?.internalStatus ? executionStatuses.includes(selectedTask.internalStatus) : false;

  // Review states: reviewer agent conversation
  const reviewStatuses = ["reviewing", "review_passed"];
  const isReviewMode = selectedTask?.internalStatus ? reviewStatuses.includes(selectedTask.internalStatus) : false;

  const setActiveConversation = useChatStore((s) => s.setActiveConversation);

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
  // Uses context-aware keys: "task_execution:id", "review:id", or standard keys
  const storeContextKey = useMemo(() => {
    if (isExecutionMode && selectedTaskId) {
      return `task_execution:${selectedTaskId}`;
    }
    if (isReviewMode && selectedTaskId) {
      return `review:${selectedTaskId}`;
    }
    return getContextKey(chatContext);
  }, [isExecutionMode, isReviewMode, selectedTaskId, chatContext]);

  // Use context-aware selectors - unified queue works for all modes
  const queuedMessagesSelector = useMemo(() => selectQueuedMessages(storeContextKey), [storeContextKey]);
  const queuedMessages = useChatStore(queuedMessagesSelector);
  const isAgentRunningSelector = useMemo(() => selectIsAgentRunning(storeContextKey), [storeContextKey]);
  const isAgentRunning = useChatStore(isAgentRunningSelector);


  // Streaming tool calls - accumulated during agent execution (defined early for context change effect)
  const [streamingToolCalls, setStreamingToolCalls] = useState<ToolCall[]>([]);

  // Reset active conversation when context changes
  // This ensures we load the correct conversations for the new context
  const contextKey = ideationSessionId
    ? `ideation:${ideationSessionId}`
    : selectedTaskId
      ? `${isExecutionMode ? "execution" : isReviewMode ? "review" : "task"}:${selectedTaskId}`
      : `project:${projectId}`;
  // Initialize with empty string to ensure cleanup runs on first mount
  // This prevents showing conversations from a different context
  const prevContextKeyRef = useRef("");
  const prevContextTypeRef = useRef<{ type: string; id: string } | null>(null);

  // Auto-select the most recent conversation for this context
  // Use a ref to track initialization and prevent infinite loops
  const hasAutoSelectedRef = useRef(false);

  // Track the previous context type and id for cache invalidation
  useEffect(() => {
    const currentContextType = ideationSessionId
      ? "ideation"
      : selectedTaskId
        ? (isExecutionMode ? "task_execution" : isReviewMode ? "review" : "task")
        : "project";
    const currentContextId = ideationSessionId || selectedTaskId || projectId;
    prevContextTypeRef.current = { type: currentContextType, id: currentContextId };
  }, [selectedTaskId, isExecutionMode, isReviewMode, projectId, ideationSessionId]);

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

      // Reset auto-select flag when context changes
      hasAutoSelectedRef.current = false;

      prevContextKeyRef.current = contextKey;
    }
  }, [contextKey, setActiveConversation, queryClient]);

  // For execution/review mode, fetch conversations directly with specific context type
  const regularChatData = useChat(chatContext);

  // Fetch execution conversations when in execution mode
  const executionConversationsQuery = useQuery({
    queryKey: chatKeys.conversationList("task_execution", selectedTaskId ?? ""),
    queryFn: () => chatApi.listConversations("task_execution", selectedTaskId ?? ""),
    enabled: isExecutionMode && !!selectedTaskId,
  });

  // Fetch review conversations when in review mode
  const reviewConversationsQuery = useQuery({
    queryKey: chatKeys.conversationList("review", selectedTaskId ?? ""),
    queryFn: () => chatApi.listConversations("review", selectedTaskId ?? ""),
    enabled: isReviewMode && !!selectedTaskId,
  });

  // Use execution/review conversations when in those modes, otherwise regular conversations
  const conversations = isExecutionMode
    ? executionConversationsQuery
    : isReviewMode
      ? reviewConversationsQuery
      : regularChatData.conversations;

  // Auto-select the most recent conversation when:
  // 1. We're in execution or review mode (these modes always have a specific conversation)
  // 2. No conversation is currently selected
  // 3. Conversations are loaded
  useEffect(() => {
    const isLoading = isExecutionMode
      ? executionConversationsQuery.isLoading
      : isReviewMode
        ? reviewConversationsQuery.isLoading
        : false;

    console.log(`[IntegratedChatPanel] Auto-select effect: isExec=${isExecutionMode}, isReview=${isReviewMode}, activeId=${activeConversationId}, isLoading=${isLoading}, convCount=${conversations.data?.length ?? 0}, hasAutoSelected=${hasAutoSelectedRef.current}, contextKey=${contextKey}`);

    // Only auto-select for execution/review modes where we want to show existing conversations
    if (!isExecutionMode && !isReviewMode) {
      return;
    }

    // Wait for conversations to load before any validation/selection
    if (isLoading) {
      console.log(`[IntegratedChatPanel] Waiting for conversations to load...`);
      return;
    }

    // CRITICAL: Check for stale activeConversationId FIRST, before checking hasAutoSelectedRef.
    // The activeConversationId in the store is global - it persists across context switches.
    // If it doesn't belong to the current context's conversations, it's stale and must be reset.
    if (activeConversationId && conversations.data && conversations.data.length > 0) {
      const belongsToContext = conversations.data.some(c => c.id === activeConversationId);
      if (!belongsToContext) {
        console.log(`[IntegratedChatPanel] Stale activeConversationId=${activeConversationId} not in context ${contextKey}, resetting`);
        // Reset both the ID and the flag so auto-select can run
        hasAutoSelectedRef.current = false;
        setActiveConversation(null);
        return; // Will re-run on next render with null activeConversationId
      }
    }

    // Reset the flag if we're in execution/review mode but have no active conversation
    // This handles the case where a task is closed and reopened - we need to re-select
    if (!activeConversationId && hasAutoSelectedRef.current) {
      console.log(`[IntegratedChatPanel] Resetting auto-select flag: no active conversation in ${isReviewMode ? 'review' : 'execution'} mode`);
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
        console.log(`[IntegratedChatPanel] Auto-selecting conversation: ${mostRecent.id} (${isReviewMode ? 'review' : 'execution'} mode)`);
        hasAutoSelectedRef.current = true;
        setActiveConversation(mostRecent.id);
      }
    } else if (!activeConversationId && conversations.data?.length === 0) {
      console.log(`[IntegratedChatPanel] No conversations available to auto-select`);
    }
  }, [activeConversationId, conversations.data, isExecutionMode, isReviewMode, setActiveConversation, executionConversationsQuery.isLoading, reviewConversationsQuery.isLoading, contextKey]);

  // Fetch agent run status for the active conversation
  const agentRunQuery = useQuery({
    queryKey: chatKeys.agentRun(activeConversationId ?? ""),
    queryFn: () => activeConversationId ? chatApi.getAgentRunStatus(activeConversationId) : null,
    enabled: !!activeConversationId,
    staleTime: 5000,
  });

  // Track dismissed error banners by run ID
  const [dismissedErrorId, setDismissedErrorId] = useState<string | null>(null);
  const failedRun = agentRunQuery.data?.status === "failed" ? agentRunQuery.data : null;
  const showFailedBanner = failedRun && failedRun.errorMessage && failedRun.id !== dismissedErrorId;

  const {
    messages: activeConversation,
    sendMessage,
    switchConversation: handleSelectConversation,
    createConversation: handleNewConversation,
  } = regularChatData;

  const [hasUnread, setHasUnread] = useState(false);
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const lastMessageCountRef = useRef(0);

  // Determine current context type and ID for validation
  const currentContextType: ContextType = ideationSessionId
    ? "ideation"
    : selectedTaskId
      ? (isExecutionMode ? "task_execution" : isReviewMode ? "review" : "task")
      : "project";
  const currentContextId = ideationSessionId || selectedTaskId || projectId;

  // Extract messages array from active conversation
  // Only show messages if:
  // 1. We have an active conversation ID
  // 2. The conversation belongs to the CURRENT context (not stale from previous context)
  // This prevents showing messages from a previous session when switching contexts
  const conversationContext = activeConversation.data?.conversation;
  const isConversationInCurrentContext =
    conversationContext?.contextType === currentContextType &&
    conversationContext?.contextId === currentContextId;

  // Memoize messagesData to avoid dependency chain issues in useEffect hooks
  const messagesData = useMemo(
    () =>
      activeConversationId && isConversationInCurrentContext
        ? (activeConversation.data?.messages ?? [])
        : [],
    [activeConversationId, isConversationInCurrentContext, activeConversation.data?.messages]
  );

  // Use custom hooks for extracted logic
  const { messagesEndRef } = useIntegratedChatScroll({
    messagesData,
    chatCollapsed,
    isAgentRunning,
    streamingToolCallsLength: streamingToolCalls.length,
  });

  const {
    handleSend,
    handleQueue,
    handleEditLastQueued,
    handleDeleteQueuedMessage,
    handleEditQueuedMessage,
    handleStopAgent,
  } = useIntegratedChatHandlers({
    isExecutionMode,
    isReviewMode,
    selectedTaskId: selectedTaskId ?? undefined,
    projectId,
    ideationSessionId,
    storeContextKey,
    sendMessage,
  });

  // Wrapper for handleEditLastQueued that provides the queued messages - unified queue
  const handleEditLastQueuedWrapper = () => {
    handleEditLastQueued(queuedMessages);
  };

  // Handle stopping agent - clear streaming tool calls
  const handleStopAgentWrapper = async () => {
    await handleStopAgent();
    setStreamingToolCalls([]);
  };

  useIntegratedChatEvents({
    activeConversationId,
    messagesEndRef,
    setStreamingToolCalls,
  });

  // Track unread messages when collapsed
  useEffect(() => {
    const messageCount = messagesData.length;
    if (chatCollapsed && messageCount > lastMessageCountRef.current) {
      setHasUnread(true);
    }
    lastMessageCountRef.current = messageCount;
  }, [messagesData.length, chatCollapsed]);

  // Clear unread when expanded
  useEffect(() => {
    if (!chatCollapsed) {
      setHasUnread(false);
    }
  }, [chatCollapsed]);

  // Sort messages by createdAt - render in chronological order, no grouping
  const sortedMessages = useMemo(() => {
    return [...messagesData].sort((a, b) =>
      new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime()
    );
  }, [messagesData]);

  if (chatCollapsed) {
    return (
      <CollapsedPanel
        onExpand={() => setChatCollapsed(false)}
        hasUnread={hasUnread}
      />
    );
  }

  const isLoading = activeConversation.isLoading;
  const isSending = sendMessage.isPending;
  const isEmpty = !isLoading && sortedMessages.length === 0;

  return (
    <>
      <style>{animationStyles}</style>
      <div
        data-testid="integrated-chat-panel"
        className="h-full flex flex-col border-l overflow-hidden"
        style={{
          backgroundColor: "var(--bg-surface)",
          borderColor: "var(--border-subtle)",
        }}
      >
        {/* Header - Glass effect */}
        <div
          data-testid="integrated-chat-header"
          className="flex items-center justify-between h-11 px-3 border-b backdrop-blur-sm shrink-0"
          style={{
            borderColor: "rgba(255,255,255,0.06)",
            background: "linear-gradient(180deg, rgba(26,26,26,0.95) 0%, rgba(20,20,20,0.98) 100%)",
          }}
        >
          {headerContent ?? <ContextIndicator context={chatContext} isExecutionMode={isExecutionMode} isReviewMode={isReviewMode} />}

          {/* Active agent badge */}
          {(isSending || isAgentRunning || isExecutionMode) && (
            <Badge variant="secondary" className="shrink-0 mr-2">
              <Loader2 className="w-3 h-3 mr-1 animate-spin" />
              {isExecutionMode ? "Worker running..." : isAgentRunning ? "Agent responding..." : "Working"}
            </Badge>
          )}

          <div className="flex items-center gap-1 shrink-0">
            {/* Conversation Selector */}
            <ConversationSelector
              contextType={
                ideationSessionId
                  ? "ideation"
                  : isExecutionMode
                    ? "task_execution"
                    : isReviewMode
                      ? "review"
                      : selectedTaskId
                        ? "task"
                        : "project"
              }
              contextId={ideationSessionId || selectedTaskId || projectId}
              conversations={conversations.data ?? []}
              activeConversationId={activeConversationId}
              onSelectConversation={handleSelectConversation}
              onNewConversation={handleNewConversation}
              isLoading={conversations.isLoading}
            />
            {showCollapseButton && (
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={() => setChatCollapsed(true)}
                aria-label="Collapse chat panel"
                className="hover:bg-white/5"
              >
                <PanelRightClose className="w-[18px] h-[18px]" />
              </Button>
            )}
          </div>
        </div>

        {/* Messages Area */}
        {isLoading ? (
          <div className="flex-1 flex items-center justify-center" data-testid="integrated-chat-messages">
            <LoadingState />
          </div>
        ) : isEmpty ? (
          <div className="flex-1 flex items-center justify-center" data-testid="integrated-chat-messages">
            {emptyState ?? <EmptyState />}
          </div>
        ) : (
          <ScrollArea
            ref={scrollAreaRef}
            className="flex-1"
            data-testid="integrated-chat-messages"
          >
            <div
              className="p-3 w-full"
              style={{ maxWidth: "100%", overflowWrap: "break-word", wordBreak: "break-word" }}
            >
              {/* Show failed run banner if last run failed */}
              {showFailedBanner && failedRun?.errorMessage && (
                <FailedRunBanner
                  errorMessage={failedRun.errorMessage}
                  onDismiss={() => setDismissedErrorId(failedRun.id)}
                />
              )}

              {/* Show worker executing indicator when in execution mode */}
              {isExecutionMode && <WorkerExecutingIndicator />}

              {sortedMessages.map((msg) => (
                <MessageItem
                  key={msg.id}
                  role={msg.role}
                  content={msg.content}
                  createdAt={msg.createdAt}
                  toolCalls={msg.toolCalls}
                  contentBlocks={msg.contentBlocks}
                />
              ))}
              {/* Show streaming tool calls or typing indicator while agent is working */}
              {(isSending || isAgentRunning) && (
                streamingToolCalls.length > 0 ? (
                  <StreamingToolIndicator toolCalls={streamingToolCalls} isActive={true} />
                ) : (
                  <TypingIndicator />
                )
              )}
              <div ref={messagesEndRef} />
            </div>
          </ScrollArea>
        )}

        {/* Input Area */}
        <div
          className={inputContainerClassName ?? "border-t shrink-0"}
          style={inputContainerClassName ? undefined : { borderColor: "var(--border-subtle)" }}
        >
          {/* Queued Messages - unified queue with context-aware keys */}
          {queuedMessages.length > 0 && (
            <div className="p-3 pb-0">
              <QueuedMessageList
                messages={queuedMessages}
                onEdit={handleEditQueuedMessage}
                onDelete={handleDeleteQueuedMessage}
              />
            </div>
          )}

          {/* Chat Input */}
          <div className="p-3">
            <ChatInput
              onSend={handleSend}
              onQueue={handleQueue}
              onStop={handleStopAgentWrapper}
              isAgentRunning={isExecutionMode || isAgentRunning}
              isSending={isSending}
              hasQueuedMessages={queuedMessages.length > 0}
              onEditLastQueued={handleEditLastQueuedWrapper}
              placeholder={
                ideationSessionId
                  ? "Send a message..."
                  : isExecutionMode
                    ? "Message worker... (will be sent when current response completes)"
                    : selectedTaskId
                      ? "Ask about this task..."
                      : "Send a message..."
              }
              showHelperText={showHelperTextAlways || queuedMessages.length > 0}
              autoFocus
            />
          </div>
        </div>
      </div>
    </>
  );
}
