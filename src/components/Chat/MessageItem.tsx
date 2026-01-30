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
import { cn } from "@/lib/utils";
import { ToolCallIndicator, type ToolCall } from "./ToolCallIndicator";
import { TextBubble } from "./TextBubble";
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
              return <TextBubble key={`block-${index}`} text={block.text} isUser={isUser} />;
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
            <TextBubble text={content} isUser={isUser} />
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
