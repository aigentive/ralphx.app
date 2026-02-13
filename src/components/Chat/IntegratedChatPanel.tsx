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

import { useState, useRef, useEffect, useMemo, useCallback } from "react";
import { useShallow } from "zustand/react/shallow";
import { type VirtuosoHandle } from "react-virtuoso";
import { useChat, chatKeys } from "@/hooks/useChat";
import { useChatStore, selectQueuedMessages, selectIsAgentRunning, selectIsSending } from "@/stores/chatStore";
import { useUiStore } from "@/stores/uiStore";
import { useTasks } from "@/hooks/useTasks";
import { useChatPanelContext } from "@/hooks/useChatPanelContext";
import { useQuery } from "@tanstack/react-query";
import { chatApi } from "@/api/chat";
import type { ContextType } from "@/types/chat-conversation";
import { ALL_REVIEW_STATUSES, EXECUTION_STATUSES, MERGE_STATUSES } from "@/types/status";
import { AGENT_WORKER, AGENT_REVIEWER } from "@/constants/agents";
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
  HistoryEmptyState,
} from "./IntegratedChatPanel.components";
import { useChatActions } from "@/hooks/useChatActions";
import { useChatEvents } from "@/hooks/useChatEvents";
import { useChatRecovery } from "@/hooks/useChatRecovery";
// useAgentEvents is already called inside useChat — no direct import needed
import { useAskUserQuestion } from "@/hooks/useAskUserQuestion";
import { useQuestionInput } from "@/hooks/useQuestionInput";
import { QuestionInputBanner } from "./QuestionInputBanner";
import { RecoveryPromptDialog } from "@/components/recovery/RecoveryPromptDialog";
import { useEventBus } from "@/providers/EventProvider";
import { logger } from "@/lib/logger";
import { ChildSessionNotification } from "./ChildSessionNotification";
import { useIdeationStore } from "@/stores/ideationStore";

// Stable empty array to avoid new reference on every render when tasks query returns undefined
const EMPTY_TASKS: never[] = [];

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
  /** Whether to autofocus chat input on mount */
  autoFocusInput?: boolean;
}

export function IntegratedChatPanel({
  projectId,
  ideationSessionId,
  emptyState,
  showHelperTextAlways = false,
  inputContainerClassName,
  headerContent,
  onClose,
  autoFocusInput = true,
}: IntegratedChatPanelProps) {
  const bus = useEventBus();
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  // History state from store - shared with TaskDetailOverlay for time-travel feature
  const taskHistoryState = useUiStore((s) => s.taskHistoryState);
  const isHistoryMode = !!taskHistoryState;
  const hasHistoryConversation = !!taskHistoryState?.conversationId;

  // Get task data from React Query (useTasks) which has full task data
  const { data: tasks = EMPTY_TASKS } = useTasks(projectId);
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
  // Include 'approved' so historical view loads the reviewer's conversation
  const isReviewMode = !ideationSessionId && effectiveStatus
    ? (ALL_REVIEW_STATUSES as readonly string[]).includes(effectiveStatus) || effectiveStatus === "approved"
    : false;

  // Merge states: merger agent conversation (only when NOT in ideation mode)
  const isMergeMode = !ideationSessionId && effectiveStatus
    ? (MERGE_STATUSES as readonly string[]).includes(effectiveStatus)
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
    streamingContentBlocks,
    setStreamingContentBlocks,
    streamingTasks,
    setStreamingTasks,
    autoSelectConversation,
    // overrideAgentRunId is available but we use taskHistoryState.timestamp for scroll positioning
  } = useChatPanelContext({
    projectId,
    ideationSessionId,
    selectedTaskId: selectedTaskId ?? undefined,
    isExecutionMode,
    isReviewMode,
    isMergeMode,
    isHistoryMode,
    // Pass history mode overrides for conversation selection
    overrideConversationId: taskHistoryState?.conversationId,
    overrideAgentRunId: taskHistoryState?.agentRunId,
  });

  const setActiveConversation = useChatStore((s) => s.setActiveConversation);

  // Agent lifecycle events (useAgentEvents) are handled inside useChat — no duplicate subscription needed.

  // If a new run starts in this context, switch to its conversation (live mode only)
  useEffect(() => {
    if (isHistoryMode) {
      return undefined;
    }

    return bus.subscribe<{
      context_type: string;
      context_id: string;
      conversation_id: string;
    }>("agent:run_started", (payload) => {
      // Existing exact match
      if (
        payload.context_type === currentContextType &&
        payload.context_id === currentContextId &&
        payload.conversation_id
      ) {
        setActiveConversation(payload.conversation_id);
        return;
      }
      // Handle retry scenario: task context watching a new execution starting
      // When task is in failed/ready state, currentContextType is "task" but
      // the new execution emits "task_execution". Accept if task ID matches.
      if (
        payload.context_type === "task_execution" &&
        currentContextType === "task" &&
        payload.context_id === currentContextId &&
        payload.conversation_id
      ) {
        setActiveConversation(payload.conversation_id);
      }
    });
  }, [bus, currentContextType, currentContextId, isHistoryMode, setActiveConversation]);

  // Use context-aware selectors - unified queue works for all modes
  const queuedMessagesSelector = useMemo(() => selectQueuedMessages(storeContextKey), [storeContextKey]);
  const queuedMessages = useChatStore(queuedMessagesSelector);
  const isAgentRunningSelector = useMemo(() => selectIsAgentRunning(storeContextKey), [storeContextKey]);
  const isAgentRunning = useChatStore(isAgentRunningSelector);
  const isSendingSelector = useMemo(() => selectIsSending(storeContextKey), [storeContextKey]);
  const isSending = useChatStore(isSendingSelector);
  const setAgentRunning = useChatStore((s) => s.setAgentRunning);

  // For execution/review mode, fetch conversations directly with specific context type
  const regularChatData = useChat(chatContext);

  // Single dynamic query for all agent contexts (execution/review/merge)
  // When currentContextType changes, the query key changes and a fresh fetch fires
  const isAgentContext = isExecutionMode || isReviewMode || isMergeMode;

  const agentConversationsQuery = useQuery({
    queryKey: chatKeys.conversationList(currentContextType, selectedTaskId ?? ""),
    queryFn: () => chatApi.listConversations(currentContextType as ContextType, selectedTaskId ?? ""),
    enabled: isAgentContext && !!selectedTaskId,
  });

  // Use agent query for agent contexts, regular chat data otherwise
  const conversations = isAgentContext
    ? agentConversationsQuery
    : regularChatData.conversations;

  // Auto-select the most recent conversation in execution/review/merge modes
  // Extract stable primitives from TanStack Query result to avoid re-render on every query object change
  const conversationsData = conversations.data;
  const conversationsLoading = conversations.isLoading;
  useEffect(() => {
    autoSelectConversation({ data: conversationsData, isLoading: conversationsLoading });
  }, [autoSelectConversation, conversationsData, conversationsLoading]);

  // Check if active conversation belongs to current context (needed by recovery effects below)
  const activeConversationContext = regularChatData.messages.data?.conversation;
  const isConversationInCurrentContext = useMemo(
    () =>
      (activeConversationContext?.contextType === currentContextType ||
       (currentContextType === "task" && activeConversationContext?.contextType === "task_execution")) &&
      activeConversationContext?.contextId === currentContextId,
    [activeConversationContext?.contextType, activeConversationContext?.contextId,
     currentContextType, currentContextId]
  );

  // Fetch agent run status for the active conversation
  const agentRunQuery = useQuery({
    queryKey: chatKeys.agentRun(activeConversationId ?? ""),
    queryFn: () => activeConversationId ? chatApi.getAgentRunStatus(activeConversationId) : null,
    enabled: !!activeConversationId,
    staleTime: 5000,
  });

  // Recovery and polling effects (extracted to hook)
  useChatRecovery({
    activeConversationId,
    storeContextKey,
    currentContextType,
    isHistoryMode,
    isAgentContext,
    isAgentRunning,
    isConversationInCurrentContext,
    agentRunStatus: agentRunQuery.data?.status ?? undefined,
    setAgentRunning,
    selectedTaskId: selectedTaskId ?? undefined,
    ideationSessionId,
    projectId,
    effectiveStatus,
  });

  // Track dismissed error banners by run ID
  const [dismissedErrorId, setDismissedErrorId] = useState<string | null>(null);
  const failedRun = agentRunQuery.data?.status === "failed" ? agentRunQuery.data : null;
  const showFailedBanner = failedRun && failedRun.errorMessage && failedRun.id !== dismissedErrorId;

  // Memoize failedRun prop to avoid creating a new object reference each render,
  // which would bust ChatMessageList's virtuosoComponents useMemo via the failedRun dep.
  const failedRunProp = useMemo(
    () => showFailedBanner && failedRun ? { id: failedRun.id, errorMessage: failedRun.errorMessage! } : null,
    [showFailedBanner, failedRun]
  );

  const {
    messages: activeConversation,
    sendMessage,
    switchConversation: handleSelectConversation,
    createConversation: handleNewConversation,
  } = regularChatData;

  const virtuosoRef = useRef<VirtuosoHandle>(null);


  // Memoize messagesData to avoid dependency chain issues in useEffect hooks
  // No time-based filtering needed - we switch context types based on historical state
  const messagesData = useMemo(
    () =>
      activeConversationId && isConversationInCurrentContext
        ? (activeConversation.data?.messages ?? [])
        : [],
    [activeConversationId, isConversationInCurrentContext, activeConversation.data?.messages]
  );

  // Debug logging for history mode
  logger.debug('[IntegratedChatPanel] Context mode:', {
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
  } = useChatActions({
    contextType: currentContextType,
    contextId: currentContextId,
    storeContextKey,
    selectedTaskId: selectedTaskId ?? undefined,
    ideationSessionId,
    sendMessage,
    messageCount: messagesData.length,
  });

  // Wrapper for handleEditLastQueued that provides the queued messages
  const handleEditLastQueuedWrapper = () => {
    handleEditLastQueued(queuedMessages);
  };

  // Handle stopping agent - clear streaming state
  const handleStopAgentWrapper = useCallback(async () => {
    await handleStopAgent();
    setStreamingToolCalls(prev => prev.length === 0 ? prev : []);
    setStreamingContentBlocks(prev => prev.length === 0 ? prev : []);
    setStreamingTasks(prev => prev.size === 0 ? prev : new Map());
  }, [handleStopAgent, setStreamingToolCalls, setStreamingContentBlocks, setStreamingTasks]);

  useChatEvents({
    activeConversationId,
    contextId: currentContextId,
    contextType: currentContextType,
    setStreamingToolCalls,
    setStreamingContentBlocks,
    setStreamingTasks,
  });

  // Ask user question state — scoped to current context (ideation session, task, or project)
  const {
    activeQuestion,
    answeredQuestion,
    submitAnswer,
    dismissQuestion,
    clearAnswered,
    isLoading: isSubmittingAnswer,
  } = useAskUserQuestion(currentContextId);

  // Question UI state — chip selection, input sync, question-aware send
  const {
    selectedOptions,
    questionInputValue,
    setQuestionInputValue,
    handleChipClick,
    handleMatchedOptions,
    handleQuestionSend,
  } = useQuestionInput({
    activeQuestion: activeQuestion ?? null,
    submitAnswer,
    handleSend,
  });

  // Ideation store for session navigation
  const selectSession = useIdeationStore((s) => s.selectSession);
  const allSessions = useIdeationStore(useShallow((s) => Object.values(s.sessions)));

  // Handler for navigating to child session
  const handleNavigateToChildSession = useCallback((childSessionId: string) => {
    const session = allSessions.find((s) => s.id === childSessionId);
    if (session) {
      selectSession(session);
    }
  }, [allSessions, selectSession]);

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
  const isConversationsLoading = conversations.isLoading;
  const isActiveConversationLoading = activeConversationId ? activeConversation.isLoading : false;
  const isLoading = isConversationsLoading || isActiveConversationLoading;

  // Status badge helpers - disabled in history mode (no live agent)
  // Only show active state when an agent run is actually happening (not based on workflow status)
  const isAgentActive = !isHistoryMode && (isSending || isAgentRunning);
  const agentType: AgentType = isHistoryMode
    ? "idle"
    : isExecutionMode
      ? AGENT_WORKER
      : isReviewMode
        ? AGENT_REVIEWER
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
      <RecoveryPromptDialog surface="chat" taskId={selectedTaskId ?? undefined} />
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
                  : isMergeMode
                    ? "merge"
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
              {emptyState ??
                (isHistoryMode && !hasHistoryConversation ? (
                  <HistoryEmptyState />
                ) : (
                  <EmptyState />
                ))}
            </div>
          ) : (
            <ChatMessageList
              ref={virtuosoRef}
              messages={sortedMessages}
              conversationId={activeConversationId}
              failedRun={failedRunProp}
              onDismissFailedRun={setDismissedErrorId}
              isSending={isSending}
              isAgentRunning={isAgentRunning}
              streamingToolCalls={streamingToolCalls}
              streamingTasks={streamingTasks}
              streamingContentBlocks={streamingContentBlocks}
              scrollToTimestamp={isHistoryMode ? taskHistoryState?.timestamp : null}
            />
          )}

          {/* Child Session Notification - shows when follow-up is created (ideation mode only) */}
          {ideationSessionId && !isHistoryMode && (
            <ChildSessionNotification
              sessionId={ideationSessionId}
              onNavigateToSession={handleNavigateToChildSession}
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

            {/* Question Input Banner - renders above ChatInput when question is active */}
            {(activeQuestion || answeredQuestion) && (
              <QuestionInputBanner
                question={activeQuestion ?? null}
                selectedIndices={selectedOptions}
                onChipClick={handleChipClick}
                onDismiss={dismissQuestion}
                answeredValue={answeredQuestion}
                onDismissAnswered={clearAnswered}
              />
            )}

            {/* Chat Input */}
            <div className="p-3">
              <ChatInput
                onSend={activeQuestion ? handleQuestionSend : handleSend}
                onQueue={handleQueue}
                onStop={handleStopAgentWrapper}
                isAgentRunning={isAgentRunning}
                isSending={isSending || isSubmittingAnswer}
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
                showHelperText={showHelperTextAlways || queuedMessages.length > 0 || !!activeQuestion}
                {...(activeQuestion ? {
                  value: questionInputValue,
                  onChange: setQuestionInputValue,
                  questionMode: {
                    optionCount: activeQuestion.options.length,
                    multiSelect: activeQuestion.multiSelect,
                    onMatchedOptions: handleMatchedOptions,
                  },
                } : {})}
                autoFocus={autoFocusInput}
              />
            </div>
          </div>
        </div>
      </div>
    </>
  );
}
