/**
 * TextBubble - Chat message text bubble with copy functionality
 *
 * Renders text content with:
 * - User vs assistant styling
 * - Copy button on hover
 * - Markdown rendering for user and assistant messages
 */

import { useState, useCallback } from "react";
import { Copy, Check } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { markdownComponents } from "./MessageItem.markdown";

interface TextBubbleProps {
  text: string;
  isUser: boolean;
}

export function TextBubble({ text, isUser }: TextBubbleProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Silently fail
    }
  }, [text]);

  return (
    <div
      data-testid={isUser ? "text-bubble-user" : "text-bubble-assistant"}
      className={cn(
        "relative group w-fit max-w-[350px] px-3 py-2 text-[13px] leading-relaxed break-words",
        /* macOS Tahoe: uniform rounded corners */
        "rounded-xl",
        isUser ? "self-end" : "self-start"
      )}
      style={{
        /* macOS Tahoe: flat solid colors, no gradients */
        background: isUser
          ? "var(--accent-primary)" /* Accent orange - flat */
          : "var(--bg-elevated)", /* Dark surface - flat */
        color: isUser ? "var(--text-inverse)" : "var(--text-primary)",
        border: "none",
        boxShadow: "none",
      }}
    >
      <div className="max-w-none overflow-hidden [&>p]:mb-0">
        <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
          {text}
        </ReactMarkdown>
      </div>
      <Button
        variant="ghost"
        size="icon-sm"
        onClick={handleCopy}
        className={cn(
          "absolute top-1 right-1 opacity-0 group-hover:opacity-100 transition-opacity",
          isUser
            ? "hover:bg-[color-mix(in_srgb,_var(--text-inverse)_20%,_transparent)] text-[var(--text-inverse)]"
            : "hover:bg-[var(--overlay-moderate)]"
        )}
        aria-label={copied ? "Copied" : "Copy message"}
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
