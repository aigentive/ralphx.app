/**
 * TaskCard utility functions
 *
 * Contains styling helpers extracted from TaskCard.tsx to reduce component size.
 * Design: macOS Tahoe (2025) - Clean, flat, minimal like Finder
 */

import { type InternalStatus, NON_DRAGGABLE_STATUSES } from "@/types/status";
import { getStatusBorderStyles } from "@/types/status-icons";

/**
 * Priority colors - simple, flat
 */
const PRIORITY_COLORS: Record<number, string> = {
  1: "hsl(0 70% 55%)",      // Critical - Red
  2: "hsl(25 90% 55%)",     // High - Orange
  3: "hsl(14 100% 60%)",    // Medium - Accent orange
  4: "hsl(220 10% 35%)",    // Low - Gray
};

/**
 * Get priority color for the left border stripe
 */
export function getPriorityColor(priority: number, isArchived: boolean): string {
  if (isArchived) return "hsl(220 10% 25%)";
  return PRIORITY_COLORS[priority] ?? "transparent";
}

/**
 * Build base card styles (macOS Tahoe - subtle floating elevation)
 * Content cards get light elevation to distinguish them as distinct items.
 */
export function getBaseCardStyles(
  priority: number,
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
    borderLeft: `3px solid ${getPriorityColor(priority, isArchived)}`,
  };
}

/**
 * Get card styles based on state (dragging, selected, default)
 */
export function getCardStyles(
  priority: number,
  isArchived: boolean,
  isDragging: boolean,
  isDraggable: boolean,
  isSelected: boolean
): React.CSSProperties {
  const baseStyles = getBaseCardStyles(priority, isArchived, isDraggable);

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
 * Get execution state class name for CSS animations
 */
export function getExecutionStateClass(status: InternalStatus): string {
  switch (status) {
    case "executing":
      return "task-card-executing";
    case "revision_needed":
    case "re_executing":
      return "task-card-revision";
    case "reviewing":
      return "task-card-reviewing";
    case "review_passed":
      return "task-card-review-passed";
    default:
      return "";
  }
}

/**
 * Get execution/review state border styles
 * Uses shared STATUS_ICON_CONFIG for consistent colors with icons
 */
export function getExecutionBorderStyles(status: InternalStatus): React.CSSProperties {
  return getStatusBorderStyles(status);
}

/**
 * Check if a task with given status is draggable
 * Uses centralized NON_DRAGGABLE_STATUSES from @/types/status
 */
export function isDraggableStatus(status: InternalStatus): boolean {
  return !(NON_DRAGGABLE_STATUSES as readonly string[]).includes(status);
}
