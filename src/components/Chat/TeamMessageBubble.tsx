/**
 * TeamMessageBubble — Inter-agent message display
 *
 * Shows messages between teammates: "from → to: content"
 * Uses a dimmed background style to distinguish from regular chat messages.
 */

import React, { useState, useRef, useEffect, useCallback } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { markdownComponents } from "./MessageItem.markdown";

const COLLAPSED_MAX_HEIGHT = 73;
const GRADIENT_HEIGHT = 36;
const BG_COLOR = "hsl(220 10% 11%)";

interface TeamMessageBubbleProps {
  from: string;
  to: string;
  content: string;
  fromColor?: string | undefined;
  timestamp?: string | undefined;
}

export const TeamMessageBubble = React.memo(function TeamMessageBubble({
  from,
  to,
  content,
  fromColor,
  timestamp,
}: TeamMessageBubbleProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [needsCollapse, setNeedsCollapse] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (contentRef.current) {
      setNeedsCollapse(contentRef.current.scrollHeight > COLLAPSED_MAX_HEIGHT);
    }
  }, [content]);

  const toggle = useCallback(() => setIsExpanded((prev) => !prev), []);

  return (
    <div className="flex min-w-0 mb-3 justify-start">
      <div
        className="min-w-0 w-full rounded-lg px-3 py-2"
        style={{
          backgroundColor: BG_COLOR,
          border: "1px solid hsl(220 10% 15%)",
        }}
      >
        {/* Header: from → to */}
        <div className="flex items-center gap-1.5 mb-1">
          {fromColor && (
            <span
              className="w-2 h-2 rounded-full shrink-0"
              style={{ backgroundColor: fromColor }}
            />
          )}
          <span className="text-[11px] font-medium" style={{ color: "hsl(220 10% 60%)" }}>
            {from}
          </span>
          <span className="text-[11px]" style={{ color: "hsl(220 10% 40%)" }}>→</span>
          <span className="text-[11px] font-medium" style={{ color: "hsl(220 10% 60%)" }}>
            {to}
          </span>
          {timestamp && (
            <span className="text-[10px] ml-auto" style={{ color: "hsl(220 10% 35%)" }}>
              {new Date(timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
            </span>
          )}
        </div>
        {/* Content — heading sizes clamped for compact bubble layout */}
        <div
          style={{
            position: "relative",
            maxHeight: !isExpanded && needsCollapse ? COLLAPSED_MAX_HEIGHT : undefined,
            overflow: !isExpanded && needsCollapse ? "hidden" : undefined,
            transition: "max-height 200ms ease",
          }}
        >
          <div
            ref={contentRef}
            className="text-[12px] [&_h1]:text-[14px] [&_h2]:text-[13px] [&_h3]:text-[12px]"
            style={{ color: "hsl(220 10% 70%)" }}
          >
            <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
              {content}
            </ReactMarkdown>
          </div>
          {/* Gradient fade when collapsed */}
          {needsCollapse && !isExpanded && (
            <div
              style={{
                position: "absolute",
                bottom: 0,
                left: 0,
                right: 0,
                height: GRADIENT_HEIGHT,
                background: `linear-gradient(to bottom, transparent, ${BG_COLOR})`,
                pointerEvents: "none",
              }}
            />
          )}
        </div>
        {/* Show more / Show less toggle */}
        {needsCollapse && (
          <button
            onClick={toggle}
            style={{
              background: "none",
              border: "none",
              padding: "2px 0 0",
              cursor: "pointer",
              fontSize: 10.5,
              color: "hsl(220 10% 45%)",
            }}
            onMouseEnter={(e) => { e.currentTarget.style.color = "hsl(220 10% 65%)"; }}
            onMouseLeave={(e) => { e.currentTarget.style.color = "hsl(220 10% 45%)"; }}
          >
            {isExpanded ? "Show less" : "Show more"}
          </button>
        )}
      </div>
    </div>
  );
});
