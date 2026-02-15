/**
 * TeammatePaneGrid — Right column container for teammate panes
 *
 * CSS Grid layout with equal-height rows for each teammate.
 * 1px gap between panes, scrollable when many teammates.
 */

import React, { useMemo } from "react";
import { useTeamStore, selectTeammates } from "@/stores/teamStore";
import { TeammatePane } from "./TeammatePane";
import type { TeammateState } from "@/stores/teamStore";

interface TeammatePaneGridProps {
  contextKey: string;
  onStopTeammate?: ((name: string) => void) | undefined;
  onSendMessage?: ((name: string, content: string) => void) | undefined;
}

export const TeammatePaneGrid = React.memo(function TeammatePaneGrid({
  contextKey,
  onStopTeammate,
  onSendMessage,
}: TeammatePaneGridProps) {
  const teammatesSelector = useMemo(() => selectTeammates(contextKey), [contextKey]);
  const teammates = useTeamStore(teammatesSelector);

  if (teammates.length === 0) {
    return (
      <div
        className="flex items-center justify-center h-full"
        style={{ backgroundColor: "hsl(220 10% 7%)" }}
      >
        <p className="text-[12px]" style={{ color: "hsl(220 10% 40%)" }}>
          No teammates spawned yet
        </p>
      </div>
    );
  }

  return (
    <div
      className="h-full overflow-y-auto"
      style={{
        display: "grid",
        gridTemplateRows: `repeat(${teammates.length}, 1fr)`,
        gap: 1,
        backgroundColor: "hsl(220 10% 14%)",
      }}
    >
      {teammates.map((mate: TeammateState) => (
        <TeammatePane
          key={mate.name}
          contextKey={contextKey}
          teammateName={mate.name}
          onStop={onStopTeammate}
          onSendMessage={onSendMessage}
        />
      ))}
    </div>
  );
});
