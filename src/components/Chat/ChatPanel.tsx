/**
 * ChatPanel - Premium resizable side panel for context-aware chat
 *
 * Design spec: specs/design/pages/chat-panel.md
 * - Refined Studio aesthetic with layered depth
 * - Glass effect header with backdrop-blur
 * - Asymmetric message bubbles (user: warm orange, agent: dark gradient)
 * - Compact sizing for application UI
 * - Slide-in/out animations
 * - Collapsible to thin bar with unread indicator
 */

import { useState, useRef, useEffect, useCallback, useMemo } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useChat, chatKeys } from "@/hooks/useChat";
import { useChatStore, selectQueuedMessages, selectIsAgentRunning, selectActiveConversationId, selectExecutionQueuedMessages, getContextKey } from "@/stores/chatStore";
import type { ChatContext } from "@/types/chat";
import { useTaskStore } from "@/stores/taskStore";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { chatApi, stopAgent } from "@/api/chat";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  MessageSquare,
  CheckSquare,
  FolderKanban,
  X,
  PanelRightClose,
  PanelRightOpen,
  Loader2,
  Hammer,
} from "lucide-react";
import { ConversationSelector } from "./ConversationSelector";
import { QueuedMessageList } from "./QueuedMessageList";
import { ChatInput } from "./ChatInput";
import { type ToolCall } from "./ToolCallIndicator";
import { ChatMessages } from "./ChatMessages";
import { ResizeablePanel, useResizePanel } from "./ResizeablePanel";

// ============================================================================
// Constants
// ============================================================================

const COLLAPSED_WIDTH = 40;

// ============================================================================
// CSS Animations (defined as style tag content)
// ============================================================================

const animationStyles = `
@keyframes slideInRight {
  from { transform: translateX(100%); opacity: 0.5; }
  to { transform: translateX(0); opacity: 1; }
}

@keyframes slideOutRight {
  from { transform: translateX(0); opacity: 1; }
  to { transform: translateX(100%); opacity: 0.5; }
}

@keyframes typingBounce {
  0%, 60%, 100% { transform: translateY(0); }
  30% { transform: translateY(-4px); }
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.7; transform: scale(1.1); }
}

.chat-panel-enter {
  animation: slideInRight 250ms ease-out forwards;
}

.chat-panel-exit {
  animation: slideOutRight 200ms ease-in forwards;
}

.typing-dot {
  animation: typingBounce 1.4s ease-in-out infinite;
}

.typing-dot:nth-child(2) { animation-delay: 0.15s; }
.typing-dot:nth-child(3) { animation-delay: 0.3s; }

.unread-dot {
  animation: pulse 2s ease-in-out infinite;
}
`;

// ============================================================================
// Sub-components
// ============================================================================

interface ContextIndicatorProps {
  context: ChatContext;
  isExecutionMode?: boolean;
}

function ContextIndicator({ context, isExecutionMode = false }: ContextIndicatorProps) {
  const getContextInfo = () => {
    if (isExecutionMode) {
      return { icon: Hammer, label: "Worker Execution" };
    }

    switch (context.view) {
      case "ideation":
        return { icon: MessageSquare, label: "Chat" };
      case "kanban":
        return context.selectedTaskId
          ? { icon: CheckSquare, label: "Task" }
          : { icon: FolderKanban, label: "Project" };
      case "task_detail":
        return { icon: CheckSquare, label: "Task" };
      case "activity":
        return { icon: MessageSquare, label: "Activity" };
      case "settings":
        return { icon: MessageSquare, label: "Settings" };
      default:
        return { icon: MessageSquare, label: "Chat" };
    }
  };

  const { icon: Icon, label } = getContextInfo();

  return (
    <div className="flex items-center gap-2 min-w-0 flex-1">
      <Icon className="w-3.5 h-3.5 shrink-0 text-white/50" />
      <span className="text-[13px] font-medium truncate text-white/80">{label}</span>
    </div>
  );
}

// ============================================================================
// Collapsed Panel
// ============================================================================

interface CollapsedPanelProps {
  onExpand: () => void;
  hasUnread: boolean;
}

function CollapsedPanel({ onExpand, hasUnread }: CollapsedPanelProps) {
  return (
    <div
      data-testid="chat-panel-collapsed"
      className="fixed top-14 right-0 bottom-0 flex flex-col items-center justify-center"
      style={{
        width: `${COLLAPSED_WIDTH}px`,
        backgroundColor: "var(--bg-surface)",
        borderLeft: "1px solid var(--border-subtle)",
      }}
    >
      {hasUnread && (
        <div
          className="unread-dot absolute top-4 w-2 h-2 rounded-full"
          style={{ backgroundColor: "var(--accent-primary)" }}
        />
      )}
      <Button
        variant="ghost"
        size="icon-sm"
        onClick={onExpand}
        aria-label="Expand chat panel"
      >
        <PanelRightOpen className="w-[18px] h-[18px]" />
      </Button>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

interface ChatPanelProps {
  context: ChatContext;
}

// Wrapper component that checks isOpen before rendering the full panel
// This prevents expensive hooks from running when the panel is closed
export function ChatPanel({ context }: ChatPanelProps) {
  const isOpen = useChatStore((s) => s.isOpen);

  if (!isOpen) {
    return null;
  }

  return <ChatPanelContent context={context} />;
}

function ChatPanelContent({ context }: ChatPanelProps) {
  const queryClient = useQueryClient();
  const chatStore = useChatStore();
  const {
    width,
    togglePanel,
    setWidth,
    queueMessage,
    deleteQueuedMessage,
    startEditingQueuedMessage,
    queueExecutionMessage,
    deleteExecutionQueuedMessage,
  } = chatStore;
  const activeConversationId = useChatStore(selectActiveConversationId);

  // Compute context key for queue/agent state operations
  const contextKey = useMemo(() => getContextKey(context), [context]);

  // Use context-aware selectors
  const queuedMessagesSelector = useMemo(() => selectQueuedMessages(contextKey), [contextKey]);
  const queuedMessages = useChatStore(queuedMessagesSelector);
  const isAgentRunningSelector = useMemo(() => selectIsAgentRunning(contextKey), [contextKey]);
  const isAgentRunning = useChatStore(isAgentRunningSelector);

  // Detect execution mode: if task is executing, switch to task_execution context
  const selectedTask = useTaskStore((state) =>
    context.selectedTaskId ? state.tasks[context.selectedTaskId] : undefined
  );
  const isExecutionMode = selectedTask?.internalStatus === "executing";

  // Memoize the selector to avoid creating new function references on each render
  const taskIdForQueue = context.selectedTaskId ?? "";
  const executionQueuedMessagesSelector = useMemo(
    () => selectExecutionQueuedMessages(taskIdForQueue),
    [taskIdForQueue]
  );
  const executionQueuedMessages = useChatStore(executionQueuedMessagesSelector);

  // For execution mode, fetch execution conversations directly using task_execution context
  // For regular chat, use the standard useChat hook
  const regularChatData = useChat(context);

  // Fetch execution conversations when in execution mode
  const executionConversationsQuery = useQuery({
    queryKey: chatKeys.conversationList("task_execution", context.selectedTaskId ?? ""),
    queryFn: () => chatApi.listConversations("task_execution", context.selectedTaskId ?? ""),
    enabled: isExecutionMode && !!context.selectedTaskId,
  });

  // Use execution conversations when in execution mode, otherwise regular conversations
  const conversations = isExecutionMode
    ? executionConversationsQuery
    : regularChatData.conversations;

  // Fetch agent run status for the active conversation to detect failed runs
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

  const [isCollapsed, setIsCollapsed] = useState(false);
  const [isExiting, setIsExiting] = useState(false);
  const [hasUnread, setHasUnread] = useState(false);
  // Streaming tool calls - accumulated during agent execution
  const [streamingToolCalls, setStreamingToolCalls] = useState<ToolCall[]>([]);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const lastMessageCountRef = useRef(0);
  // Ref for activeConversationId so event listeners always have current value
  const activeConversationIdRef = useRef(activeConversationId);

  // Resize panel hook
  const { ResizeHandle } = useResizePanel({
    initialWidth: width,
    onWidthChange: setWidth,
  });

  useEffect(() => {
    activeConversationIdRef.current = activeConversationId;
  }, [activeConversationId]);

  // Extract messages array from active conversation (memoized to avoid dependency chain issues)
  const messagesData = useMemo(
    () => activeConversation.data?.messages ?? [],
    [activeConversation.data?.messages]
  );

  // Track unread messages when collapsed
  useEffect(() => {
    const messageCount = messagesData.length;
    if (isCollapsed && messageCount > lastMessageCountRef.current) {
      setHasUnread(true);
    }
    lastMessageCountRef.current = messageCount;
  }, [messagesData.length, isCollapsed]);

  // Clear unread when expanded
  useEffect(() => {
    if (!isCollapsed) {
      setHasUnread(false);
    }
  }, [isCollapsed]);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    if (messagesEndRef.current && messagesData.length) {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messagesData.length]);

  // Close with animation
  const handleClose = useCallback(() => {
    setIsExiting(true);
    setTimeout(() => {
      togglePanel();
      setIsExiting(false);
    }, 200);
  }, [togglePanel]);

  // Stop the running agent
  const handleStopAgent = useCallback(async () => {
    const ctxType = isExecutionMode
      ? "task_execution"
      : context.view === "ideation"
        ? "ideation"
        : context.view === "task_detail"
          ? "task"
          : "project";
    const ctxId = context.view === "ideation" && context.ideationSessionId
      ? context.ideationSessionId
      : context.selectedTaskId || context.projectId;

    try {
      await stopAgent(ctxType, ctxId);
      // Clear streaming tool calls when agent is stopped
      setStreamingToolCalls([]);
    } catch (error) {
      console.error("Failed to stop agent:", error);
    }
  }, [isExecutionMode, context]);

  // Escape to close panel (only runs when panel is open via wrapper)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !isCollapsed) {
        handleClose();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isCollapsed, handleClose]);

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

  // Get current context type and ID for queue operations
  const getQueueContext = useCallback(() => {
    const ctxType = isExecutionMode
      ? "task_execution"
      : context.view === "ideation"
        ? "ideation"
        : context.view === "task_detail"
          ? "task"
          : "project";
    const ctxId = context.view === "ideation" && context.ideationSessionId
      ? context.ideationSessionId
      : context.selectedTaskId || context.projectId;
    return { ctxType, ctxId } as const;
  }, [isExecutionMode, context]);

  // Generate a unique ID for queued messages
  const generateQueuedMessageId = useCallback(() => {
    return `queued-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
  }, []);

  // Queue message handler (when agent is running)
  // Uses backend queue API for ALL contexts so messages are properly processed
  const handleQueue = useCallback(
    async (content: string) => {
      if (!content.trim()) return;

      const { ctxType, ctxId } = getQueueContext();

      // Generate ID FIRST - this ID will be used by both frontend and backend
      const messageId = generateQueuedMessageId();

      // Add to local store immediately for optimistic UI (using the same ID)
      if (isExecutionMode && context.selectedTaskId) {
        queueExecutionMessage(context.selectedTaskId, content, messageId);
      } else {
        queueMessage(contextKey, content, messageId);
      }

      try {
        // Queue via backend API with the same ID
        await chatApi.queueAgentMessage(ctxType, ctxId, content, messageId);
        console.debug(`[queue] Queued message ${messageId} for ${ctxType}/${ctxId}`);
      } catch (error) {
        console.error("Failed to queue message to backend:", error);
        // Message is already in local store, which is fine - it just won't be processed by backend
        // User can delete and re-queue if needed
      }
    },
    [isExecutionMode, context.selectedTaskId, queueMessage, queueExecutionMessage, getQueueContext, generateQueuedMessageId, contextKey]
  );

  // Delete queued message handler - syncs with backend
  // Both frontend and backend use the same ID, so we can delete directly by ID
  const handleDeleteQueuedMessage = useCallback(
    async (messageId: string) => {
      const { ctxType, ctxId } = getQueueContext();

      // Delete from local store immediately (optimistic)
      if (isExecutionMode && context.selectedTaskId) {
        deleteExecutionQueuedMessage(context.selectedTaskId, messageId);
      } else {
        deleteQueuedMessage(contextKey, messageId);
      }

      // Delete from backend using the same ID
      try {
        await chatApi.deleteQueuedAgentMessage(ctxType, ctxId, messageId);
        console.debug(`[queue] Deleted message ${messageId} from backend`);
      } catch (error) {
        console.error("Failed to delete queued message from backend:", error);
        // Message already removed from local store, which is fine
      }
    },
    [isExecutionMode, context.selectedTaskId, deleteQueuedMessage, deleteExecutionQueuedMessage, getQueueContext, contextKey]
  );

  // Edit queued message handler - delete old and queue new
  // Both frontend and backend use the same ID, so we can operate directly by ID
  const handleEditQueuedMessage = useCallback(
    async (messageId: string, newContent: string) => {
      const { ctxType, ctxId } = getQueueContext();

      // Delete old message from backend
      try {
        await chatApi.deleteQueuedAgentMessage(ctxType, ctxId, messageId);
      } catch (error) {
        console.error("Failed to delete old queued message:", error);
      }

      // Delete from local store
      if (isExecutionMode && context.selectedTaskId) {
        deleteExecutionQueuedMessage(context.selectedTaskId, messageId);
      } else {
        deleteQueuedMessage(contextKey, messageId);
      }

      // Generate new ID and queue the edited content
      const newMessageId = generateQueuedMessageId();

      // Add to local store first (optimistic)
      if (isExecutionMode && context.selectedTaskId) {
        queueExecutionMessage(context.selectedTaskId, newContent, newMessageId);
      } else {
        queueMessage(contextKey, newContent, newMessageId);
      }

      // Queue to backend with same ID
      try {
        await chatApi.queueAgentMessage(ctxType, ctxId, newContent, newMessageId);
        console.debug(`[queue] Edited message: old=${messageId}, new=${newMessageId}`);
      } catch (error) {
        console.error("Failed to queue edited message to backend:", error);
        // Message is already in local store
      }
    },
    [isExecutionMode, context.selectedTaskId, deleteQueuedMessage, deleteExecutionQueuedMessage, queueMessage, queueExecutionMessage, getQueueContext, generateQueuedMessageId, contextKey]
  );

  // Edit last queued message
  const handleEditLastQueued = useCallback(() => {
    const messagesToUse = isExecutionMode ? executionQueuedMessages : queuedMessages;
    const lastMessage = messagesToUse[messagesToUse.length - 1];
    if (!lastMessage) return;
    startEditingQueuedMessage(contextKey, lastMessage.id);
  }, [isExecutionMode, executionQueuedMessages, queuedMessages, startEditingQueuedMessage, contextKey]);

  // Subscribe to Tauri events for real-time updates (only on mount)
  // Using unified agent:* events (Phase 5-6 consolidation)
  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    (async () => {
      // Listen for tool calls - accumulate for streaming display and invalidate cache
      // Unified event: agent:tool_call (replaces chat:tool_call and execution:tool_call)
      const toolCallUnlisten = await listen<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        tool_name: string;
        arguments: unknown;
        result: unknown;
      }>("agent:tool_call", (event) => {
        const { tool_name, arguments: args, result, conversation_id, context_type } = event.payload;
        // Only show for active conversation
        if (conversation_id === activeConversationIdRef.current) {
          setStreamingToolCalls((prev) => [
            ...prev,
            {
              id: `streaming-${Date.now()}-${prev.length}`,
              name: tool_name,
              arguments: args,
              result,
            },
          ]);
          // Invalidate cache to pick up any new messages from backend
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
        // Log for debugging
        console.debug(`[agent:tool_call] context=${context_type}, tool=${tool_name}`);
      });
      unlisteners.push(toolCallUnlisten);

      // Listen for run completion - clear streaming state and refresh
      // Unified event: agent:run_completed (replaces chat:run_completed and execution:run_completed)
      const runCompletedUnlisten = await listen<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        status: string;
      }>("agent:run_completed", (event) => {
        const { conversation_id } = event.payload;
        // Clear streaming tool calls
        setStreamingToolCalls([]);
        // Invalidate cache to get final messages
        if (conversation_id) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
        // Force scroll to bottom after completion
        setTimeout(() => {
          if (messagesEndRef.current) {
            messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
          }
        }, 100);
      });
      unlisteners.push(runCompletedUnlisten);

      // Listen for agent errors - clear streaming state
      // Unified event: agent:error
      const errorUnlisten = await listen<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        error: string;
      }>("agent:error", (event) => {
        const { conversation_id, error, context_type } = event.payload;
        console.error(`Agent error: context=${context_type}, conversation=${conversation_id}:`, error);
        // Clear streaming tool calls on error
        setStreamingToolCalls([]);
        // Invalidate cache
        if (conversation_id) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
      });
      unlisteners.push(errorUnlisten);

      // Listen for run started - for progress tracking
      // Unified event: agent:run_started
      const runStartedUnlisten = await listen<{
        context_type: string;
        context_id: string;
        conversation_id: string;
        agent_run_id: string;
      }>("agent:run_started", (event) => {
        const { conversation_id, context_type, agent_run_id } = event.payload;
        console.debug(`[agent:run_started] context=${context_type}, conversation=${conversation_id}, run=${agent_run_id}`);
        // Invalidate agent run status to pick up new run
        if (conversation_id) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.agentRun(conversation_id),
          });
        }
      });
      unlisteners.push(runStartedUnlisten);

      // Listen for queue_sent - backend notifies when it sends a queued message
      // This updates the optimistic UI for execution queued messages
      // Since frontend and backend use the same ID, we can match exactly by ID
      const queueSentUnlisten = await listen<{
        message_id: string;
        conversation_id: string;
        context_type: string;
        context_id: string;
      }>("agent:queue_sent", (event) => {
        const { message_id, context_type, context_id } = event.payload;
        console.debug(`[agent:queue_sent] message=${message_id}, context=${context_type}/${context_id}`);

        // For task_execution context, remove from execution queue by exact ID
        if (context_type === "task_execution") {
          useChatStore.getState().deleteExecutionQueuedMessage(context_id, message_id);
        } else {
          // For other contexts, build context key and remove from queue
          const eventContextKey = context_type === "ideation"
            ? `session:${context_id}`
            : context_type === "task"
              ? `task:${context_id}`
              : `project:${context_id}`;
          useChatStore.getState().deleteQueuedMessage(eventContextKey, message_id);
        }
      });
      unlisteners.push(queueSentUnlisten);
    })();

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [queryClient]);

  if (isCollapsed) {
    return (
      <CollapsedPanel
        onExpand={() => setIsCollapsed(false)}
        hasUnread={hasUnread}
      />
    );
  }

  const isLoading = activeConversation.isLoading;
  const isSending = sendMessage.isPending;

  return (
    <>
      <style>{animationStyles}</style>
      <ResizeablePanel
        width={width}
        isExiting={isExiting}
        testId="chat-panel"
        ariaLabel="Chat panel"
        ResizeHandle={ResizeHandle}
      >
        {/* Header - Glass effect */}
        <div
          data-testid="chat-panel-header"
          className="flex items-center justify-between h-11 px-3 border-b backdrop-blur-sm"
          style={{
            borderColor: "rgba(255,255,255,0.06)",
            background: "linear-gradient(180deg, rgba(26,26,26,0.95) 0%, rgba(20,20,20,0.98) 100%)",
          }}
        >
          <ContextIndicator context={context} isExecutionMode={isExecutionMode} />

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
                isExecutionMode
                  ? "task_execution"
                  : context.view === "ideation"
                    ? "ideation"
                    : context.view === "task_detail"
                      ? "task"
                      : "project"
              }
              contextId={
                context.view === "ideation" && context.ideationSessionId
                  ? context.ideationSessionId
                  : context.selectedTaskId || context.projectId
              }
              conversations={conversations.data ?? []}
              activeConversationId={activeConversationId}
              onSelectConversation={handleSelectConversation}
              onNewConversation={handleNewConversation}
              isLoading={conversations.isLoading}
            />
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => setIsCollapsed(true)}
              aria-label="Collapse chat panel"
            >
              <PanelRightClose className="w-[18px] h-[18px]" />
            </Button>
            <Button
              data-testid="chat-panel-close"
              variant="ghost"
              size="icon-sm"
              onClick={handleClose}
              aria-label="Close chat panel"
            >
              <X className="w-[18px] h-[18px]" />
            </Button>
          </div>
        </div>

        {/* Messages Area */}
        <ChatMessages
          messages={messagesData}
          isLoading={isLoading}
          isSending={isSending}
          isAgentRunning={isAgentRunning}
          isExecutionMode={isExecutionMode}
          streamingToolCalls={streamingToolCalls}
          failedErrorMessage={showFailedBanner && failedRun?.errorMessage ? failedRun.errorMessage : undefined}
          onDismissError={failedRun ? () => setDismissedErrorId(failedRun.id) : undefined}
          messagesEndRef={messagesEndRef}
          scrollAreaRef={scrollAreaRef}
        />

        {/* Input Area */}
        <div className="border-t" style={{ borderColor: "var(--border-subtle)" }}>
          {/* Queued Messages - use execution queue in execution mode */}
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
              onStop={handleStopAgent}
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
      </ResizeablePanel>
    </>
  );
}
