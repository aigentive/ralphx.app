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
