/**
 * ResizeHandle - Shared resize handle component for split layouts
 *
 * Features:
 * - 12px wide hit area for easy grabbing
 * - 2px visible line (subtle gray default)
 * - Orange highlight on hover
 * - Orange highlight while dragging
 */

import { memo } from "react";

interface ResizeHandleProps {
  /** Whether currently resizing */
  isResizing: boolean;
  /** Mouse down handler to start resizing */
  onMouseDown: (e: React.MouseEvent) => void;
  /** Test ID for testing */
  testId?: string;
}

export const ResizeHandle = memo(function ResizeHandle({
  isResizing,
  onMouseDown,
  testId = "resize-handle",
}: ResizeHandleProps) {
  return (
    <div
      data-testid={testId}
      className="shrink-0 cursor-ew-resize flex items-center justify-center group relative z-10"
      style={{ width: "12px", marginLeft: "-6px", marginRight: "-6px" }}
      onMouseDown={onMouseDown}
    >
      <div
        className={`h-full transition-all duration-150 ${
          isResizing
            ? "w-[3px] bg-[hsl(14_100%_55%)]"
            : "w-[2px] bg-[hsla(220_20%_100%_/_0.15)] group-hover:w-[3px] group-hover:bg-[hsl(14_100%_55%)]"
        }`}
      />
    </div>
  );
});

/**
 * Static separator line (non-interactive)
 * Used when resize is not available (e.g., timeline in graph view)
 */
export const SeparatorLine = memo(function SeparatorLine() {
  return (
    <div className="shrink-0 w-px bg-[hsla(220_20%_100%_/_0.04)]" />
  );
});
