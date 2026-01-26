/**
 * TaskChatPanel - Embedded chat panel for TaskFullView
 *
 * Reuses ChatPanel internals but without resize/collapse functionality.
 * Shows context-aware chat based on task state (execution/review/discussion).
 */

import { useState, useRef, useEffect, useCallback, useMemo } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useChat, chatKeys } from "@/hooks/useChat";
import { useChatStore, selectQueuedMessages, selectIsAgentRunning, selectActiveConversationId, selectExecutionQueuedMessages } from "@/stores/chatStore";
import type { ChatContext } from "@/types/chat";
import { useQuery } from "@tanstack/react-query";
import { chatApi } from "@/api/chat";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  MessageSquare,
  CheckSquare,
  Bot,
  Loader2,
  Copy,
  Check,
  Hammer,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import { cn } from "@/lib/utils";
import { ConversationSelector } from "../Chat/ConversationSelector";
import { QueuedMessageList } from "../Chat/QueuedMessageList";
import { ChatInput } from "../Chat/ChatInput";
import { ToolCallIndicator, type ToolCall } from "../Chat/ToolCallIndicator";

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

// ============================================================================
// Code Block with Copy Button
// ============================================================================

interface CodeBlockProps {
  children: string;
  language?: string | undefined;
}

function CodeBlock({ children, language }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(children);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Silently fail
    }
  }, [children]);

  return (
    <div className="relative group my-2">
      {language && (
        <span
          className="absolute top-1 left-3 text-[11px]"
          style={{ color: "var(--text-muted)" }}
        >
          {language}
        </span>
      )}
      <pre
        className="rounded-md overflow-hidden"
        style={{
          backgroundColor: "var(--bg-base)",
          border: "1px solid var(--border-subtle)",
        }}
      >
        <code
          className={cn(
            "block p-3 text-[13px] overflow-x-auto",
            language && "pt-6"
          )}
          style={{ fontFamily: "var(--font-mono)" }}
        >
          {children}
        </code>
      </pre>
      <Button
        variant="ghost"
        size="icon-sm"
        onClick={handleCopy}
        className="absolute top-1 right-1 opacity-0 group-hover:opacity-100 transition-opacity"
        aria-label={copied ? "Copied" : "Copy code"}
      >
        {copied ? (
          <Check className="w-4 h-4 text-[var(--status-success)]" />
        ) : (
          <Copy className="w-4 h-4" />
        )}
      </Button>
    </div>
  );
}

// ============================================================================
// Markdown Components
// ============================================================================

const markdownComponents = {
  a: ({
    href,
    children,
    ...props
  }: React.AnchorHTMLAttributes<HTMLAnchorElement>) => (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="underline hover:no-underline"
      style={{ color: "var(--accent-primary)" }}
      {...props}
    >
      {children}
    </a>
  ),
  code: ({
    className,
    children,
    ...props
  }: React.HTMLAttributes<HTMLElement>) => {
    const match = /language-(\w+)/.exec(className || "");
    const isBlock = Boolean(match);
    if (isBlock) {
      return (
        <CodeBlock language={match?.[1]}>{String(children).trim()}</CodeBlock>
      );
    }
    return (
      <code
        className="px-1 py-0.5 rounded text-[13px]"
        style={{
          backgroundColor: "var(--bg-base)",
          fontFamily: "var(--font-mono)",
        }}
        {...props}
      >
        {children}
      </code>
    );
  },
  pre: ({ children }: React.HTMLAttributes<HTMLPreElement>) => (
    <>{children}</>
  ),
  p: ({ children, ...props }: React.HTMLAttributes<HTMLParagraphElement>) => (
    <p className="mb-2 last:mb-0 leading-normal" {...props}>
      {children}
    </p>
  ),
  h1: ({ children, ...props }: React.HTMLAttributes<HTMLHeadingElement>) => (
    <h1 className="text-lg font-bold mb-2" {...props}>
      {children}
    </h1>
  ),
  h2: ({ children, ...props }: React.HTMLAttributes<HTMLHeadingElement>) => (
    <h2 className="text-base font-bold mb-2" {...props}>
      {children}
    </h2>
  ),
  h3: ({ children, ...props }: React.HTMLAttributes<HTMLHeadingElement>) => (
    <h3 className="text-[15px] font-bold mb-2" {...props}>
      {children}
    </h3>
  ),
  ul: ({ children, ...props }: React.HTMLAttributes<HTMLUListElement>) => (
    <ul className="list-disc pl-4 mb-2" {...props}>
      {children}
    </ul>
  ),
  ol: ({ children, ...props }: React.HTMLAttributes<HTMLOListElement>) => (
    <ol className="list-decimal pl-4 mb-2" {...props}>
      {children}
    </ol>
  ),
  li: ({ children, ...props }: React.LiHTMLAttributes<HTMLLIElement>) => (
    <li className="mb-1" {...props}>
      {children}
    </li>
  ),
  strong: ({ children, ...props }: React.HTMLAttributes<HTMLElement>) => (
    <strong className="font-semibold" {...props}>
      {children}
    </strong>
  ),
  em: ({ children, ...props }: React.HTMLAttributes<HTMLElement>) => (
    <em className="italic" {...props}>
      {children}
    </em>
  ),
};

// ============================================================================
// Message Component
// ============================================================================

interface MessageItemProps {
  role: string;
  content: string;
  createdAt: string;
  toolCalls?: string | null;
  isFirstInGroup?: boolean;
  isLastInGroup?: boolean;
}

function MessageItem({
  role,
  content,
  createdAt,
  toolCalls,
  isFirstInGroup = true,
  isLastInGroup = true,
}: MessageItemProps) {
  const isUser = role === "user";

  const timestamp = useMemo(() => {
    const date = new Date(createdAt);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);

    if (diffMins < 1) return "Just now";
    if (diffMins < 60) return `${diffMins}m ago`;

    return date.toLocaleTimeString([], {
      hour: "numeric",
      minute: "2-digit",
    });
  }, [createdAt]);

  // Parse tool calls from JSON string
  const parsedToolCalls = useMemo((): ToolCall[] => {
    if (!toolCalls) return [];
    try {
      const parsed = JSON.parse(toolCalls);
      if (Array.isArray(parsed)) {
        return parsed.map((tc, idx) => ({
          id: tc.id ?? `tool-${idx}`,
          name: tc.name ?? "unknown",
          arguments: tc.arguments ?? {},
          result: tc.result,
          error: tc.error,
        }));
      }
      return [];
    } catch {
      return [];
    }
  }, [toolCalls]);

  return (
    <div
      className={cn(
        "flex",
        isUser ? "justify-end" : "justify-start",
        isLastInGroup ? "mb-3" : "mb-1"
      )}
    >
      {/* Agent indicator for first assistant message */}
      {!isUser && isFirstInGroup && (
        <Bot className="w-3.5 h-3.5 mt-2 mr-2 shrink-0 text-white/40" />
      )}
      {!isUser && !isFirstInGroup && <div className="w-3.5 mr-2 shrink-0" />}

      <div className="flex flex-col max-w-[85%]">
        {/* Tool calls (shown before text content for assistant messages) */}
        {!isUser && parsedToolCalls.length > 0 && (
          <div className="space-y-1.5 mb-2">
            {parsedToolCalls.map((tc) => (
              <ToolCallIndicator key={tc.id} toolCall={tc} />
            ))}
          </div>
        )}

        {/* Message content - Refined Studio gradient bubbles */}
        <div
          className={cn(
            "px-3 py-2 text-[13px] leading-relaxed",
            isUser
              ? "rounded-[10px_10px_4px_10px]"
              : "rounded-[10px_10px_10px_4px]"
          )}
          style={{
            background: isUser
              ? "linear-gradient(135deg, #ff6b35 0%, #e85a28 100%)"
              : "linear-gradient(180deg, rgba(28,28,28,0.95) 0%, rgba(22,22,22,0.98) 100%)",
            color: isUser ? "white" : "var(--text-primary)",
            border: isUser ? "none" : "1px solid rgba(255,255,255,0.06)",
            boxShadow: isUser
              ? "0 2px 8px rgba(255,107,53,0.2)"
              : "0 1px 4px rgba(0,0,0,0.15)",
          }}
        >
          {isUser ? (
            <p className="whitespace-pre-wrap break-words">{content}</p>
          ) : (
            <div className="prose prose-sm prose-invert max-w-none">
              <ReactMarkdown components={markdownComponents}>
                {content}
              </ReactMarkdown>
            </div>
          )}
        </div>
        {isLastInGroup && (
          <span
            className={cn(
              "text-[10px] mt-1 px-1",
              isUser ? "text-right" : "text-left"
            )}
            style={{ color: "rgba(255,255,255,0.4)" }}
          >
            {timestamp}
          </span>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export interface TaskChatPanelProps {
  taskId: string;
  /** Context type - 'task' for regular chat, 'task_execution' for worker execution */
  contextType: "task" | "task_execution";
}

export function TaskChatPanel({ taskId, contextType }: TaskChatPanelProps) {
  const {
    queueMessage,
    editQueuedMessage,
    deleteQueuedMessage,
    startEditingQueuedMessage,
    queueExecutionMessage,
    deleteExecutionQueuedMessage,
  } = useChatStore();
  const queuedMessages = useChatStore(selectQueuedMessages);
  const isAgentRunning = useChatStore(selectIsAgentRunning);
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
        queueMessage(content);
      }
    },
    [isExecutionMode, taskId, queueMessage, queueExecutionMessage]
  );

  // Edit last queued message
  const handleEditLastQueued = useCallback(() => {
    const messagesToUse = isExecutionMode ? executionQueuedMessages : queuedMessages;
    const lastMessage = messagesToUse[messagesToUse.length - 1];
    if (!lastMessage) return;
    startEditingQueuedMessage(lastMessage.id);
  }, [isExecutionMode, executionQueuedMessages, queuedMessages, startEditingQueuedMessage]);

  // Subscribe to Tauri events for real-time updates (only on mount)
  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    (async () => {
      // Listen for chat chunks (streaming text)
      // Note: chat:chunk events arrive but contain complete messages (not streaming deltas)
      // We just show a typing indicator instead of trying to render partial content

      // Listen for tool calls
      const toolCallUnlisten = await listen<{
        tool_name: string;
        arguments: unknown;
        result: unknown;
        conversation_id: string;
      }>("chat:tool_call", (event) => {
        console.log("Tool call received:", event.payload);
        // Tool calls are shown in the final persisted message
      });
      unlisteners.push(toolCallUnlisten);

      // Listen for chat run completion
      const runCompletedUnlisten = await listen<{
        conversation_id: string;
      }>("chat:run_completed", (event) => {
        console.log("Chat run completed:", event.payload);
        // Query will auto-refetch
      });
      unlisteners.push(runCompletedUnlisten);

      // Execution-specific events (Phase 15B)
      // Note: execution:chunk events contain complete messages, not streaming deltas

      // Listen for execution tool calls
      const execToolCallUnlisten = await listen<{
        conversation_id: string;
        tool_name: string;
        arguments: unknown;
      }>("execution:tool_call", (event) => {
        console.log("Execution tool call received:", event.payload);
        // Tool calls are shown in the final persisted message
      });
      unlisteners.push(execToolCallUnlisten);

      // Listen for execution completion
      const execCompletedUnlisten = await listen<{
        conversation_id: string;
      }>("execution:run_completed", (event) => {
        console.log("Worker execution completed:", event.payload);
        // Query will auto-refetch due to the task:event invalidation
      });
      unlisteners.push(execCompletedUnlisten);
    })();

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []); // Empty deps - only run on mount/unmount

  // Process messages into groups
  const groupedMessages = useMemo(() => {
    return messagesData.map((msg, index) => {
      const prevMsg = messagesData[index - 1];
      const nextMsg = messagesData[index + 1];
      const isFirstInGroup = !prevMsg || prevMsg.role !== msg.role;
      const isLastInGroup = !nextMsg || nextMsg.role !== msg.role;
      return { ...msg, isFirstInGroup, isLastInGroup };
    });
  }, [messagesData]);

  const isLoading = activeConversation.isLoading;
  const isSending = sendMessage.isPending;
  const isEmpty = !isLoading && groupedMessages.length === 0;

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
                {groupedMessages.map((msg) => (
                  <MessageItem
                    key={msg.id}
                    role={msg.role}
                    content={msg.content}
                    createdAt={msg.createdAt}
                    toolCalls={msg.toolCalls}
                    isFirstInGroup={msg.isFirstInGroup}
                    isLastInGroup={msg.isLastInGroup}
                  />
                ))}
                {/* Show typing indicator while agent is working */}
                {isSending && <TypingIndicator />}
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
              : deleteQueuedMessage;

            return messagesToDisplay.length > 0 && (
              <div className="p-3 pb-0">
                <QueuedMessageList
                  messages={messagesToDisplay}
                  onEdit={editQueuedMessage}
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
