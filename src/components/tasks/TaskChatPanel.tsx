/**
 * TaskChatPanel - Embedded chat panel for TaskFullView
 *
 * Reuses ChatPanel internals but without resize/collapse functionality.
 * Shows context-aware chat based on task state (execution/review/discussion).
 */

import { useRef, useEffect, useCallback, useMemo } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useChat, chatKeys } from "@/hooks/useChat";
import { useChatStore, selectQueuedMessages, selectIsAgentRunning, selectActiveConversationId, selectExecutionQueuedMessages, getContextKey } from "@/stores/chatStore";
import type { ChatContext } from "@/types/chat";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { chatApi } from "@/api/chat";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  MessageSquare,
  CheckSquare,
  Bot,
  Loader2,
  Hammer,
} from "lucide-react";
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

function WorkerExecutingIndicator() {
  return (
    <div
      data-testid="task-chat-worker-executing"
      className="flex items-center gap-2 px-3 py-2 mb-2 rounded-lg"
      style={{
        background: "linear-gradient(135deg, rgba(255,107,53,0.08) 0%, rgba(255,107,53,0.03) 100%)",
        border: "1px solid rgba(255,107,53,0.15)",
      }}
    >
      <Hammer className="w-3.5 h-3.5 text-[#ff6b35]" />
      <div className="flex items-center gap-2 flex-1">
        <span className="text-[13px] font-medium text-white/80">Worker is executing...</span>
        <div className="flex items-center gap-1">
          <div className="typing-dot w-1.5 h-1.5 rounded-full bg-[#ff6b35]" />
          <div className="typing-dot w-1.5 h-1.5 rounded-full bg-[#ff6b35]" />
          <div className="typing-dot w-1.5 h-1.5 rounded-full bg-[#ff6b35]" />
        </div>
      </div>
    </div>
  );
}

interface ContextIndicatorProps {
  isExecutionMode: boolean;
}

function ContextIndicator({ isExecutionMode }: ContextIndicatorProps) {
  const Icon = isExecutionMode ? Hammer : CheckSquare;
  const label = isExecutionMode ? "Worker Execution" : "Task";

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
  /** Context type - 'task' for regular chat, 'task_execution' for worker execution */
  contextType: "task" | "task_execution";
}

export function TaskChatPanel({ taskId, contextType }: TaskChatPanelProps) {
  const queryClient = useQueryClient();
  const {
    queueMessage,
    editQueuedMessage,
    deleteQueuedMessage,
    startEditingQueuedMessage,
    queueExecutionMessage,
    deleteExecutionQueuedMessage,
  } = useChatStore();
  const activeConversationId = useChatStore(selectActiveConversationId);

  const isExecutionMode = contextType === "task_execution";

  // Memoize the selector to avoid creating new function references on each render
  const executionQueuedMessagesSelector = useMemo(
    () => selectExecutionQueuedMessages(taskId),
    [taskId]
  );
  const executionQueuedMessages = useChatStore(executionQueuedMessagesSelector);

  // Build context for chat hook
  const context: ChatContext = useMemo(() => ({
    view: "task_detail",
    projectId: "", // Will be populated by useChat
    selectedTaskId: taskId,
  }), [taskId]);

  // Compute store context key for queue/agent state operations
  const storeContextKey = useMemo(() => getContextKey(context), [context]);

  // Use context-aware selectors
  const queuedMessagesSelector = useMemo(() => selectQueuedMessages(storeContextKey), [storeContextKey]);
  const queuedMessages = useChatStore(queuedMessagesSelector);
  const isAgentRunningSelector = useMemo(() => selectIsAgentRunning(storeContextKey), [storeContextKey]);
  const isAgentRunning = useChatStore(isAgentRunningSelector);

  // For execution mode, fetch execution conversations directly
  // For regular chat, use the standard useChat hook
  const regularChatData = useChat(context);

  // Fetch execution conversations when in execution mode
  const executionConversationsQuery = useQuery({
    queryKey: chatKeys.conversationList("task_execution", taskId),
    queryFn: () => chatApi.listConversations("task_execution", taskId),
    enabled: isExecutionMode,
  });

  // Use execution conversations when in execution mode, otherwise regular conversations
  const conversations = isExecutionMode
    ? executionConversationsQuery
    : regularChatData.conversations;

  const {
    messages: activeConversation,
    sendMessage,
    switchConversation: handleSelectConversation,
    createConversation: handleNewConversation,
  } = regularChatData;

  // Streaming state - accumulates text chunks as they arrive
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  // Ref for activeConversationId so event listeners always have current value
  const activeConversationIdRef = useRef(activeConversationId);

  useEffect(() => {
    activeConversationIdRef.current = activeConversationId;
  }, [activeConversationId]);

  // Extract messages array from active conversation
  const messagesData = activeConversation.data?.messages ?? [];

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

  // Queue message handler (when agent is running)
  const handleQueue = useCallback(
    (content: string) => {
      if (!content.trim()) return;
      // Use execution queue if in execution mode
      if (isExecutionMode) {
        queueExecutionMessage(taskId, content);
      } else {
        queueMessage(storeContextKey, content);
      }
    },
    [isExecutionMode, taskId, queueMessage, queueExecutionMessage, storeContextKey]
  );

  // Edit last queued message
  const handleEditLastQueued = useCallback(() => {
    const messagesToUse = isExecutionMode ? executionQueuedMessages : queuedMessages;
    const lastMessage = messagesToUse[messagesToUse.length - 1];
    if (!lastMessage) return;
    startEditingQueuedMessage(storeContextKey, lastMessage.id);
  }, [isExecutionMode, executionQueuedMessages, queuedMessages, startEditingQueuedMessage, storeContextKey]);

  // Subscribe to Tauri events for real-time updates (only on mount)
  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    (async () => {
      // Listen for tool calls - invalidate cache to pick up new messages
      const toolCallUnlisten = await listen<{
        tool_name: string;
        arguments: unknown;
        result: unknown;
        conversation_id: string;
      }>("chat:tool_call", (event) => {
        const { conversation_id } = event.payload;
        console.log("Tool call received:", event.payload);
        // Invalidate cache to pick up any new messages from backend
        if (conversation_id === activeConversationIdRef.current) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
      });
      unlisteners.push(toolCallUnlisten);

      // Listen for chat run completion - refresh messages
      const runCompletedUnlisten = await listen<{
        conversation_id: string;
      }>("chat:run_completed", (event) => {
        const { conversation_id } = event.payload;
        console.log("Chat run completed:", event.payload);
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
      unlisteners.push(runCompletedUnlisten);

      // Execution-specific events
      // Listen for execution tool calls - invalidate cache
      const execToolCallUnlisten = await listen<{
        conversation_id: string;
        tool_name: string;
        arguments: unknown;
      }>("execution:tool_call", (event) => {
        const { conversation_id } = event.payload;
        console.log("Execution tool call received:", event.payload);
        // Invalidate cache to pick up any new messages from backend
        if (conversation_id === activeConversationIdRef.current) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
      });
      unlisteners.push(execToolCallUnlisten);

      // Listen for execution completion - refresh messages
      const execCompletedUnlisten = await listen<{
        conversation_id: string;
      }>("execution:run_completed", (event) => {
        const { conversation_id } = event.payload;
        console.log("Worker execution completed:", event.payload);
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
      unlisteners.push(execCompletedUnlisten);
    })();

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [queryClient]);

  // Sort messages by createdAt - render in chronological order, no grouping
  const sortedMessages = useMemo(() => {
    return [...messagesData].sort((a, b) =>
      new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime()
    );
  }, [messagesData]);

  const isLoading = activeConversation.isLoading;
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
          <ContextIndicator isExecutionMode={isExecutionMode} />

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
              contextType={isExecutionMode ? "task_execution" : "task"}
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
            {/* Show worker executing indicator when in execution mode */}
            {isExecutionMode && <WorkerExecutingIndicator />}

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
          {/* Queued Messages - use execution queue in execution mode */}
          {(() => {
            const messagesToDisplay = isExecutionMode ? executionQueuedMessages : queuedMessages;
            const deleteHandler = isExecutionMode
              ? (id: string) => deleteExecutionQueuedMessage(taskId, id)
              : (id: string) => deleteQueuedMessage(storeContextKey, id);
            // Edit handler always needs context key
            const editHandler = (id: string, content: string) => editQueuedMessage(storeContextKey, id, content);

            return messagesToDisplay.length > 0 && (
              <div className="p-3 pb-0">
                <QueuedMessageList
                  messages={messagesToDisplay}
                  onEdit={editHandler}
                  onDelete={deleteHandler}
                />
              </div>
            );
          })()}

          {/* Chat Input */}
          <div className="p-3">
            <ChatInput
              onSend={handleSend}
              onQueue={handleQueue}
              isAgentRunning={isExecutionMode || isAgentRunning}
              isSending={isSending}
              hasQueuedMessages={(isExecutionMode ? executionQueuedMessages : queuedMessages).length > 0}
              onEditLastQueued={handleEditLastQueued}
              placeholder={
                isExecutionMode
                  ? "Message worker... (will be sent when current response completes)"
                  : "Send a message..."
              }
              showHelperText={(isExecutionMode ? executionQueuedMessages : queuedMessages).length > 0}
              autoFocus
            />
          </div>
        </div>
      </div>
    </>
  );
}
