/**
 * TeamFilterTabs — Per-teammate message filter tabs
 *
 * Dynamic tabs from active team's member list:
 * [All] [Lead] [coder-1] [coder-2] ...
 * Color indicator dots per teammate.
 */

import React from "react";
import { cn } from "@/lib/utils";
import type { TeammateState, TeammateStatus } from "@/stores/teamStore";

export type TeamFilterValue = "all" | "lead" | string;

const STATUS_DOT_COLORS: Record<TeammateStatus, string> = {
  running: "hsl(142 71% 45%)",
  spawning: "hsl(142 71% 45%)",
  idle: "hsl(48 96% 53%)",
  completed: "hsl(220 10% 40%)",
  shutdown: "hsl(220 10% 40%)",
  failed: "hsl(220 10% 40%)",
};

interface TeamFilterTabsProps {
  teammates: TeammateState[];
  activeFilter: TeamFilterValue;
  onFilterChange: (filter: TeamFilterValue) => void;
}

export const TeamFilterTabs = React.memo(function TeamFilterTabs({
  teammates,
  activeFilter,
  onFilterChange,
}: TeamFilterTabsProps) {
  return (
    <div
      className="flex items-center gap-1 px-3 py-1.5 overflow-x-auto shrink-0"
      style={{
        borderTop: "1px solid hsl(220 10% 14%)",
      }}
    >
      {/* All tab */}
      <FilterChip
        label="All"
        isActive={activeFilter === "all"}
        onClick={() => onFilterChange("all")}
      />
      {/* Lead tab */}
      <FilterChip
        label="Lead"
        isActive={activeFilter === "lead"}
        onClick={() => onFilterChange("lead")}
      />
      {/* Teammate tabs */}
      {teammates.map((mate) => (
        <FilterChip
          key={mate.name}
          label={mate.name}
          color={STATUS_DOT_COLORS[mate.status]}
          isActive={activeFilter === mate.name}
          onClick={() => onFilterChange(mate.name)}
        />
      ))}
    </div>
  );
});

// ============================================================================
// FilterChip
// ============================================================================

interface FilterChipProps {
  label: string;
  color?: string;
  isActive: boolean;
  onClick: () => void;
}

function FilterChip({ label, color, isActive, onClick }: FilterChipProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex items-center gap-1.5 px-2 py-0.5 rounded text-[11px] whitespace-nowrap transition-colors",
        "outline-none focus-visible:ring-1",
      )}
      style={{
        backgroundColor: isActive ? "hsl(220 10% 18%)" : "transparent",
        color: isActive ? "hsl(220 10% 85%)" : "hsl(220 10% 50%)",
        ...(isActive ? { outline: "1px solid hsla(220 80% 60% / 0.5)" } : {}),
      }}
    >
      {color && (
        <span
          className="w-1.5 h-1.5 rounded-full shrink-0"
          style={{ backgroundColor: color }}
        />
      )}
      {label}
    </button>
  );
}
