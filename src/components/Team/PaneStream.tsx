/**
 * PaneStream — Streaming text display for a teammate pane
 *
 * Auto-scrolling monospace text with tool call inline badges.
 * Shows streamingText from teamStore for this teammate.
 */

import React, { useRef, useEffect, useMemo } from "react";
import { useTeamStore, selectTeammateByName } from "@/stores/teamStore";

interface PaneStreamProps {
  contextKey: string;
  teammateName: string;
}

import { renderStreamContent } from "./renderStreamContent";

export const PaneStream = React.memo(function PaneStream({
  contextKey,
  teammateName,
}: PaneStreamProps) {
  const mateSelector = useMemo(
    () => selectTeammateByName(contextKey, teammateName),
    [contextKey, teammateName],
  );
  const mate = useTeamStore(mateSelector);
  const scrollRef = useRef<HTMLDivElement>(null);
  const streamingText = mate?.streamingText ?? "";

  // Auto-scroll to bottom on new content
  useEffect(() => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [streamingText]);

  return (
    <div
      ref={scrollRef}
      className="flex-1 overflow-y-auto px-2.5 py-2"
      style={{ backgroundColor: "hsl(220 10% 6%)" }}
    >
      {streamingText ? (
        <pre
          className="text-[11px] leading-relaxed whitespace-pre-wrap break-words m-0"
          style={{
            color: "hsl(220 10% 70%)",
            fontFamily: "SF Mono, ui-monospace, monospace",
          }}
        >
          {renderStreamContent(streamingText)}
        </pre>
      ) : (
        <p
          className="text-[11px] text-center py-4"
          style={{ color: "hsl(220 10% 35%)" }}
        >
          Waiting for output...
        </p>
      )}
    </div>
  );
});
