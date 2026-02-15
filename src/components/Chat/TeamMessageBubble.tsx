/**
 * TeamMessageBubble — Inter-agent message display
 *
 * Shows messages between teammates: "from → to: content"
 * Uses a dimmed background style to distinguish from regular chat messages.
 */

import React from "react";

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
  return (
    <div className="flex min-w-0 mb-3 justify-start">
      <div
        className="min-w-0 w-full rounded-lg px-3 py-2"
        style={{
          backgroundColor: "hsl(220 10% 11%)",
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
        {/* Content */}
        <p className="text-[12px] leading-relaxed" style={{ color: "hsl(220 10% 70%)" }}>
          {content}
        </p>
      </div>
    </div>
  );
});
