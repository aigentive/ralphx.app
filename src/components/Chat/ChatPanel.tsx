/**
 * ChatPanel - Premium resizable side panel for context-aware chat
 *
 * Features:
 * - Toggle with Cmd+K keyboard shortcut
 * - Resizable width (min 280px, max 50% viewport)
 * - Slide-in/out animation
 * - Collapsible to thin bar with unread indicator
 * - Context indicator showing current view
 * - Message bubbles with asymmetric corners
 * - Auto-scroll with manual scroll override
 * - Typing indicator
 * - Markdown rendering with syntax highlighting
 */

import { useState, useRef, useEffect, useCallback, useMemo } from "react";
import { useChat } from "@/hooks/useChat";
import { useChatStore } from "@/stores/chatStore";
import type { ChatContext } from "@/types/chat";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Textarea } from "@/components/ui/textarea";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  MessageSquare,
  CheckSquare,
  FolderKanban,
  Bot,
  X,
  PanelRightClose,
  PanelRightOpen,
  ArrowUp,
  Loader2,
  Copy,
  Check,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import { cn } from "@/lib/utils";

// ============================================================================
// Constants
// ============================================================================

const MIN_WIDTH = 280;
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
      <Bot className="w-3.5 h-3.5 mt-2.5 shrink-0 text-[var(--text-muted)]" />
      <div
        className="px-3.5 py-2.5 rounded-[10px_10px_10px_4px]"
        style={{
          backgroundColor: "var(--bg-elevated)",
          border: "1px solid var(--border-subtle)",
        }}
      >
        <div className="flex items-center gap-1">
          <div
            className="typing-dot w-1.5 h-1.5 rounded-full"
            style={{ backgroundColor: "var(--text-muted)" }}
          />
          <div
            className="typing-dot w-1.5 h-1.5 rounded-full"
            style={{ backgroundColor: "var(--text-muted)" }}
          />
          <div
            className="typing-dot w-1.5 h-1.5 rounded-full"
            style={{ backgroundColor: "var(--text-muted)" }}
          />
        </div>
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div
      data-testid="chat-panel-empty"
      className="flex flex-col items-center justify-center h-full p-8 text-center"
    >
      <MessageSquare
        className="w-10 h-10 mb-3"
        style={{ color: "var(--text-muted)" }}
      />
      <p
        className="text-sm font-medium"
        style={{ color: "var(--text-secondary)" }}
      >
        Start a conversation
      </p>
      <p className="text-[13px] mt-1" style={{ color: "var(--text-muted)" }}>
        Ask questions or get help with your tasks
      </p>
    </div>
  );
}

function LoadingState() {
  return (
    <div
      data-testid="chat-panel-loading"
      className="flex items-center justify-center p-8"
    >
      <Loader2
        className="w-6 h-6 animate-spin"
        style={{ color: "var(--accent-primary)" }}
      />
    </div>
  );
}

interface ContextIndicatorProps {
  context: ChatContext;
}

function ContextIndicator({ context }: ContextIndicatorProps) {
  const getContextInfo = () => {
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
      <Icon className="w-4 h-4 shrink-0 text-[var(--text-secondary)]" />
      <span className="text-sm font-medium truncate">{label}</span>
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
  isFirstInGroup?: boolean;
  isLastInGroup?: boolean;
}

function MessageItem({
  role,
  content,
  createdAt,
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
        <Bot className="w-3.5 h-3.5 mt-2.5 mr-2 shrink-0 text-[var(--text-muted)]" />
      )}
      {!isUser && !isFirstInGroup && <div className="w-3.5 mr-2 shrink-0" />}

      <div className="flex flex-col max-w-[85%]">
        <div
          className={cn(
            "px-3 py-2 text-sm",
            isUser
              ? "rounded-[10px_10px_4px_10px]"
              : "rounded-[10px_10px_10px_4px]"
          )}
          style={{
            backgroundColor: isUser
              ? "var(--accent-primary)"
              : "var(--bg-elevated)",
            color: isUser ? "white" : "var(--text-primary)",
            border: isUser ? "none" : "1px solid var(--border-subtle)",
            boxShadow: isUser ? "var(--shadow-xs)" : "none",
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
              "text-[11px] mt-1 px-1",
              isUser ? "text-right" : "text-left"
            )}
            style={{ color: "var(--text-muted)" }}
          >
            {timestamp}
          </span>
        )}
      </div>
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
      className="fixed top-12 right-0 bottom-0 flex flex-col items-center justify-center"
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

export function ChatPanel({ context }: ChatPanelProps) {
  const { isOpen, width, togglePanel, setWidth } = useChatStore();
  const { messages, sendMessage } = useChat(context);
  const [inputValue, setInputValue] = useState("");
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const [isExiting, setIsExiting] = useState(false);
  const [hasUnread, setHasUnread] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const resizeRef = useRef<{ startX: number; startWidth: number } | null>(null);
  const lastMessageCountRef = useRef(0);

  // Track unread messages when collapsed
  useEffect(() => {
    const messageCount = messages.data?.length ?? 0;
    if (isCollapsed && messageCount > lastMessageCountRef.current) {
      setHasUnread(true);
    }
    lastMessageCountRef.current = messageCount;
  }, [messages.data?.length, isCollapsed]);

  // Clear unread when expanded
  useEffect(() => {
    if (!isCollapsed) {
      setHasUnread(false);
    }
  }, [isCollapsed]);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    if (messagesEndRef.current && messages.data?.length) {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages.data?.length]);

  // Close with animation
  const handleClose = useCallback(() => {
    setIsExiting(true);
    setTimeout(() => {
      togglePanel();
      setIsExiting(false);
    }, 200);
  }, [togglePanel]);

  // Escape to close panel
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isOpen && !isCollapsed) {
        handleClose();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, isCollapsed, handleClose]);

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
  const handleSend = useCallback(async () => {
    const trimmedValue = inputValue.trim();
    if (!trimmedValue || sendMessage.isPending) return;

    try {
      await sendMessage.mutateAsync(trimmedValue);
      setInputValue("");
    } catch {
      // Error is handled by the mutation
    }
  }, [inputValue, sendMessage]);

  // Input key handler
  const handleInputKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend]
  );

  // Process messages into groups
  const groupedMessages = useMemo(() => {
    const msgs = messages.data ?? [];
    return msgs.map((msg, index) => {
      const prevMsg = msgs[index - 1];
      const nextMsg = msgs[index + 1];
      const isFirstInGroup = !prevMsg || prevMsg.role !== msg.role;
      const isLastInGroup = !nextMsg || nextMsg.role !== msg.role;
      return { ...msg, isFirstInGroup, isLastInGroup };
    });
  }, [messages.data]);

  if (!isOpen) {
    return null;
  }

  if (isCollapsed) {
    return (
      <CollapsedPanel
        onExpand={() => setIsCollapsed(false)}
        hasUnread={hasUnread}
      />
    );
  }

  const isLoading = messages.isLoading;
  const isSending = sendMessage.isPending;
  const isEmpty = !isLoading && groupedMessages.length === 0;

  return (
    <>
      <style>{animationStyles}</style>
      <aside
        ref={panelRef}
        data-testid="chat-panel"
        role="complementary"
        aria-label="Chat panel"
        className={cn(
          "fixed top-12 right-0 bottom-0 flex flex-col",
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

        {/* Header */}
        <div
          data-testid="chat-panel-header"
          className="flex items-center justify-between h-12 px-3 border-b"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          <ContextIndicator context={context} />

          {/* Active agent badge */}
          {isSending && (
            <Badge variant="secondary" className="shrink-0 mr-2">
              <Loader2 className="w-3 h-3 mr-1 animate-spin" />
              Working
            </Badge>
          )}

          <div className="flex items-center gap-1 shrink-0">
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
                    isFirstInGroup={msg.isFirstInGroup}
                    isLastInGroup={msg.isLastInGroup}
                  />
                ))}
                {isSending && <TypingIndicator />}
                <div ref={messagesEndRef} />
              </>
            )}
          </div>
        </ScrollArea>

        {/* Input Area */}
        <div
          className="relative p-3 border-t"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          <Textarea
            data-testid="chat-panel-input"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={handleInputKeyDown}
            disabled={isSending}
            placeholder="Send a message..."
            className="pr-12 min-h-[40px] max-h-[100px] resize-none text-sm"
            style={{
              backgroundColor: "var(--bg-elevated)",
              borderColor: "var(--border-subtle)",
            }}
            rows={1}
            aria-label="Message input"
          />
          <Button
            data-testid="chat-panel-send"
            variant={inputValue.trim() ? "default" : "ghost"}
            size="icon-sm"
            disabled={!inputValue.trim() || isSending}
            onClick={handleSend}
            className="absolute right-4 top-1/2 -translate-y-1/2"
            style={{
              backgroundColor: inputValue.trim()
                ? "var(--accent-primary)"
                : "transparent",
            }}
            aria-label="Send message"
          >
            {isSending ? (
              <Loader2 className="w-[18px] h-[18px] animate-spin" />
            ) : (
              <ArrowUp className="w-[18px] h-[18px]" />
            )}
          </Button>
        </div>
      </aside>
    </>
  );
}
