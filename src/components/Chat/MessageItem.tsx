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

import React from "react";
import { Bot } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { cn } from "@/lib/utils";
import { ToolCallIndicator, type ToolCall } from "./ToolCallIndicator";
import { markdownComponents } from "./MessageItem.markdown";
import { formatTimestamp } from "./MessageItem.utils";

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
  /** Pre-parsed tool calls array (parsed at API layer) */
  toolCalls?: ToolCall[] | null;
  /** Pre-parsed content blocks array (parsed at API layer) */
  contentBlocks?: ContentBlockItem[] | null;
}

// ============================================================================
// Message Component
// ============================================================================

export const MessageItem = React.memo(function MessageItem({
  role,
  content,
  createdAt,
  toolCalls,
  contentBlocks,
}: MessageItemProps) {
  const isUser = role === "user";

  // Use pre-parsed data directly (parsing now happens at API layer)
  const parsedContentBlocks = contentBlocks ?? [];
  const parsedToolCalls = toolCalls ?? [];
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
          {formatTimestamp(createdAt)}
        </span>
      </div>
    </div>
  );
}, (prev, next) => {
  // Custom equality function - only re-render if these props change
  // For arrays, compare by reference (they're parsed once at API layer)
  return prev.role === next.role
    && prev.content === next.content
    && prev.createdAt === next.createdAt
    && prev.toolCalls === next.toolCalls
    && prev.contentBlocks === next.contentBlocks;
});
