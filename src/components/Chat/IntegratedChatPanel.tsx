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

import { useState, useRef, useEffect, useCallback, useMemo } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useChat, chatKeys } from "@/hooks/useChat";
import { useChatStore, selectQueuedMessages, selectIsAgentRunning, selectActiveConversationId, selectExecutionQueuedMessages, getContextKey } from "@/stores/chatStore";
import { useUiStore } from "@/stores/uiStore";
import { useTaskStore } from "@/stores/taskStore";
import type { ChatContext } from "@/types/chat";
import type { ContextType } from "@/types/chat-conversation";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { chatApi, stopAgent } from "@/api/chat";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  MessageSquare,
  CheckSquare,
  FolderKanban,
  Bot,
  PanelRightClose,
  PanelRightOpen,
  Loader2,
  Hammer,
  Activity,
  X,
} from "lucide-react";
import { ConversationSelector } from "./ConversationSelector";
import { QueuedMessageList } from "./QueuedMessageList";
import { ChatInput } from "./ChatInput";
import { type ToolCall } from "./ToolCallIndicator";
import { StreamingToolIndicator } from "./StreamingToolIndicator";
import { MessageItem } from "./MessageItem";

// ============================================================================
// Constants
// ============================================================================

const COLLAPSED_WIDTH = 48;

// ============================================================================
// CSS Animations
// ============================================================================

const animationStyles = `
@keyframes typingBounce {
  0%, 60%, 100% { transform: translateY(0); }
  30% { transform: translateY(-4px); }
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.7; transform: scale(1.1); }
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

function TypingIndicator() {
  return (
    <div
      data-testid="chat-typing-indicator"
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
      data-testid="chat-panel-empty"
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
        Ask questions or get help with your tasks
      </p>
    </div>
  );
}

function LoadingState() {
  return (
    <div
      data-testid="chat-panel-loading"
      className="flex items-center justify-center p-6"
    >
      <Loader2 className="w-5 h-5 animate-spin text-[#ff6b35]" />
    </div>
  );
}

function WorkerExecutingIndicator() {
  const setCurrentView = useUiStore((s) => s.setCurrentView);

  return (
    <div
      data-testid="worker-executing-indicator"
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
      <Button
        variant="ghost"
        size="sm"
        onClick={() => setCurrentView("activity")}
        className="shrink-0 h-7 px-2"
        aria-label="View all activity"
      >
        <Activity className="w-3.5 h-3.5 mr-1" />
        <span className="text-[11px]">Activity</span>
      </Button>
    </div>
  );
}

interface FailedRunBannerProps {
  errorMessage: string;
  onDismiss?: () => void;
}

function FailedRunBanner({ errorMessage, onDismiss }: FailedRunBannerProps) {
  return (
    <div
      data-testid="failed-run-banner"
      className="flex items-start gap-2 px-3 py-2 mb-2 rounded-lg"
      style={{
        background: "linear-gradient(135deg, rgba(239,68,68,0.12) 0%, rgba(239,68,68,0.05) 100%)",
        border: "1px solid rgba(239,68,68,0.25)",
      }}
    >
      <Activity className="w-3.5 h-3.5 mt-0.5 text-red-400 shrink-0" />
      <div className="flex-1 min-w-0">
        <span className="text-[13px] font-medium text-red-300 block">
          Agent run failed
        </span>
        <span className="text-[12px] text-red-300/70 block mt-0.5 break-words">
          {errorMessage.slice(0, 200)}
          {errorMessage.length > 200 && "..."}
        </span>
      </div>
      {onDismiss && (
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={onDismiss}
          className="shrink-0 text-red-300/60 hover:text-red-300"
          aria-label="Dismiss error"
        >
          <X className="w-3.5 h-3.5" />
        </Button>
      )}
    </div>
  );
}

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

// MessageItem is now imported from "./MessageItem" - shared component

// ============================================================================
// Collapsed Panel (Thin bar with expand button)
// ============================================================================

interface CollapsedPanelProps {
  onExpand: () => void;
  hasUnread: boolean;
}

function CollapsedPanel({ onExpand, hasUnread }: CollapsedPanelProps) {
  return (
    <div
      data-testid="integrated-chat-collapsed"
      className="relative h-full flex flex-col items-center justify-center border-l"
      style={{
        width: `${COLLAPSED_WIDTH}px`,
        backgroundColor: "var(--bg-surface)",
        borderColor: "var(--border-subtle)",
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
        className="hover:bg-white/5"
      >
        <PanelRightOpen className="w-[18px] h-[18px]" />
      </Button>
      <span
        className="text-[10px] mt-2 rotate-90 whitespace-nowrap"
        style={{ color: "var(--text-muted)" }}
      >
        Chat
      </span>
    </div>
  );
}

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

  const {
    queueMessage,
    deleteQueuedMessage,
    startEditingQueuedMessage,
    queueExecutionMessage,
    deleteExecutionQueuedMessage,
  } = useChatStore();
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
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const lastMessageCountRef = useRef(0);
  const activeConversationIdRef = useRef(activeConversationId);

  useEffect(() => {
    activeConversationIdRef.current = activeConversationId;
  }, [activeConversationId]);

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

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    if (messagesEndRef.current && messagesData.length) {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messagesData.length]);

  // Scroll to bottom instantly when panel expands
  useEffect(() => {
    if (!chatCollapsed && messagesEndRef.current && messagesData.length) {
      messagesEndRef.current.scrollIntoView({ behavior: "instant" });
    }
  }, [chatCollapsed, messagesData.length]);

  // Auto-scroll during streaming (tool calls and agent running)
  // Use requestAnimationFrame to debounce rapid updates
  const scrollRAFRef = useRef<number | null>(null);
  useEffect(() => {
    if (isAgentRunning && messagesEndRef.current) {
      // Cancel any pending scroll
      if (scrollRAFRef.current) {
        cancelAnimationFrame(scrollRAFRef.current);
      }
      // Schedule scroll on next frame
      scrollRAFRef.current = requestAnimationFrame(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
        scrollRAFRef.current = null;
      });
    }
    return () => {
      if (scrollRAFRef.current) {
        cancelAnimationFrame(scrollRAFRef.current);
      }
    };
  }, [isAgentRunning, streamingToolCalls.length]);

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
      : ideationSessionId
        ? "ideation"
        : selectedTaskId
          ? "task"
          : "project";
    const ctxId = ideationSessionId || selectedTaskId || projectId;
    return { ctxType, ctxId } as const;
  }, [isExecutionMode, ideationSessionId, selectedTaskId, projectId]);

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
      if (isExecutionMode && selectedTaskId) {
        queueExecutionMessage(selectedTaskId, content, messageId);
      } else {
        queueMessage(storeContextKey, content, messageId);
      }

      // ALSO queue to backend so it gets processed when agent completes
      try {
        await chatApi.queueAgentMessage(ctxType, ctxId, content, messageId);
        console.debug(`[queue] Queued message ${messageId} for ${ctxType}/${ctxId}`);
      } catch (error) {
        console.error("Failed to queue message to backend:", error);
        // Message is already in local store, which is fine - it just won't be processed by backend
      }
    },
    [isExecutionMode, selectedTaskId, queueMessage, queueExecutionMessage, storeContextKey, getQueueContext, generateQueuedMessageId]
  );

  // Edit last queued message
  const handleEditLastQueued = useCallback(() => {
    const messagesToUse = isExecutionMode ? executionQueuedMessages : queuedMessages;
    const lastMessage = messagesToUse[messagesToUse.length - 1];
    if (!lastMessage) return;
    startEditingQueuedMessage(storeContextKey, lastMessage.id);
  }, [isExecutionMode, executionQueuedMessages, queuedMessages, startEditingQueuedMessage, storeContextKey]);

  // Delete queued message handler - syncs with backend
  const handleDeleteQueuedMessage = useCallback(
    async (messageId: string) => {
      const { ctxType, ctxId } = getQueueContext();

      // Delete from local store immediately (optimistic)
      if (isExecutionMode && selectedTaskId) {
        deleteExecutionQueuedMessage(selectedTaskId, messageId);
      } else {
        deleteQueuedMessage(storeContextKey, messageId);
      }

      // Delete from backend using the same ID
      try {
        await chatApi.deleteQueuedAgentMessage(ctxType, ctxId, messageId);
        console.debug(`[queue] Deleted message ${messageId} from backend`);
      } catch (error) {
        console.error("Failed to delete queued message from backend:", error);
      }
    },
    [isExecutionMode, selectedTaskId, deleteQueuedMessage, deleteExecutionQueuedMessage, getQueueContext, storeContextKey]
  );

  // Edit queued message handler - delete old and queue new (syncs with backend)
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
      if (isExecutionMode && selectedTaskId) {
        deleteExecutionQueuedMessage(selectedTaskId, messageId);
      } else {
        deleteQueuedMessage(storeContextKey, messageId);
      }

      // Generate new ID and queue the edited content
      const newMessageId = generateQueuedMessageId();

      // Add to local store first (optimistic)
      if (isExecutionMode && selectedTaskId) {
        queueExecutionMessage(selectedTaskId, newContent, newMessageId);
      } else {
        queueMessage(storeContextKey, newContent, newMessageId);
      }

      // Queue to backend with same ID
      try {
        await chatApi.queueAgentMessage(ctxType, ctxId, newContent, newMessageId);
        console.debug(`[queue] Edited message, new ID ${newMessageId} for ${ctxType}/${ctxId}`);
      } catch (error) {
        console.error("Failed to queue edited message to backend:", error);
      }
    },
    [isExecutionMode, selectedTaskId, deleteQueuedMessage, deleteExecutionQueuedMessage, queueMessage, queueExecutionMessage, getQueueContext, generateQueuedMessageId, storeContextKey]
  );

  // Stop the running agent
  const handleStopAgent = useCallback(async () => {
    const ctxType = isExecutionMode
      ? "task_execution"
      : ideationSessionId
        ? "ideation"
        : selectedTaskId
          ? "task"
          : "project";
    const ctxId = ideationSessionId || selectedTaskId || projectId;

    try {
      await stopAgent(ctxType, ctxId);
      // Clear streaming tool calls when agent is stopped
      setStreamingToolCalls([]);
    } catch (error) {
      console.error("Failed to stop agent:", error);
    }
  }, [isExecutionMode, ideationSessionId, selectedTaskId, projectId]);

  // Subscribe to Tauri events for real-time updates
  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    (async () => {
      // Listen for tool calls - accumulate for streaming display and invalidate cache
      const toolCallUnlisten = await listen<{
        tool_name: string;
        arguments: unknown;
        result: unknown;
        conversation_id: string;
      }>("chat:tool_call", (event) => {
        const { tool_name, arguments: args, result, conversation_id } = event.payload;
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
      });
      unlisteners.push(toolCallUnlisten);

      // Listen for chat run completion - clear streaming state and refresh
      const runCompletedUnlisten = await listen<{
        conversation_id: string;
      }>("chat:run_completed", (event) => {
        console.log("Chat run completed:", event.payload);
        const { conversation_id } = event.payload;
        // Clear streaming tool calls
        setStreamingToolCalls([]);
        // Invalidate cache to get final messages
        if (conversation_id) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
        // Scroll to bottom after a short delay to let messages render
        setTimeout(() => {
          if (messagesEndRef.current) {
            messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
          }
        }, 100);
      });
      unlisteners.push(runCompletedUnlisten);

      // Execution-specific events
      const execToolCallUnlisten = await listen<{
        conversation_id: string;
        tool_name: string;
        arguments: unknown;
      }>("execution:tool_call", (event) => {
        const { tool_name, arguments: args, conversation_id } = event.payload;
        // Only show for active conversation
        if (conversation_id === activeConversationIdRef.current) {
          setStreamingToolCalls((prev) => [
            ...prev,
            {
              id: `streaming-exec-${Date.now()}-${prev.length}`,
              name: tool_name,
              arguments: args,
            },
          ]);
          // Invalidate cache to pick up any new messages from backend
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
      });
      unlisteners.push(execToolCallUnlisten);

      // Listen for execution completion - clear streaming state and refresh
      const execCompletedUnlisten = await listen<{
        conversation_id: string;
      }>("execution:run_completed", (event) => {
        console.log("Worker execution completed:", event.payload);
        const { conversation_id } = event.payload;
        // Clear streaming tool calls
        setStreamingToolCalls([]);
        // Invalidate cache to get final messages
        if (conversation_id) {
          queryClient.invalidateQueries({
            queryKey: chatKeys.conversation(conversation_id),
          });
        }
        // Scroll to bottom after a short delay to let messages render
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
              onStop={handleStopAgent}
              isAgentRunning={isExecutionMode || isAgentRunning}
              isSending={isSending}
              hasQueuedMessages={(isExecutionMode ? executionQueuedMessages : queuedMessages).length > 0}
              onEditLastQueued={handleEditLastQueued}
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
