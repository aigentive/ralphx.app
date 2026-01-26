/**
 * MessageItem - Shared chat message component
 *
 * Renders a single chat message with support for:
 * - Interleaved text and tool calls (content blocks)
 * - Legacy rendering fallback (tool calls first, then text)
 * - User vs assistant styling
 * - Markdown rendering for assistant messages
 * - Code blocks with copy functionality
 */

import { useState, useCallback, useMemo } from "react";
import { Bot, Copy, Check } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { ToolCallIndicator, type ToolCall } from "./ToolCallIndicator";

// ============================================================================
// Types
// ============================================================================

/**
 * Content block item - represents either text or a tool use
 */
export interface ContentBlockItem {
  type: "text" | "tool_use";
  text?: string;
  id?: string;
  name?: string;
  arguments?: unknown;
  result?: unknown;
}

export interface MessageItemProps {
  role: string;
  content: string;
  createdAt: string;
  toolCalls?: string | null;
  contentBlocks?: string | null;
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
    <div className="relative group my-2 max-w-full overflow-hidden">
      {language && (
        <span
          className="absolute top-1 left-3 text-[11px]"
          style={{ color: "var(--text-muted)" }}
        >
          {language}
        </span>
      )}
      <pre
        className="rounded-md overflow-x-auto max-w-full"
        style={{
          backgroundColor: "var(--bg-base)",
          border: "1px solid var(--border-subtle)",
        }}
      >
        <code
          className={cn("block p-3 text-[13px]", language && "pt-6")}
          style={{
            fontFamily: "var(--font-mono)",
            whiteSpace: "pre-wrap",
            wordBreak: "break-all",
          }}
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
        className="px-1 py-0.5 rounded text-[13px] break-all"
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
  pre: ({ children }: React.HTMLAttributes<HTMLPreElement>) => <>{children}</>,
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
  // Table support
  table: ({ children, ...props }: React.TableHTMLAttributes<HTMLTableElement>) => (
    <div className="overflow-x-auto my-2">
      <table
        className="min-w-full text-[12px] border-collapse"
        style={{ borderColor: "var(--border-subtle)" }}
        {...props}
      >
        {children}
      </table>
    </div>
  ),
  thead: ({ children, ...props }: React.HTMLAttributes<HTMLTableSectionElement>) => (
    <thead
      style={{ backgroundColor: "var(--bg-base)" }}
      {...props}
    >
      {children}
    </thead>
  ),
  tbody: ({ children, ...props }: React.HTMLAttributes<HTMLTableSectionElement>) => (
    <tbody {...props}>{children}</tbody>
  ),
  tr: ({ children, ...props }: React.HTMLAttributes<HTMLTableRowElement>) => (
    <tr
      className="border-b"
      style={{ borderColor: "var(--border-subtle)" }}
      {...props}
    >
      {children}
    </tr>
  ),
  th: ({ children, ...props }: React.ThHTMLAttributes<HTMLTableCellElement>) => (
    <th
      className="px-2 py-1.5 text-left font-semibold"
      style={{ color: "var(--text-primary)", borderColor: "var(--border-subtle)" }}
      {...props}
    >
      {children}
    </th>
  ),
  td: ({ children, ...props }: React.TdHTMLAttributes<HTMLTableCellElement>) => (
    <td
      className="px-2 py-1.5"
      style={{ color: "var(--text-secondary)", borderColor: "var(--border-subtle)" }}
      {...props}
    >
      {children}
    </td>
  ),
};

// ============================================================================
// Message Component
// ============================================================================

export function MessageItem({
  role,
  content,
  createdAt,
  toolCalls,
  contentBlocks,
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

  // Parse content blocks for interleaved rendering
  const parsedContentBlocks = useMemo((): ContentBlockItem[] => {
    if (!contentBlocks) return [];
    try {
      const parsed = JSON.parse(contentBlocks);
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  }, [contentBlocks]);

  // Parse tool calls from JSON string (fallback for legacy messages)
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

  const hasContentBlocks = parsedContentBlocks.length > 0;

  // Render a text bubble
  const renderTextBubble = (text: string, key: string) => (
    <div
      key={key}
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
        <p className="whitespace-pre-wrap break-words overflow-hidden">{text}</p>
      ) : (
        <div className="prose prose-sm prose-invert max-w-none overflow-hidden">
          <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
            {text}
          </ReactMarkdown>
        </div>
      )}
    </div>
  );

  return (
    <div
      className={cn(
        "flex min-w-0 mb-5",
        isUser ? "justify-end" : "justify-start"
      )}
    >
      {/* Agent indicator for assistant messages */}
      {!isUser && (
        <Bot className="w-3.5 h-3.5 mt-2 mr-2 shrink-0 text-white/40" />
      )}

      <div className="flex flex-col gap-3 max-w-[85%] min-w-0">
        {hasContentBlocks ? (
          // Render content blocks in order (interleaved text and tool calls)
          parsedContentBlocks.map((block, index) => {
            if (block.type === "text" && block.text) {
              return renderTextBubble(block.text, `block-${index}`);
            } else if (block.type === "tool_use" && block.name) {
              const toolCall: ToolCall = {
                id: block.id || `tool-${index}`,
                name: block.name,
                arguments: block.arguments,
                result: block.result,
              };
              return <ToolCallIndicator key={`block-${index}`} toolCall={toolCall} />;
            }
            return null;
          })
        ) : (
          // Legacy rendering: tool calls first, then content
          <>
            {!isUser && parsedToolCalls.length > 0 && (
              <div className="space-y-1.5 overflow-hidden">
                {parsedToolCalls.map((tc) => (
                  <ToolCallIndicator key={tc.id} toolCall={tc} />
                ))}
              </div>
            )}
            {renderTextBubble(content, "content")}
          </>
        )}

        <span
          className={cn(
            "text-[10px] mt-1 px-1",
            isUser ? "text-right" : "text-left"
          )}
          style={{ color: "rgba(255,255,255,0.4)" }}
        >
          {timestamp}
        </span>
      </div>
    </div>
  );
}
