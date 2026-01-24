/**
 * ChatMessage - Individual chat message display component
 *
 * Features:
 * - Role indicator (user vs orchestrator vs system)
 * - Markdown rendering for content
 * - Timestamp display (compact or full)
 * - User messages aligned right, orchestrator/system left
 * - Warm colors for user, neutral for orchestrator
 */

import { useMemo } from "react";
import ReactMarkdown from "react-markdown";
import type { ChatMessage as ChatMessageType, MessageRole } from "@/types/ideation";

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

  const bubbleStyle = useMemo(
    () => ({
      backgroundColor: isUser ? "var(--accent-primary)" : "var(--bg-elevated)",
      color: "var(--text-primary)",
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

      {/* Message bubble */}
      <div
        data-testid="chat-message-bubble"
        className="max-w-[85%] px-3 py-2 rounded-lg break-words"
        style={bubbleStyle}
      >
        <div className="text-sm">
          <ReactMarkdown components={markdownComponents}>
            {message.content}
          </ReactMarkdown>
        </div>
      </div>

      {/* Timestamp */}
      <time
        data-testid="chat-message-timestamp"
        dateTime={message.createdAt}
        className="text-xs mt-1 px-1"
        style={{ color: "var(--text-muted)" }}
        role="time"
      >
        {timestamp}
      </time>
    </article>
  );
}
