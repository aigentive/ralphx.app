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
import { useUiStore } from "@/stores/uiStore";
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
  X,
  PanelRightClose,
  PanelRightOpen,
  Loader2,
  Hammer,
  Activity,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { ConversationSelector } from "./ConversationSelector";
import { QueuedMessageList } from "./QueuedMessageList";
import { ChatInput } from "./ChatInput";
import { type ToolCall } from "./ToolCallIndicator";
import { StreamingToolIndicator } from "./StreamingToolIndicator";
import { MessageItem } from "./MessageItem";

// ============================================================================
// Constants
// ============================================================================

const MIN_WIDTH = 320;
const MAX_WIDTH_PERCENT = 50;
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
// Resize Handle
// ============================================================================

interface ResizeHandleProps {
  isDragging: boolean;
  onMouseDown: (e: React.MouseEvent) => void;
}

function ResizeHandle({ isDragging, onMouseDown }: ResizeHandleProps) {
  return (
    <div
      data-testid="chat-panel-resize-handle"
      className="absolute top-0 bottom-0 w-1.5 cursor-ew-resize z-[41]"
      style={{ left: "-3px" }}
      onMouseDown={onMouseDown}
    >
      <div
        className={cn(
          "absolute top-1/2 left-1/2 w-0.5 h-12 -translate-x-1/2 -translate-y-1/2 rounded-sm transition-all duration-150",
          isDragging ? "h-16" : ""
        )}
        style={{
          backgroundColor: isDragging
            ? "var(--accent-primary)"
            : "transparent",
          boxShadow: isDragging
            ? "0 0 8px rgba(255,107,53,0.4)"
            : "none",
        }}
      />
      <style>{`
        [data-testid="chat-panel-resize-handle"]:hover > div {
          background-color: var(--border-default);
        }
      `}</style>
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
  const {
    width,
    togglePanel,
    setWidth,
    queueMessage,
    deleteQueuedMessage,
    startEditingQueuedMessage,
    queueExecutionMessage,
    deleteExecutionQueuedMessage,
  } = useChatStore();
  const activeConversationId = useChatStore(selectActiveConversationId);

  // Compute context key for queue/agent state operations
  const contextKey = useMemo(() => getContextKey(context), [context]);

  // Use context-aware selectors
  const queuedMessagesSelector = useMemo(() => selectQueuedMessages(contextKey), [contextKey]);
  const queuedMessages = useChatStore(queuedMessagesSelector);
  const isAgentRunningSelector = useMemo(() => selectIsAgentRunning(contextKey), [contextKey]);
  const isAgentRunning = useChatStore(isAgentRunningSelector);

  // Debug: log context key and agent running state changes
  useEffect(() => {
    console.log(`[ChatPanel] contextKey=${contextKey}, isAgentRunning=${isAgentRunning}`);
  }, [contextKey, isAgentRunning]);

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
  const [isDragging, setIsDragging] = useState(false);
  const [isExiting, setIsExiting] = useState(false);
  const [hasUnread, setHasUnread] = useState(false);
  // Streaming tool calls - accumulated during agent execution
  const [streamingToolCalls, setStreamingToolCalls] = useState<ToolCall[]>([]);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const resizeRef = useRef<{ startX: number; startWidth: number } | null>(null);
  const lastMessageCountRef = useRef(0);
  // Ref for activeConversationId so event listeners always have current value
  const activeConversationIdRef = useRef(activeConversationId);

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

  // Resize handlers
  const handleResizeStart = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      setIsDragging(true);
      resizeRef.current = {
        startX: e.clientX,
        startWidth: width,
      };

      const handleResizeMove = (moveEvent: MouseEvent) => {
        if (!resizeRef.current) return;
        const deltaX = resizeRef.current.startX - moveEvent.clientX;
        const newWidth = resizeRef.current.startWidth + deltaX;
        const maxWidth = window.innerWidth * (MAX_WIDTH_PERCENT / 100);
        setWidth(Math.max(MIN_WIDTH, Math.min(maxWidth, newWidth)));
      };

      const handleResizeEnd = () => {
        resizeRef.current = null;
        setIsDragging(false);
        document.removeEventListener("mousemove", handleResizeMove);
        document.removeEventListener("mouseup", handleResizeEnd);
      };

      document.addEventListener("mousemove", handleResizeMove);
      document.addEventListener("mouseup", handleResizeEnd);
    },
    [width, setWidth]
  );

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
        const { conversation_id, context_type } = event.payload;
        console.log(`Agent run completed: context=${context_type}, conversation=${conversation_id}`);
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

  // Sort messages by createdAt - render in chronological order, no grouping
  const sortedMessages = useMemo(() => {
    return [...messagesData].sort((a, b) =>
      new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime()
    );
  }, [messagesData]);

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
  const isEmpty = !isLoading && sortedMessages.length === 0;

  return (
    <>
      <style>{animationStyles}</style>
      <aside
        ref={panelRef}
        data-testid="chat-panel"
        role="complementary"
        aria-label="Chat panel"
        className={cn(
          "fixed top-14 right-0 bottom-0 flex flex-col",
          isExiting ? "chat-panel-exit" : "chat-panel-enter"
        )}
        style={{
          width: `${width}px`,
          minWidth: `${MIN_WIDTH}px`,
          backgroundColor: "var(--bg-surface)",
          borderLeft: "1px solid var(--border-subtle)",
          boxShadow: "var(--shadow-md)",
          zIndex: 40,
        }}
      >
        {/* Resize Handle */}
        <ResizeHandle isDragging={isDragging} onMouseDown={handleResizeStart} />

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
        <ScrollArea
          ref={scrollAreaRef}
          className="flex-1"
          data-testid="chat-panel-messages"
        >
          <div className="p-3">
            {/* Show failed run banner if last run failed */}
            {showFailedBanner && failedRun?.errorMessage && (
              <FailedRunBanner
                errorMessage={failedRun.errorMessage}
                onDismiss={() => setDismissedErrorId(failedRun.id)}
              />
            )}

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
                {/* Show streaming tool calls or typing indicator while agent is working */}
                {(isSending || isAgentRunning) && (
                  streamingToolCalls.length > 0 ? (
                    <StreamingToolIndicator toolCalls={streamingToolCalls} isActive={true} />
                  ) : (
                    <TypingIndicator />
                  )
                )}
                <div ref={messagesEndRef} />
              </>
            )}
          </div>
        </ScrollArea>

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
      </aside>
    </>
  );
}
