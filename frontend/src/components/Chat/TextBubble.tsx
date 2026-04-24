/**
 * TextBubble - Chat message text bubble with copy functionality
 *
 * Renders text content with:
 * - User vs assistant styling
 * - Markdown rendering for user and assistant messages
 */

import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { cn } from "@/lib/utils";
import { markdownComponents } from "./MessageItem.markdown";

interface TextBubbleProps {
  text: string;
  isUser: boolean;
}

export function TextBubble({ text, isUser }: TextBubbleProps) {
  return (
    <div
      data-testid={isUser ? "text-bubble-user" : "text-bubble-assistant"}
      className={cn(
        "w-fit text-[13px] leading-relaxed break-words",
        isUser ? "px-3 py-2 rounded-xl" : "px-0 py-0 rounded-none",
        isUser ? "self-end" : "self-start"
      )}
      style={{
        maxWidth: "min(85%, 620px)",
        background: isUser ? "var(--chat-user-bubble-bg)" : "transparent",
        color: isUser ? "var(--chat-user-bubble-text)" : "var(--text-primary)",
        borderWidth: isUser ? "1px" : "0",
        borderStyle: isUser ? "solid" : "none",
        borderColor: isUser ? "var(--chat-user-bubble-border)" : "transparent",
        boxShadow: "none",
      }}
    >
      <div className="max-w-none overflow-hidden [&>p]:mb-0">
        <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
          {text}
        </ReactMarkdown>
      </div>
    </div>
  );
}
