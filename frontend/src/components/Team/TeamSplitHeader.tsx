/**
 * TeamSplitHeader — Header bar for the team split view
 *
 * Shows back button, team name, active count badge, aggregate cost, and stop all button.
 */

import React, { useMemo } from "react";
import { ChevronLeft, Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTeamStore, selectActiveTeam, selectTeammates } from "@/stores/teamStore";
import { useUiStore } from "@/stores/uiStore";
import type { TeammateState } from "@/stores/teamStore";

interface TeamSplitHeaderProps {
  contextKey: string;
  onStopAll?: (() => void) | undefined;
}

export const TeamSplitHeader = React.memo(function TeamSplitHeader({
  contextKey,
  onStopAll,
}: TeamSplitHeaderProps) {
  const teamSelector = useMemo(() => selectActiveTeam(contextKey), [contextKey]);
  const team = useTeamStore(teamSelector);
  const teammatesSelector = useMemo(() => selectTeammates(contextKey), [contextKey]);
  const teammates = useTeamStore(teammatesSelector);

  const previousView = useUiStore((s) => s.previousView);
  const setCurrentView = useUiStore((s) => s.setCurrentView);
  const setPreviousView = useUiStore((s) => s.setPreviousView);

  const activeCount = useMemo(
    () => teammates.filter((m: TeammateState) => m.status !== "shutdown").length,
    [teammates],
  );

  const handleBack = () => {
    const target = previousView ?? "kanban";
    setPreviousView(null);
    setCurrentView(target);
  };

  const costLabel = team
    ? `$${team.totalEstimatedCostUsd.toFixed(2)}`
    : "$0.00";

  return (
    <div
      className="flex items-center justify-between px-4 shrink-0"
      style={{
        height: 48,
        backgroundColor: "hsl(220 10% 9%)",
        borderBottom: "1px solid hsl(220 10% 14%)",
        backdropFilter: "blur(24px)",
        WebkitBackdropFilter: "blur(24px)",
      }}
    >
      {/* Left: Back + Team Name + Badge */}
      <div className="flex items-center gap-3">
        <Button
          variant="ghost"
          size="sm"
          onClick={handleBack}
          className="gap-1.5 h-7 px-2 text-[13px]"
          style={{ color: "hsl(220 10% 60%)" }}
        >
          <ChevronLeft className="w-4 h-4" />
          Back
        </Button>

        <span
          className="text-[13px] font-medium"
          style={{ color: "hsl(220 10% 85%)" }}
        >
          {team?.teamName ?? "Team"}
        </span>

        <span
          className="text-[10px] px-1.5 py-0.5 rounded"
          style={{
            backgroundColor: "hsl(220 10% 14%)",
            color: "hsl(220 10% 55%)",
          }}
        >
          {activeCount}/{teammates.length} active
        </span>

        <span
          className="text-[11px] font-mono"
          style={{ color: "hsl(220 10% 45%)" }}
        >
          {costLabel}
        </span>
      </div>

      {/* Right: Stop All */}
      {onStopAll && activeCount > 0 && (
        <Button
          variant="ghost"
          size="sm"
          onClick={onStopAll}
          className="gap-1.5 h-7 text-[11px]"
          style={{ color: "hsl(0 70% 60%)" }}
        >
          <Square className="w-3 h-3" />
          Stop All
        </Button>
      )}
    </div>
  );
});
