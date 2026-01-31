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

import { useState, useRef, useEffect, useMemo, useCallback } from "react";
import { useChat, chatKeys } from "@/hooks/useChat";
import { useChatStore, selectQueuedMessages, selectIsAgentRunning, selectActiveConversationId, getContextKey } from "@/stores/chatStore";
import { useUiStore } from "@/stores/uiStore";
import type { ChatContext } from "@/types/chat";
import { useTaskStore } from "@/stores/taskStore";
import { useQuery } from "@tanstack/react-query";
import { chatApi } from "@/api/chat";
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
import { ChatMessages } from "./ChatMessages";
import { ResizeablePanel } from "./ResizeablePanel";
import { useResizePanel } from "./useResizePanel";
import { useChatPanelHandlers } from "@/hooks/useChatPanelHandlers";

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

// Wrapper component that checks visibility before rendering the full panel
// This prevents expensive hooks from running when the panel is closed
export function ChatPanel({ context }: ChatPanelProps) {
  const chatVisibleByView = useUiStore((s) => s.chatVisibleByView);
  const isVisible = chatVisibleByView[context.view];

  if (!isVisible) {
    return null;
  }

  return <ChatPanelContent context={context} />;
}

function ChatPanelContent({ context }: ChatPanelProps) {
  const width = useChatStore((s) => s.width);
  const setWidth = useChatStore((s) => s.setWidth);
  const toggleChatVisible = useUiStore((s) => s.toggleChatVisible);
  const activeConversationId = useChatStore(selectActiveConversationId);

  // Detect execution mode: if task is executing, switch to task_execution context
  const selectedTask = useTaskStore((state) =>
    context.selectedTaskId ? state.tasks[context.selectedTaskId] : undefined
  );
  const isExecutionMode = selectedTask?.internalStatus === "executing";

  // Compute context key for queue/agent state operations
  // Uses context-aware keys: "task_execution:id" for execution mode, otherwise standard keys
  const contextKey = useMemo(() => {
    if (isExecutionMode && context.selectedTaskId) {
      return `task_execution:${context.selectedTaskId}`;
    }
    return getContextKey(context);
  }, [isExecutionMode, context]);

  // Use context-aware selectors - unified queue works for all modes
  const queuedMessagesSelector = useMemo(() => selectQueuedMessages(contextKey), [contextKey]);
  const queuedMessages = useChatStore(queuedMessagesSelector);
  const isAgentRunningSelector = useMemo(() => selectIsAgentRunning(contextKey), [contextKey]);
  const isAgentRunning = useChatStore(isAgentRunningSelector);

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
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const lastMessageCountRef = useRef(0);

  // Resize panel hook
  const { ResizeHandle } = useResizePanel({
    initialWidth: width,
    onWidthChange: setWidth,
  });

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

  // Use extracted handlers hook - unified queue with context-aware keys
  const {
    streamingToolCalls,
    handleSend,
    handleQueue,
    handleStopAgent,
    handleDeleteQueuedMessage,
    handleEditQueuedMessage,
    handleEditLastQueued,
  } = useChatPanelHandlers({
    context,
    isExecutionMode,
    contextKey,
    activeConversationId,
    sendMessage,
    queuedMessages,
    messagesEndRef,
  });

  // Close with animation
  const handleClose = useCallback(() => {
    setIsExiting(true);
    setTimeout(() => {
      toggleChatVisible(context.view);
      setIsExiting(false);
    }, 200);
  }, [toggleChatVisible, context.view]);

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
        />

        {/* Input Area */}
        <div className="border-t" style={{ borderColor: "var(--border-subtle)" }}>
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
              onStop={handleStopAgent}
              isAgentRunning={isExecutionMode || isAgentRunning}
              isSending={isSending}
              hasQueuedMessages={queuedMessages.length > 0}
              onEditLastQueued={handleEditLastQueued}
              placeholder={
                isExecutionMode
                  ? "Message worker... (will be sent when current response completes)"
                  : "Send a message..."
              }
              showHelperText={queuedMessages.length > 0}
              autoFocus
            />
          </div>
        </div>
      </ResizeablePanel>
    </>
  );
}
