/**
 * ChatMessage - Individual chat message display component
 *
 * Design spec: specs/design/pages/chat-panel.md
 * - Refined Studio aesthetic with gradient bubbles
 * - Asymmetric corners (10px/10px/10px/4px)
 * - User: warm orange gradient, right-aligned
 * - Agent: layered dark gradient with subtle border
 * - Compact sizing for application UI
 */

import { useMemo } from "react";
import ReactMarkdown from "react-markdown";
import type { ChatMessage as ChatMessageType, MessageRole } from "@/types/ideation";
import { ToolCallIndicator, type ToolCall } from "./ToolCallIndicator";

// ============================================================================
// Types
// ============================================================================

interface ChatMessageProps {
  /** The message to display */
  message: ChatMessageType;
  /** Show full timestamp with date instead of just time */
  showFullTimestamp?: boolean;
  /** Compact mode with reduced spacing and no role indicator */
  compact?: boolean;
}

// ============================================================================
// Helpers
// ============================================================================

function getRoleLabel(role: MessageRole): string {
  switch (role) {
    case "user":
      return "You";
    case "orchestrator":
      return "Orchestrator";
    case "system":
      return "System";
    default:
      return "Unknown";
  }
}

function getAccessibleName(role: MessageRole): string {
  const roleLabel = getRoleLabel(role);
  return `Message from ${roleLabel}`;
}

function formatTimestamp(dateString: string, full: boolean): string {
  const date = new Date(dateString);

  if (full) {
    return date.toLocaleString([], {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  return date.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
}

// ============================================================================
// Markdown Components
// ============================================================================

const markdownComponents = {
  // Style links
  a: ({ href, children, ...props }: React.AnchorHTMLAttributes<HTMLAnchorElement>) => (
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
  // Style inline code
  code: ({ className, children, ...props }: React.HTMLAttributes<HTMLElement>) => {
    const isBlock = className?.includes("language-");
    if (isBlock) {
      return (
        <code
          className={`block p-3 rounded text-sm overflow-x-auto ${className || ""}`}
          style={{ backgroundColor: "var(--bg-elevated)" }}
          {...props}
        >
          {children}
        </code>
      );
    }
    return (
      <code
        className="px-1 py-0.5 rounded text-sm"
        style={{ backgroundColor: "var(--bg-elevated)" }}
        {...props}
      >
        {children}
      </code>
    );
  },
  // Style code blocks container
  pre: ({ children, ...props }: React.HTMLAttributes<HTMLPreElement>) => (
    <pre
      className="my-2 rounded overflow-hidden"
      style={{ backgroundColor: "var(--bg-elevated)" }}
      {...props}
    >
      {children}
    </pre>
  ),
  // Style paragraphs
  p: ({ children, ...props }: React.HTMLAttributes<HTMLParagraphElement>) => (
    <p className="mb-2 last:mb-0" {...props}>
      {children}
    </p>
  ),
  // Style lists
  ul: ({ children, ...props }: React.HTMLAttributes<HTMLUListElement>) => (
    <ul className="list-disc list-inside mb-2" {...props}>
      {children}
    </ul>
  ),
  ol: ({ children, ...props }: React.HTMLAttributes<HTMLOListElement>) => (
    <ol className="list-decimal list-inside mb-2" {...props}>
      {children}
    </ol>
  ),
  // Style list items
  li: ({ children, ...props }: React.LiHTMLAttributes<HTMLLIElement>) => (
    <li className="mb-1" {...props}>
      {children}
    </li>
  ),
  // Style strong/bold
  strong: ({ children, ...props }: React.HTMLAttributes<HTMLElement>) => (
    <strong className="font-semibold" {...props}>
      {children}
    </strong>
  ),
  // Style emphasis/italic
  em: ({ children, ...props }: React.HTMLAttributes<HTMLElement>) => (
    <em className="italic" {...props}>
      {children}
    </em>
  ),
};

// ============================================================================
// Component
// ============================================================================

export function ChatMessage({
  message,
  showFullTimestamp = false,
  compact = false,
}: ChatMessageProps) {
  const isUser = message.role === "user";
  const alignmentClass = isUser ? "items-end" : "items-start";
  const spacingClass = compact ? "mb-1" : "mb-3";

  // Refined Studio bubble styles with gradients
  const bubbleStyle = useMemo(
    (): React.CSSProperties => ({
      background: isUser
        ? "linear-gradient(135deg, #ff6b35 0%, #e85a28 100%)"
        : "linear-gradient(180deg, rgba(28,28,28,0.95) 0%, rgba(22,22,22,0.98) 100%)",
      color: isUser ? "white" : "var(--text-primary)",
      border: isUser ? "none" : "1px solid rgba(255,255,255,0.06)",
      boxShadow: isUser
        ? "0 2px 8px rgba(255,107,53,0.2)"
        : "0 1px 4px rgba(0,0,0,0.15)",
    }),
    [isUser]
  );

  const timestamp = useMemo(
    () => formatTimestamp(message.createdAt, showFullTimestamp),
    [message.createdAt, showFullTimestamp]
  );

  const accessibleName = useMemo(
    () => getAccessibleName(message.role),
    [message.role]
  );

  // Parse tool calls if present
  const toolCalls = useMemo<ToolCall[]>(() => {
    if (!message.toolCalls) return [];
    try {
      const parsed = JSON.parse(message.toolCalls);
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  }, [message.toolCalls]);

  return (
    <article
      data-testid={`chat-message-${message.id}`}
      className={`flex flex-col ${alignmentClass} ${spacingClass}`}
      aria-label={accessibleName}
    >
      {/* Role indicator (hidden in compact mode) */}
      {!compact && (
        <span
          data-testid="chat-message-role"
          className="text-xs font-medium mb-1 px-1"
          style={{ color: "var(--text-muted)" }}
        >
          {getRoleLabel(message.role)}
        </span>
      )}

      {/* Message bubble - Refined Studio asymmetric corners */}
      <div
        data-testid="chat-message-bubble"
        className="max-w-[85%] px-3 py-2 break-words"
        style={{
          ...bubbleStyle,
          borderRadius: isUser ? "10px 10px 4px 10px" : "10px 10px 10px 4px",
        }}
      >
        <div className="text-[13px] leading-relaxed">
          <ReactMarkdown components={markdownComponents}>
            {message.content}
          </ReactMarkdown>
        </div>

        {/* Tool calls (if any) */}
        {toolCalls.length > 0 && (
          <div className="mt-2.5 space-y-1.5" data-testid="chat-message-tool-calls">
            {toolCalls.map((toolCall) => (
              <ToolCallIndicator key={toolCall.id} toolCall={toolCall} />
            ))}
          </div>
        )}
      </div>

      {/* Timestamp */}
      <time
        data-testid="chat-message-timestamp"
        dateTime={message.createdAt}
        className="text-[10px] mt-1 px-1"
        style={{ color: "rgba(255,255,255,0.4)" }}
        role="time"
      >
        {timestamp}
      </time>
    </article>
  );
}
