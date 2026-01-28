/**
 * TaskCard utility functions
 *
 * Contains styling helpers extracted from TaskCard.tsx to reduce component size.
 */

import type { InternalStatus } from "@/types/status";

/**
 * Get priority color for the left border stripe (Refined Studio aesthetic)
 */
export function getPriorityColor(priority: number, isArchived: boolean): string {
  // Archived tasks always use gray
  if (isArchived) {
    return "#525252"; // neutral-600
  }

  switch (priority) {
    case 1: // Critical
      return "#ef4444"; // red-500
    case 2: // High
      return "#f97316"; // orange-500
    case 3: // Medium
      return "#ff6b35"; // accent-primary
    case 4: // Low
      return "#525252"; // neutral-600
    default: // None or unknown
      return "transparent";
  }
}

/**
 * Build base card styles (macOS Tahoe - Liquid Glass)
 */
export function getBaseCardStyles(
  priority: number,
  isArchived: boolean,
  isDraggable: boolean
): React.CSSProperties {
  return {
    cursor: isDraggable ? "grab" : "default",
    transition: "all 180ms ease-out",
    background: "rgba(255,255,255,0.04)",
    backdropFilter: "blur(20px)",
    WebkitBackdropFilter: "blur(20px)",
    border: "1px solid rgba(255,255,255,0.08)",
    // Priority stripe - must come AFTER border shorthand to override left border
    borderLeft: `3px solid ${getPriorityColor(priority, isArchived)}`,
    boxShadow: "0 1px 3px rgba(0,0,0,0.12)",
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
      boxShadow: "0 12px 32px rgba(0,0,0,0.25)",
      background: "rgba(255,255,255,0.06)",
      zIndex: 50,
    };
  }

  if (isSelected) {
    return {
      ...baseStyles,
      background: "rgba(255,107,53,0.08)",
      borderColor: "rgba(255,107,53,0.25)",
      boxShadow: "0 0 0 1px rgba(255,107,53,0.15), 0 2px 8px rgba(0,0,0,0.15)",
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
 */
export function getExecutionBorderStyles(status: InternalStatus): React.CSSProperties {
  // QA states: pulsing orange border
  if (status.startsWith("qa_")) {
    return {
      borderWidth: "2px",
      borderColor: "var(--accent-primary)",
      animation: "var(--animation-executing-pulse)",
    };
  }
  // Pending review: static amber border
  if (status === "pending_review") {
    return {
      borderWidth: "2px",
      borderColor: "var(--status-warning)",
    };
  }
  // Reviewing: blue pulsing border
  if (status === "reviewing") {
    return {
      borderWidth: "2px",
      borderColor: "var(--status-info)",
      animation: "var(--animation-reviewing-pulse)",
    };
  }
  // Review passed: green accent border
  if (status === "review_passed") {
    return {
      borderWidth: "2px",
      borderColor: "var(--status-success)",
    };
  }
  // Revision needed / Re-executing: orange accent border
  if (status === "revision_needed" || status === "re_executing") {
    return {
      borderWidth: "2px",
      borderColor: "var(--status-warning)",
    };
  }
  return {};
}

/**
 * Statuses that cannot be manually dragged
 */
export const NON_DRAGGABLE_STATUSES: readonly InternalStatus[] = [
  "executing",
  "qa_refining",
  "qa_testing",
  "qa_passed",
  "qa_failed",
  "pending_review",
  "revision_needed",
  "reviewing",
  "review_passed",
  "re_executing",
] as const;

/**
 * Check if a task with given status is draggable
 */
export function isDraggableStatus(status: InternalStatus): boolean {
  return !NON_DRAGGABLE_STATUSES.includes(status);
}

/**
 * Review-related statuses that show the ReviewStateBadge
 */
export const REVIEW_STATE_STATUSES: readonly InternalStatus[] = [
  "revision_needed",
  "pending_review",
  "reviewing",
  "review_passed",
  "re_executing",
] as const;

/**
 * Check if status should show review state badge
 */
export function isReviewStateStatus(status: InternalStatus): boolean {
  return REVIEW_STATE_STATUSES.includes(status);
}

/**
 * Check if status is actively processing (shows activity dots)
 */
export function isActivelyProcessing(status: InternalStatus): boolean {
  return status === "reviewing" || status === "re_executing";
}
