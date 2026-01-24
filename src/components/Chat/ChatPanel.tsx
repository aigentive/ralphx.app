/**
 * ChatPanel - Resizable side panel for context-aware chat
 *
 * Features:
 * - Toggle with Cmd+K keyboard shortcut
 * - Resizable width (min 280px, max 50% viewport)
 * - Context indicator showing current view
 * - Message list with auto-scroll
 * - Input with Enter to send, Shift+Enter for newline
 */

import { useState, useRef, useEffect, useCallback } from "react";
import { useChat } from "@/hooks/useChat";
import { useChatStore } from "@/stores/chatStore";
import type { ChatContext } from "@/types/chat";

// ============================================================================
// Constants
// ============================================================================

const MIN_WIDTH = 280;
const MAX_WIDTH_PERCENT = 50;

// ============================================================================
// Icons
// ============================================================================

function CloseIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M12 4L4 12M4 4L12 12"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

function SendIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M14 2L2 7.5L6.5 9.5M14 2L9.5 14L6.5 9.5M14 2L6.5 9.5"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

// ============================================================================
// Sub-components
// ============================================================================

function LoadingSpinner() {
  return (
    <div
      data-testid="chat-panel-loading"
      className="flex items-center justify-center p-8"
    >
      <div
        className="w-6 h-6 border-2 rounded-full animate-spin"
        style={{
          borderColor: "var(--border-subtle)",
          borderTopColor: "var(--accent-primary)",
        }}
      />
    </div>
  );
}

function EmptyState() {
  return (
    <div
      data-testid="chat-panel-empty"
      className="flex flex-col items-center justify-center h-full p-8 text-center"
    >
      <svg
        width="48"
        height="48"
        viewBox="0 0 48 48"
        fill="none"
        className="mb-4"
        style={{ color: "var(--text-muted)" }}
      >
        <circle
          cx="24"
          cy="24"
          r="20"
          stroke="currentColor"
          strokeWidth="2"
          strokeDasharray="4 4"
        />
        <path
          d="M16 20h16M16 24h12M16 28h8"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
        />
      </svg>
      <p style={{ color: "var(--text-secondary)" }}>No messages yet</p>
      <p
        className="text-sm mt-1"
        style={{ color: "var(--text-muted)" }}
      >
        Start a conversation
      </p>
    </div>
  );
}

function ContextIndicator({ context }: { context: ChatContext }) {
  const getContextLabel = () => {
    switch (context.view) {
      case "ideation":
        return "Ideation";
      case "kanban":
        return context.selectedTaskId ? "Task" : "Kanban";
      case "task_detail":
        return "Task";
      case "activity":
        return "Activity";
      case "settings":
        return "Settings";
      default:
        return "Chat";
    }
  };

  return (
    <span
      className="px-2 py-0.5 text-xs font-medium rounded"
      style={{
        backgroundColor: "var(--bg-elevated)",
        color: "var(--text-secondary)",
      }}
    >
      {getContextLabel()}
    </span>
  );
}

interface MessageItemProps {
  role: string;
  content: string;
  createdAt: string;
}

function MessageItem({ role, content, createdAt }: MessageItemProps) {
  const isUser = role === "user";
  const timestamp = new Date(createdAt).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });

  return (
    <div
      className={`flex flex-col ${isUser ? "items-end" : "items-start"} mb-3`}
    >
      <div
        className="max-w-[85%] px-3 py-2 rounded-lg"
        style={{
          backgroundColor: isUser ? "var(--accent-primary)" : "var(--bg-elevated)",
          color: isUser ? "var(--text-primary)" : "var(--text-primary)",
        }}
      >
        <p className="text-sm whitespace-pre-wrap break-words">{content}</p>
      </div>
      <span
        className="text-xs mt-1 px-1"
        style={{ color: "var(--text-muted)" }}
      >
        {timestamp}
      </span>
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
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const resizeRef = useRef<{ startX: number; startWidth: number } | null>(null);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    if (messagesEndRef.current && messages.data?.length) {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages.data?.length]);

  // Keyboard shortcut handler
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.metaKey && e.key === "k") {
        // Don't toggle if an input is focused
        const activeElement = document.activeElement;
        if (
          activeElement instanceof HTMLInputElement ||
          activeElement instanceof HTMLTextAreaElement
        ) {
          return;
        }
        e.preventDefault();
        togglePanel();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [togglePanel]);

  // Resize handlers
  const handleResizeStart = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
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

  if (!isOpen) {
    return null;
  }

  const messagesList = messages.data ?? [];
  const isLoading = messages.isLoading;
  const isSending = sendMessage.isPending;
  const isEmpty = !isLoading && messagesList.length === 0;

  return (
    <aside
      ref={panelRef}
      data-testid="chat-panel"
      role="complementary"
      aria-label="Chat panel"
      className="flex flex-col h-full border-l"
      style={{
        width: `${width}px`,
        minWidth: `${MIN_WIDTH}px`,
        backgroundColor: "var(--bg-surface)",
        borderColor: "var(--border-subtle)",
      }}
    >
      {/* Resize Handle */}
      <div
        data-testid="chat-panel-resize-handle"
        className="absolute left-0 top-0 bottom-0 w-1 cursor-ew-resize hover:bg-accent-primary/20"
        onMouseDown={handleResizeStart}
        style={{ marginLeft: "-2px" }}
      />

      {/* Header */}
      <div
        data-testid="chat-panel-header"
        className="flex items-center justify-between px-4 py-3 border-b"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <div className="flex items-center gap-2">
          <h2
            className="text-sm font-semibold"
            style={{ color: "var(--text-primary)" }}
          >
            Chat
          </h2>
          <ContextIndicator context={context} />
        </div>
        <button
          data-testid="chat-panel-close"
          onClick={togglePanel}
          className="p-1 rounded hover:bg-white/5"
          style={{ color: "var(--text-secondary)" }}
          aria-label="Close chat panel"
        >
          <CloseIcon />
        </button>
      </div>

      {/* Messages Area */}
      <div
        data-testid="chat-panel-messages"
        className="flex-1 overflow-y-auto p-4"
      >
        {isLoading ? (
          <LoadingSpinner />
        ) : isEmpty ? (
          <EmptyState />
        ) : (
          <>
            {messagesList.map((msg) => (
              <MessageItem
                key={msg.id}
                role={msg.role}
                content={msg.content}
                createdAt={msg.createdAt}
              />
            ))}
            <div ref={messagesEndRef} />
          </>
        )}
      </div>

      {/* Input Area */}
      <div
        className="p-3 border-t"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <div className="flex gap-2">
          <textarea
            ref={inputRef}
            data-testid="chat-panel-input"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={handleInputKeyDown}
            disabled={isSending}
            placeholder="Send a message..."
            className="flex-1 px-3 py-2 text-sm resize-none rounded-lg outline-none"
            style={{
              backgroundColor: "var(--bg-elevated)",
              color: "var(--text-primary)",
              minHeight: "40px",
              maxHeight: "120px",
            }}
            rows={1}
            aria-label="Message input"
          />
          <button
            data-testid="chat-panel-send"
            onClick={handleSend}
            disabled={!inputValue.trim() || isSending}
            className="px-3 py-2 rounded-lg transition-colors disabled:opacity-50"
            style={{
              backgroundColor: "var(--accent-primary)",
              color: "var(--text-primary)",
            }}
            aria-label="Send message"
          >
            <SendIcon />
          </button>
        </div>
        <p
          className="text-xs mt-1"
          style={{ color: "var(--text-muted)" }}
        >
          Press Enter to send, Shift+Enter for new line
        </p>
      </div>
    </aside>
  );
}
