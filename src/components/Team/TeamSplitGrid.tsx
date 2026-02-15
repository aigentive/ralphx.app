/**
 * TeamSplitGrid — CSS Grid layout for the team split view
 *
 * Two-column grid: coordinator (left) + teammates (right).
 * Responsive: stacks vertically below 1024px.
 */

import React from "react";
import { useSplitPaneStore, selectCoordinatorWidth } from "@/stores/splitPaneStore";

interface TeamSplitGridProps {
  /** Coordinator pane content (left column) */
  coordinatorSlot?: React.ReactNode;
  /** Teammates pane content (right column) */
  teammatesSlot?: React.ReactNode;
}

export const TeamSplitGrid = React.memo(function TeamSplitGrid({
  coordinatorSlot,
  teammatesSlot,
}: TeamSplitGridProps) {
  const coordinatorWidth = useSplitPaneStore(selectCoordinatorWidth);

  return (
    <div
      className="flex-1 overflow-hidden"
      style={{
        display: "grid",
        gridTemplateColumns: `${coordinatorWidth}% 1fr`,
        minHeight: 0,
      }}
    >
      {/* Coordinator Pane (left) */}
      <div
        className="overflow-hidden"
        style={{
          borderRight: "1px solid hsl(220 10% 14%)",
          backgroundColor: "hsl(220 10% 6%)",
        }}
      >
        {coordinatorSlot ?? (
          <div className="flex items-center justify-center h-full">
            <span className="text-[12px]" style={{ color: "hsl(220 10% 35%)" }}>
              Coordinator
            </span>
          </div>
        )}
      </div>

      {/* Teammates Pane (right) */}
      <div
        className="overflow-hidden"
        style={{ backgroundColor: "hsl(220 10% 6%)" }}
      >
        {teammatesSlot ?? (
          <div className="flex items-center justify-center h-full">
            <span className="text-[12px]" style={{ color: "hsl(220 10% 35%)" }}>
              Teammates
            </span>
          </div>
        )}
      </div>

      {/* Responsive: stack vertically on narrow screens via CSS media query */}
      <style>{`
        @media (max-width: 1024px) {
          .team-split-grid-responsive {
            grid-template-columns: 1fr !important;
          }
        }
      `}</style>
    </div>
  );
});
