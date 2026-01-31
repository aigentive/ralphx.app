/**
 * TaskChatPanel - Embedded chat panel for TaskFullView
 *
 * Reuses ChatPanel internals but without resize/collapse functionality.
 * Shows context-aware chat based on task state (execution/review/discussion).
 */

import { useRef, useEffect, useCallback, useMemo } from "react";
import { useEventBus } from "@/providers/EventProvider";
import { CHAT_TOOL_CALL, CHAT_RUN_COMPLETED } from "@/lib/events";
import { useTaskChat, type TaskContextType } from "@/hooks/useTaskChat";
import { chatKeys } from "@/hooks/useChat";
import { useChatStore, selectQueuedMessages, selectIsAgentRunning } from "@/stores/chatStore";
import { useQueryClient } from "@tanstack/react-query";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import {
  MessageSquare,
  CheckSquare,
  Bot,
  Loader2,
  Hammer,
} from "lucide-react";
import { StatusActivityBadge, type AgentType } from "../Chat/StatusActivityBadge";
import { ConversationSelector } from "../Chat/ConversationSelector";
import { QueuedMessageList } from "../Chat/QueuedMessageList";
import { ChatInput } from "../Chat/ChatInput";
import { MessageItem } from "../Chat/MessageItem";

// ============================================================================
// Constants
// ============================================================================

const animationStyles = `
@keyframes typingBounce {
  0%, 60%, 100% { transform: translateY(0); }
  30% { transform: translateY(-4px); }
}

.typing-dot {
  animation: typingBounce 1.4s ease-in-out infinite;
}

.typing-dot:nth-child(2) { animation-delay: 0.15s; }
.typing-dot:nth-child(3) { animation-delay: 0.3s; }
`;

// ============================================================================
// Sub-components
// ============================================================================

function TypingIndicator() {
  return (
    <div
      data-testid="task-chat-typing-indicator"
      className="flex items-start gap-2 mb-2"
    >
      <Bot className="w-3.5 h-3.5 mt-2 shrink-0 text-white/40" />
      <div
        className="px-3 py-2 rounded-[10px_10px_10px_4px]"
        style={{
          background: "linear-gradient(180deg, rgba(28,28,28,0.95) 0%, rgba(22,22,22,0.98) 100%)",
          border: "1px solid rgba(255,255,255,0.06)",
        }}
      >
        <div className="flex items-center gap-1">
          <div className="typing-dot w-1.5 h-1.5 rounded-full bg-white/30" />
          <div className="typing-dot w-1.5 h-1.5 rounded-full bg-white/30" />
          <div className="typing-dot w-1.5 h-1.5 rounded-full bg-white/30" />
        </div>
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div
      data-testid="task-chat-empty"
      className="flex flex-col items-center justify-center h-full p-6 text-center"
    >
      <div
        className="w-12 h-12 rounded-xl flex items-center justify-center mb-3"
        style={{
          background: "linear-gradient(135deg, rgba(255,107,53,0.1) 0%, rgba(255,107,53,0.05) 100%)",
          border: "1px solid rgba(255,107,53,0.15)",
        }}
      >
        <MessageSquare className="w-5 h-5 text-[#ff6b35]" />
      </div>
      <p className="text-[13px] font-medium text-white/80">
        Start a conversation
      </p>
      <p className="text-xs mt-1 text-white/40">
        Ask questions or get help with this task
      </p>
    </div>
  );
}

function LoadingState() {
  return (
    <div
      data-testid="task-chat-loading"
      className="flex items-center justify-center p-6"
    >
      <Loader2 className="w-5 h-5 animate-spin text-[#ff6b35]" />
    </div>
  );
}

interface ContextIndicatorProps {
  isExecutionMode: boolean;
}

interface ChatModeIndicatorProps {
  isLive: boolean;
}

function ChatModeIndicator({ isLive }: ChatModeIndicatorProps) {
  if (!isLive) {
    return (
      <Badge
        variant="secondary"
        className="shrink-0 text-xs"
        style={{
          background: "rgba(255,255,255,0.05)",
          border: "1px solid rgba(255,255,255,0.1)",
        }}
      >
        Completed
      </Badge>
    );
  }

  return (
    <Badge
      variant="secondary"
      className="shrink-0 text-xs flex items-center gap-1"
      style={{
        background: "rgba(255,107,53,0.1)",
        border: "1px solid rgba(255,107,53,0.2)",
        color: "#ff6b35",
      }}
    >
      <div
        className="w-1.5 h-1.5 rounded-full bg-[#ff6b35]"
        style={{
          animation: "pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite",
        }}
      />
      Live
    </Badge>
  );
}

function ContextIndicator({ isExecutionMode, isReviewMode }: ContextIndicatorProps & { isReviewMode: boolean }) {
  const Icon = isExecutionMode ? Hammer : isReviewMode ? Bot : CheckSquare;
  const label = isExecutionMode ? "Worker Execution" : isReviewMode ? "AI Review" : "Task";

  return (
    <div className="flex items-center gap-2 min-w-0 flex-1">
      <Icon className="w-3.5 h-3.5 shrink-0 text-white/50" />
      <span className="text-[13px] font-medium truncate text-white/80">{label}</span>
    </div>
  );
}

// MessageItem is now imported from "../Chat/MessageItem" - shared component

// ============================================================================
// Main Component
// ============================================================================

export interface TaskChatPanelProps {
  taskId: string;
  /** Context type - 'task' for regular chat, 'task_execution' for worker execution, 'review' for reviewer */
  contextType: TaskContextType;
  /** Current task internal status - used to determine if chat is live or historical */
  taskStatus: string;
}

export function TaskChatPanel({ taskId, contextType, taskStatus }: TaskChatPanelProps) {
  console.log(`[TaskChatPanel] props: taskId=${taskId}, contextType=${contextType}, taskStatus=${taskStatus}`);
  const queryClient = useQueryClient();
  const {
    queueMessage,
    editQueuedMessage,
    deleteQueuedMessage,
    startEditingQueuedMessage,
  } = useChatStore();

  // Use the new useTaskChat hook - single hook call handles all context types
  const {
    conversations,
    messages: messagesData,
    isLoading: hookIsLoading,
    activeConversationId,
    contextKey,
    sendMessage,
    switchConversation: handleSelectConversation,
    createConversation: handleNewConversation,
  } = useTaskChat(taskId, contextType);

  const isExecutionMode = contextType === "task_execution";
  const isReviewMode = contextType === "review";

  // Determine if chat is live or historical based on task state
  const isLive = useMemo(() => {
    if (contextType === "task_execution") {
      return taskStatus === "executing" || taskStatus === "re_executing";
    }
    if (contextType === "review") {
      // Live when actively reviewing OR after AI review passed (user can chat with review-chat agent)
      return taskStatus === "reviewing" || taskStatus === "review_passed";
    }
    // Regular task chat is always live
    return true;
  }, [contextType, taskStatus]);

  // Use context-aware selectors with contextKey from hook (e.g., "task:id", "task_execution:id", "review:id")
  const queuedMessagesSelector = useMemo(() => selectQueuedMessages(contextKey), [contextKey]);
  const queuedMessages = useChatStore(queuedMessagesSelector);
  const isAgentRunningSelector = useMemo(() => selectIsAgentRunning(contextKey), [contextKey]);
  const isAgentRunning = useChatStore(isAgentRunningSelector);
  const { setAgentRunning } = useChatStore();

  // Track previous isLive to detect transitions
  const prevIsLiveRef = useRef(isLive);

  // Clear agent state when chat becomes historical (isLive: true → false)
  useEffect(() => {
    if (prevIsLiveRef.current && !isLive) {
      // isLive transitioned from true → false, clear stale agent state
      setAgentRunning(contextKey, false);
    }
    prevIsLiveRef.current = isLive;
  }, [isLive, contextKey, setAgentRunning]);

  // Streaming state - accumulates text chunks as they arrive
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  // Ref for activeConversationId so event listeners always have current value
  const activeConversationIdRef = useRef(activeConversationId);

  useEffect(() => {
    activeConversationIdRef.current = activeConversationId;
  }, [activeConversationId]);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    if (messagesEndRef.current && messagesData.length) {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messagesData.length]);

  // Send message handler
  const handleSend = useCallback(
    async (content: string) => {
      if (!content.trim() || sendMessage.isPending) return;

      try {
        await sendMessage.mutateAsync(content);
      } catch {
        // Error is handled by the mutation
      }
    },
    [sendMessage]
  );

  // Queue message handler (when agent is running) - uses unified queue with context-aware keys
  const handleQueue = useCallback(
    (content: string) => {
      if (!content.trim()) return;
      queueMessage(contextKey, content);
    },
    [queueMessage, contextKey]
  );

  // Edit last queued message
  const handleEditLastQueued = useCallback(() => {
    const lastMessage = queuedMessages[queuedMessages.length - 1];
    if (!lastMessage) return;
    startEditingQueuedMessage(contextKey, lastMessage.id);
  }, [queuedMessages, startEditingQueuedMessage, contextKey]);

  // Get the event bus from context (TauriEventBus or MockEventBus)
  const eventBus = useEventBus();

  // Subscribe to events for real-time updates (only on mount)
  useEffect(() => {
    // Listen for tool calls - invalidate cache to pick up new messages
    const toolCallUnsub = eventBus.subscribe<{
      tool_name: string;
      arguments: unknown;
      result: unknown;
      conversation_id: string;
    }>(CHAT_TOOL_CALL, (payload) => {
      const { conversation_id } = payload;
      // Invalidate cache to pick up any new messages from backend
      if (conversation_id === activeConversationIdRef.current) {
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(conversation_id),
        });
      }
    });

    // Listen for chat run completion - refresh messages
    const runCompletedUnsub = eventBus.subscribe<{
      conversation_id: string;
    }>(CHAT_RUN_COMPLETED, (payload) => {
      const { conversation_id } = payload;
      // Invalidate cache to get final messages
      if (conversation_id) {
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversation(conversation_id),
        });
      }
      // Scroll to bottom
      setTimeout(() => {
        if (messagesEndRef.current) {
          messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
        }
      }, 100);
    });

    // Note: execution:* and review:* events are now unified under chat:* events
    // The backend emits chat:tool_call and chat:run_completed for all context types

    return () => {
      toolCallUnsub();
      runCompletedUnsub();
    };
  }, [eventBus, queryClient]);

  // Sort messages by createdAt - render in chronological order, no grouping
  const sortedMessages = useMemo(() => {
    return [...messagesData].sort((a, b) =>
      new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime()
    );
  }, [messagesData]);

  // Use unified loading state from hook
  const isLoading = hookIsLoading;
  const isSending = sendMessage.isPending;
  const isEmpty = !isLoading && sortedMessages.length === 0;

  return (
    <>
      <style>{animationStyles}</style>
      <div
        data-testid="task-chat-panel"
        className="flex flex-col h-full"
        style={{
          backgroundColor: "var(--bg-surface)",
        }}
      >
        {/* Header - Glass effect */}
        <div
          data-testid="task-chat-panel-header"
          className="flex items-center justify-between h-11 px-3 border-b backdrop-blur-sm shrink-0"
          style={{
            borderColor: "rgba(255,255,255,0.06)",
            background: "linear-gradient(180deg, rgba(26,26,26,0.95) 0%, rgba(20,20,20,0.98) 100%)",
          }}
        >
          <ContextIndicator isExecutionMode={isExecutionMode} isReviewMode={isReviewMode} />

          {/* Chat mode indicator */}
          <ChatModeIndicator isLive={isLive} />

          {/* Unified status + activity badge */}
          {isLive && (
            <StatusActivityBadge
              isAgentActive={isSending || isAgentRunning || isExecutionMode}
              agentType={
                isExecutionMode
                  ? "worker"
                  : isReviewMode
                    ? "reviewer"
                    : (isSending || isAgentRunning)
                      ? "agent"
                      : "idle" as AgentType
              }
              contextType="task_detail"
              contextId={taskId}
            />
          )}

          <div className="flex items-center gap-1 shrink-0">
            {/* Conversation Selector */}
            <ConversationSelector
              contextType={isExecutionMode ? "task_execution" : isReviewMode ? "review" : "task"}
              contextId={taskId}
              conversations={conversations.data ?? []}
              activeConversationId={activeConversationId}
              onSelectConversation={handleSelectConversation}
              onNewConversation={handleNewConversation}
              isLoading={conversations.isLoading}
            />
          </div>
        </div>

        {/* Messages Area */}
        <ScrollArea
          ref={scrollAreaRef}
          className="flex-1"
          data-testid="task-chat-panel-messages"
        >
          <div className="p-3">
            {isLoading ? (
              <LoadingState />
            ) : isEmpty ? (
              <EmptyState />
            ) : (
              <>
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
                {/* Show typing indicator while agent is working (not streaming text) */}
                {(isSending || isAgentRunning) && <TypingIndicator />}
                <div ref={messagesEndRef} />
              </>
            )}
          </div>
        </ScrollArea>

        {/* Input Area */}
        <div className="border-t shrink-0" style={{ borderColor: "var(--border-subtle)" }}>
          {isLive ? (
            <>
              {/* Queued Messages - unified queue with context-aware keys */}
              {queuedMessages.length > 0 && (
                <div className="p-3 pb-0">
                  <QueuedMessageList
                    messages={queuedMessages}
                    onEdit={(id, content) => editQueuedMessage(contextKey, id, content)}
                    onDelete={(id) => deleteQueuedMessage(contextKey, id)}
                  />
                </div>
              )}

              {/* Chat Input */}
              <div className="p-3">
                <ChatInput
                  onSend={handleSend}
                  onQueue={handleQueue}
                  isAgentRunning={isExecutionMode || isAgentRunning}
                  isSending={isSending}
                  hasQueuedMessages={queuedMessages.length > 0}
                  onEditLastQueued={handleEditLastQueued}
                  placeholder={
                    isExecutionMode
                      ? "Message worker... (will be sent when current response completes)"
                      : isReviewMode
                        ? "Chat with the reviewer..."
                        : "Send a message..."
                  }
                  showHelperText={queuedMessages.length > 0}
                  autoFocus
                />
              </div>
            </>
          ) : (
            /* Historical mode - read-only */
            <div
              className="px-4 py-3 text-center"
              style={{
                background: "linear-gradient(180deg, rgba(255,255,255,0.02) 0%, transparent 100%)",
              }}
            >
              <p className="text-[13px] text-white/50">
                Chat ended — {contextType === "review" ? "Review" : "Execution"} completed
              </p>
            </div>
          )}
        </div>
      </div>
    </>
  );
}
