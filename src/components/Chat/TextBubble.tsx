/**
 * TextBubble - Chat message text bubble with copy functionality
 *
 * Renders text content with:
 * - User vs assistant styling
 * - Copy button on hover
 * - Markdown rendering for assistant messages
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
      className={cn(
        "relative group px-3 py-2 text-[13px] leading-relaxed",
        /* macOS Tahoe: uniform rounded corners */
        "rounded-xl"
      )}
      style={{
        /* macOS Tahoe: flat solid colors, no gradients */
        background: isUser
          ? "hsl(14 100% 60%)" /* Accent orange - flat */
          : "hsl(220 10% 14%)", /* Dark surface - flat */
        color: isUser ? "white" : "hsl(220 10% 90%)",
        border: "none",
        boxShadow: "none",
      }}
    >
      {isUser ? (
        <p className="whitespace-pre-wrap break-words overflow-hidden leading-relaxed">{text}</p>
      ) : (
        <div className="max-w-none overflow-hidden [&>p]:mb-0">
          <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
            {text}
          </ReactMarkdown>
        </div>
      )}
      <Button
        variant="ghost"
        size="icon-sm"
        onClick={handleCopy}
        className={cn(
          "absolute top-1 right-1 opacity-0 group-hover:opacity-100 transition-opacity",
          isUser
            ? "hover:bg-white/20 text-white/80 hover:text-white"
            : "hover:bg-white/10"
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
