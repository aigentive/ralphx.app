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
import { useChatStore, selectQueuedMessages, selectIsAgentRunning, selectActiveConversationId, selectExecutionQueuedMessages, getContextKey } from "@/stores/chatStore";
import { useUiStore } from "@/stores/uiStore";
import { useTaskStore } from "@/stores/taskStore";
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

  // Detect execution mode based on selected task status
  const selectedTask = useTaskStore((state) =>
    selectedTaskId ? state.tasks[selectedTaskId] : undefined
  );
  const isExecutionMode = selectedTask?.internalStatus === "executing";

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
        view: isExecutionMode ? "task_detail" : "task_detail",
        projectId,
        selectedTaskId,
      };
    }
    return {
      view: "kanban",
      projectId,
    };
  }, [selectedTaskId, isExecutionMode, projectId, ideationSessionId]);

  // Compute store context key for queue/agent state operations
  const storeContextKey = useMemo(() => getContextKey(chatContext), [chatContext]);

  // Use context-aware selectors
  const queuedMessagesSelector = useMemo(() => selectQueuedMessages(storeContextKey), [storeContextKey]);
  const queuedMessages = useChatStore(queuedMessagesSelector);
  const isAgentRunningSelector = useMemo(() => selectIsAgentRunning(storeContextKey), [storeContextKey]);
  const isAgentRunning = useChatStore(isAgentRunningSelector);

  // Debug: log context key and agent running state changes
  useEffect(() => {
    console.log(`[IntegratedChatPanel] storeContextKey=${storeContextKey}, isAgentRunning=${isAgentRunning}`);
  }, [storeContextKey, isAgentRunning]);

  // Streaming tool calls - accumulated during agent execution (defined early for context change effect)
  const [streamingToolCalls, setStreamingToolCalls] = useState<ToolCall[]>([]);

  // Reset active conversation when context changes
  // This ensures we load the correct conversations for the new context
  const contextKey = ideationSessionId
    ? `ideation:${ideationSessionId}`
    : selectedTaskId
      ? `${isExecutionMode ? "execution" : "task"}:${selectedTaskId}`
      : `project:${projectId}`;
  // Initialize with empty string to ensure cleanup runs on first mount
  // This prevents showing conversations from a different context
  const prevContextKeyRef = useRef("");
  const prevContextTypeRef = useRef<{ type: string; id: string } | null>(null);

  // Track the previous context type and id for cache invalidation
  useEffect(() => {
    const currentContextType = ideationSessionId
      ? "ideation"
      : selectedTaskId
        ? (isExecutionMode ? "task_execution" : "task")
        : "project";
    const currentContextId = ideationSessionId || selectedTaskId || projectId;
    prevContextTypeRef.current = { type: currentContextType, id: currentContextId };
  }, [selectedTaskId, isExecutionMode, projectId, ideationSessionId]);

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

      prevContextKeyRef.current = contextKey;
    }
  }, [contextKey, setActiveConversation, queryClient]);

  // Memoize the selector for execution queued messages
  const taskIdForQueue = selectedTaskId ?? "";
  const executionQueuedMessagesSelector = useMemo(
    () => selectExecutionQueuedMessages(taskIdForQueue),
    [taskIdForQueue]
  );
  const executionQueuedMessages = useChatStore(executionQueuedMessagesSelector);

  // For execution mode, fetch execution conversations directly
  const regularChatData = useChat(chatContext);

  // Fetch execution conversations when in execution mode
  const executionConversationsQuery = useQuery({
    queryKey: chatKeys.conversationList("task_execution", selectedTaskId ?? ""),
    queryFn: () => chatApi.listConversations("task_execution", selectedTaskId ?? ""),
    enabled: isExecutionMode && !!selectedTaskId,
  });

  // Use execution conversations when in execution mode, otherwise regular conversations
  const conversations = isExecutionMode
    ? executionConversationsQuery
    : regularChatData.conversations;

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
      ? (isExecutionMode ? "task_execution" : "task")
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
    selectedTaskId: selectedTaskId ?? undefined,
    projectId,
    ideationSessionId,
    storeContextKey,
    sendMessage,
  });

  // Wrapper for handleEditLastQueued that provides the queued messages
  const handleEditLastQueuedWrapper = () => {
    handleEditLastQueued(queuedMessages, executionQueuedMessages);
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
          {headerContent ?? <ContextIndicator context={chatContext} isExecutionMode={isExecutionMode} />}

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
          {/* Queued Messages */}
          {(() => {
            const messagesToDisplay = isExecutionMode ? executionQueuedMessages : queuedMessages;

            return messagesToDisplay.length > 0 && (
              <div className="p-3 pb-0">
                <QueuedMessageList
                  messages={messagesToDisplay}
                  onEdit={handleEditQueuedMessage}
                  onDelete={handleDeleteQueuedMessage}
                />
              </div>
            );
          })()}

          {/* Chat Input */}
          <div className="p-3">
            <ChatInput
              onSend={handleSend}
              onQueue={handleQueue}
              onStop={handleStopAgentWrapper}
              isAgentRunning={isExecutionMode || isAgentRunning}
              isSending={isSending}
              hasQueuedMessages={(isExecutionMode ? executionQueuedMessages : queuedMessages).length > 0}
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
              showHelperText={showHelperTextAlways || (isExecutionMode ? executionQueuedMessages : queuedMessages).length > 0}
              autoFocus
            />
          </div>
        </div>
      </div>
    </>
  );
}
