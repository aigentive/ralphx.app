/**
 * IntegratedChatPanel - Context-aware chat panel for split-screen layout
 *
 * This is a refactored version of ChatPanel that:
 * - Is part of the layout, not fixed positioned
 * - Supports context switching based on selected task
 * - No slide animations (instant show/hide)
 *
 * Design spec: specs/design/refined-studio-patterns.md
 */

import { useState, useRef, useEffect, useMemo } from "react";
import { type VirtuosoHandle } from "react-virtuoso";
import { useChat, chatKeys } from "@/hooks/useChat";
import { useChatStore, selectQueuedMessages, selectIsAgentRunning } from "@/stores/chatStore";
import { useUiStore } from "@/stores/uiStore";
import { useTasks } from "@/hooks/useTasks";
import { useChatPanelContext } from "@/hooks/useChatPanelContext";
import { useQuery } from "@tanstack/react-query";
import { chatApi } from "@/api/chat";
import { EXECUTION_STATUSES, HUMAN_REVIEW_STATUSES } from "@/types/status";
import { StatusActivityBadge, type AgentType } from "./StatusActivityBadge";
import { ConversationSelector } from "./ConversationSelector";
import { QueuedMessageList } from "./QueuedMessageList";
import { ChatInput } from "./ChatInput";
import { ChatMessageList } from "./ChatMessageList";
import {
  EmptyState,
  LoadingState,
  ContextIndicator,
  animationStyles,
} from "./IntegratedChatPanel.components";
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
  /** Custom header content to replace default context indicator */
  headerContent?: React.ReactNode;
  /** Called when Escape is pressed with input blurred - used to close the panel */
  onClose?: () => void;
}

export function IntegratedChatPanel({
  projectId,
  ideationSessionId,
  emptyState,
  showHelperTextAlways = false,
  inputContainerClassName,
  headerContent,
  onClose,
}: IntegratedChatPanelProps) {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  // History state from store - shared with TaskDetailOverlay for time-travel feature
  const taskHistoryState = useUiStore((s) => s.taskHistoryState);

  // Get task data from React Query (useTasks) which has full task data
  const { data: tasks = [] } = useTasks(projectId);
  const selectedTask = useMemo(
    () => selectedTaskId ? tasks.find((t) => t.id === selectedTaskId) : undefined,
    [tasks, selectedTaskId]
  );

  // Determine effective status - use historical status in history mode, otherwise current status
  const effectiveStatus = taskHistoryState?.status ?? selectedTask?.internalStatus;

  // Execution states: worker agent is running (only when NOT in ideation mode)
  const isExecutionMode = !ideationSessionId && effectiveStatus
    ? (EXECUTION_STATUSES as readonly string[]).includes(effectiveStatus)
    : false;

  // Review states: reviewer agent conversation (only when NOT in ideation mode)
  // Note: Uses HUMAN_REVIEW_STATUSES (review_passed, escalated) + "reviewing" (AI review in progress)
  const isReviewMode = !ideationSessionId && effectiveStatus
    ? (HUMAN_REVIEW_STATUSES as readonly string[]).includes(effectiveStatus) ||
      effectiveStatus === "reviewing"
    : false;

  // Use extracted context management hook
  const {
    chatContext,
    storeContextKey,
    currentContextType,
    currentContextId,
    activeConversationId,
    streamingToolCalls,
    setStreamingToolCalls,
    autoSelectConversation,
  } = useChatPanelContext({
    projectId,
    ideationSessionId,
    selectedTaskId: selectedTaskId ?? undefined,
    isExecutionMode,
    isReviewMode,
  });

  // Use context-aware selectors - unified queue works for all modes
  const queuedMessagesSelector = useMemo(() => selectQueuedMessages(storeContextKey), [storeContextKey]);
  const queuedMessages = useChatStore(queuedMessagesSelector);
  const isAgentRunningSelector = useMemo(() => selectIsAgentRunning(storeContextKey), [storeContextKey]);
  const isAgentRunning = useChatStore(isAgentRunningSelector);

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

  // Auto-select the most recent conversation in execution/review modes
  useEffect(() => {
    autoSelectConversation(
      conversations,
      executionConversationsQuery.isLoading,
      reviewConversationsQuery.isLoading
    );
  }, [autoSelectConversation, conversations, executionConversationsQuery.isLoading, reviewConversationsQuery.isLoading]);

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

  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Track scroll settling period - hide messages until scroll animation completes
  const [isScrollSettling, setIsScrollSettling] = useState(false);
  const prevConversationIdRef = useRef<string | null>(null);

  // When conversation changes, enter settling mode until scroll completes
  useEffect(() => {
    // Only trigger settling when conversation actually changes to a new one
    if (activeConversationId === prevConversationIdRef.current) {
      return undefined;
    }

    prevConversationIdRef.current = activeConversationId;

    // If we have a new conversation with messages, enter settling mode
    if (!activeConversationId || activeConversation.isLoading) {
      return undefined;
    }

    setIsScrollSettling(true);

    // Match the scroll delay in ChatMessageList (300ms) + small buffer
    const timeoutId = setTimeout(() => {
      setIsScrollSettling(false);
    }, 350);

    return () => clearTimeout(timeoutId);
  }, [activeConversationId, activeConversation.isLoading]);

  // Extract messages array from active conversation
  // Only show messages if conversation belongs to current context
  const conversationContext = activeConversation.data?.conversation;
  const isConversationInCurrentContext =
    conversationContext?.contextType === currentContextType &&
    conversationContext?.contextId === currentContextId;

  // Memoize messagesData to avoid dependency chain issues in useEffect hooks
  // No time-based filtering needed - we switch context types based on historical state
  const messagesData = useMemo(
    () =>
      activeConversationId && isConversationInCurrentContext
        ? (activeConversation.data?.messages ?? [])
        : [],
    [activeConversationId, isConversationInCurrentContext, activeConversation.data?.messages]
  );

  // History mode: viewing past task state via time-travel navigation
  const isHistoryMode = !!taskHistoryState;

  // Debug logging for history mode
  console.log('[IntegratedChatPanel] Context mode:', {
    isHistoryMode,
    effectiveStatus,
    isExecutionMode,
    isReviewMode,
    taskHistoryState,
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
    messageCount: messagesData.length,
  });

  // Wrapper for handleEditLastQueued that provides the queued messages
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

  // Handle Escape key to close panel
  useEffect(() => {
    if (!onClose) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  // Sort messages by createdAt - render in chronological order, no grouping
  const sortedMessages = useMemo(() => {
    return [...messagesData].sort((a, b) =>
      new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime()
    );
  }, [messagesData]);

  // Loading state: show skeleton when conversations list is loading OR active conversation is loading
  // OR scroll is settling (hides the scroll animation when switching conversations)
  const isConversationsLoading = conversations.isLoading;
  const isActiveConversationLoading = activeConversationId ? activeConversation.isLoading : false;
  const isLoading = isConversationsLoading || isActiveConversationLoading || isScrollSettling;

  const isSending = sendMessage.isPending;

  // Status badge helpers - disabled in history mode (no live agent)
  const isAgentActive = !isHistoryMode && (isSending || isAgentRunning || isExecutionMode);
  const agentType: AgentType = isHistoryMode
    ? "idle"
    : isExecutionMode
      ? "worker"
      : isReviewMode
        ? "reviewer"
        : (isSending || isAgentRunning)
          ? "agent"
          : "idle";

  // Empty state: only show when we KNOW there are no messages (not while loading)
  // Also don't show empty if conversations are loading - we might auto-select one
  const hasNoConversations = !isConversationsLoading && (conversations.data?.length ?? 0) === 0;
  const hasEmptyConversation = !isLoading && activeConversationId && sortedMessages.length === 0;
  const isEmpty = hasNoConversations || hasEmptyConversation;

  return (
    <>
      <style>{animationStyles}</style>
      {/* Outer container - matches main content bg for unified surface */}
      <div
        data-testid="integrated-chat-panel"
        className="h-full flex flex-col overflow-hidden"
        style={{
          backgroundColor: "transparent", /* Let parent bg show through */
          padding: "8px", /* Equal padding all sides - floating glass element */
        }}
      >
        {/* Inner rounded container - flat with blur */}
        <div
          className="flex-1 flex flex-col overflow-hidden"
          style={{
            borderRadius: "10px",
            /* FLAT semi-transparent (no gradient) */
            background: "hsla(220 10% 10% / 0.92)",
            backdropFilter: "blur(20px) saturate(180%)",
            WebkitBackdropFilter: "blur(20px) saturate(180%)",
            /* Luminous perimeter edge */
            border: "1px solid hsla(220 20% 100% / 0.08)",
            boxShadow: `
              0 4px 16px hsla(220 20% 0% / 0.4),
              0 12px 32px hsla(220 20% 0% / 0.3)
            `,
          }}
        >
          {/* Header - subtle separation within glass container */}
          <div
            data-testid="integrated-chat-header"
            className="flex items-center justify-between h-11 px-3 shrink-0"
            style={{
              backgroundColor: "hsla(220 15% 5% / 0.5)",
              borderBottom: "1px solid hsla(220 20% 100% / 0.04)",
            }}
          >
            {headerContent ?? <ContextIndicator context={chatContext} isExecutionMode={isExecutionMode} isReviewMode={isReviewMode} />}

            {/* Unified status + activity badge */}
            <StatusActivityBadge
              isAgentActive={isAgentActive}
              agentType={agentType}
              contextType={chatContext.view}
              contextId={ideationSessionId || selectedTaskId || null}
            />

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
            <ChatMessageList
              ref={virtuosoRef}
              messages={sortedMessages}
              conversationId={activeConversationId}
              failedRun={showFailedBanner && failedRun ? { id: failedRun.id, errorMessage: failedRun.errorMessage! } : null}
              onDismissFailedRun={setDismissedErrorId}
              isSending={isSending}
              isAgentRunning={isAgentRunning}
              streamingToolCalls={streamingToolCalls}
              messagesEndRef={messagesEndRef}
            />
          )}

          {/* Input Area - subtle separation within glass container */}
          <div
            className={inputContainerClassName ?? "shrink-0"}
            style={inputContainerClassName ? undefined : {
              backgroundColor: "hsla(220 15% 5% / 0.5)",
              borderTop: "1px solid hsla(220 20% 100% / 0.04)",
            }}
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
                isReadOnly={isHistoryMode}
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
      </div>
    </>
  );
}
