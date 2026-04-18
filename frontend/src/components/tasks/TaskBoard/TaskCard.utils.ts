/**
 * TaskCard utility functions
 *
 * Contains styling helpers extracted from TaskCard.tsx to reduce component size.
 * Design: macOS Tahoe (2025) - Clean, flat, minimal like Finder
 */

import { type InternalStatus, NON_DRAGGABLE_STATUSES } from "@/types/status";
import { getStatusBorderColor } from "@/types/status-icons";

/**
 * Build base card styles (macOS Tahoe - subtle floating elevation)
 * Content cards get light elevation to distinguish them as distinct items.
 * Left border color is determined by task status.
 */
export function getBaseCardStyles(
  status: InternalStatus,
  isArchived: boolean,
  isDraggable: boolean
): React.CSSProperties {
  return {
    cursor: isDraggable ? "grab" : "default",
    transition: "background 150ms ease, transform 150ms ease, box-shadow 150ms ease",
    borderRadius: "8px",
    background: "var(--bg-surface)",
    border: "1px solid var(--overlay-moderate)",
    boxShadow: "0 4px 16px hsla(220 20% 0% / 0.28)",
    borderLeft: `3px solid ${getStatusBorderColor(status, isArchived)}`,
  };
}

/**
 * Get card styles based on state (dragging, selected, default)
 */
export function getCardStyles(
  status: InternalStatus,
  isArchived: boolean,
  isDragging: boolean,
  isDraggable: boolean,
  isSelected: boolean
): React.CSSProperties {
  const baseStyles = getBaseCardStyles(status, isArchived, isDraggable);

  if (isDragging) {
    return {
      ...baseStyles,
      cursor: "grabbing",
      transform: "scale(1.02)",
      background: "var(--bg-elevated)",
      boxShadow: "0 8px 24px hsla(220 20% 0% / 0.38), 0 2px 10px hsla(220 20% 0% / 0.3)",
      zIndex: 50,
    };
  }

  // Selected state - subtle blue tint like Finder selection
  if (isSelected) {
    return {
      ...baseStyles,
      background: "color-mix(in srgb, var(--status-info) 24%, transparent)",
      border: "1px solid color-mix(in srgb, var(--status-info) 34%, transparent)",
    };
  }

  return baseStyles;
}

/**
 * Check if a task with given status is draggable
 * Uses centralized NON_DRAGGABLE_STATUSES from @/types/status
 */
export function isDraggableStatus(status: InternalStatus): boolean {
  return !(NON_DRAGGABLE_STATUSES as readonly string[]).includes(status);
}
