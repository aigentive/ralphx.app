/**
 * TaskCard utility functions
 *
 * Contains styling helpers extracted from TaskCard.tsx to reduce component size.
 * Design: macOS Tahoe (2025) - Clean, flat, minimal like Finder
 */

import { type InternalStatus, NON_DRAGGABLE_STATUSES } from "@/types/status";
import { getStatusIconConfig, ARCHIVED_ICON_CONFIG } from "@/types/status-icons";

/**
 * Get status color for the left border stripe
 * Uses shared STATUS_ICON_CONFIG for consistency with icons
 */
export function getStatusColor(status: InternalStatus, isArchived: boolean): string {
  if (isArchived) return ARCHIVED_ICON_CONFIG.color;
  return getStatusIconConfig(status).color;
}

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
    background: "hsla(220 10% 14% / 0.85)",
    backdropFilter: "blur(12px) saturate(150%)",
    WebkitBackdropFilter: "blur(12px) saturate(150%)",
    border: "1px solid hsla(220 10% 100% / 0.06)",
    boxShadow: "0 2px 8px hsla(220 10% 0% / 0.25)",
    borderLeft: `3px solid ${getStatusColor(status, isArchived)}`,
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
      background: "hsla(220 10% 18% / 0.95)",
      boxShadow: "0 8px 24px hsla(220 10% 0% / 0.4), 0 2px 8px hsla(220 10% 0% / 0.3)",
      zIndex: 50,
    };
  }

  // Selected state - subtle blue tint like Finder selection
  if (isSelected) {
    return {
      ...baseStyles,
      background: "hsla(220 60% 50% / 0.25)",
      border: "1px solid hsla(220 60% 60% / 0.3)",
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
