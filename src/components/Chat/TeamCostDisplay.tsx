/**
 * TeamCostDisplay — Aggregate + per-teammate token/cost breakdown
 *
 * Shows total team cost and individual teammate token usage.
 */

import React from "react";
import type { TeammateState } from "@/stores/teamStore";

interface TeamCostDisplayProps {
  totalTokens: number;
  totalEstimatedCostUsd: number;
  teammates: TeammateState[];
}

function formatCost(usd: number): string {
  return usd < 0.01 ? "<$0.01" : `$${usd.toFixed(2)}`;
}

function formatTokens(tokens: number): string {
  if (tokens < 1000) return `${tokens}`;
  return `~${Math.round(tokens / 1000)}K`;
}

export const TeamCostDisplay = React.memo(function TeamCostDisplay({
  totalTokens,
  totalEstimatedCostUsd,
  teammates,
}: TeamCostDisplayProps) {
  return (
    <div className="px-3 py-2">
      {/* Aggregate */}
      <div className="flex items-center justify-between mb-2">
        <span className="text-[11px] font-medium" style={{ color: "hsl(220 10% 50%)" }}>
          Total
        </span>
        <span className="text-[11px]" style={{ color: "hsl(220 10% 60%)" }}>
          {formatTokens(totalTokens)} tokens | {formatCost(totalEstimatedCostUsd)}
        </span>
      </div>
      {/* Per-teammate breakdown */}
      {teammates.map((mate) => (
        <div key={mate.name} className="flex items-center justify-between py-0.5">
          <div className="flex items-center gap-1.5">
            <span
              className="w-1.5 h-1.5 rounded-full shrink-0"
              style={{ backgroundColor: mate.color }}
            />
            <span className="text-[10px]" style={{ color: "hsl(220 10% 45%)" }}>
              {mate.name}
            </span>
          </div>
          <span className="text-[10px]" style={{ color: "hsl(220 10% 45%)" }}>
            {formatTokens(mate.tokensUsed)} | {formatCost(mate.estimatedCostUsd)}
          </span>
        </div>
      ))}
    </div>
  );
});
