/**
 * nodeStyles.ts - Status color mapping for Task Graph nodes
 *
 * Defines border and background colors for all 23 internal status states.
 * Colors are grouped by status category per the visual design spec.
 *
 * Status Groups:
 * - Idle: backlog, ready (muted gray)
 * - Blocked: blocked (amber)
 * - Executing: executing, re_executing (accent orange + glow)
 * - QA: qa_refining, qa_testing, qa_passed, qa_failed (purple)
 * - Review: pending_review, reviewing, review_passed, escalated, revision_needed (blue)
 * - Merge: pending_merge, merging, merge_conflict (cyan)
 * - Complete: approved, merged (green)
 * - Terminal: failed, cancelled (red)
 */

import type { InternalStatus } from "@/types/status";

// ============================================================================
// Types
// ============================================================================

export interface NodeStyle {
  /** Border color (solid HSL value) */
  borderColor: string;
  /** Background color (HSL with alpha) */
  backgroundColor: string;
  /** Optional box-shadow for glow effect */
  boxShadow?: string;
  /** Optional CSS animation for active states */
  animation?: string;
}

/**
 * Glass morphism surface styles for consistent node styling
 */
export interface GlassSurface {
  /** Glass morphism background */
  background: string;
  /** Backdrop filter for blur effect */
  backdropFilter: string;
  /** Webkit prefix for backdrop filter (Safari support) */
  WebkitBackdropFilter: string;
  /** Subtle divider border */
  border: string;
  /** Consistent soft shadow */
  boxShadow: string;
}

// ============================================================================
// Node Dimensions (Single Source of Truth)
// ============================================================================

/** Standard node width - full-size TaskNode with status badges */
export const NODE_WIDTH = 210;

/** Standard node height to accommodate: title + 1-line description + category + progress bar */
export const NODE_HEIGHT = 100;

/** Compact node width - smaller TaskNodeCompact for large graphs (50+ tasks) */
export const COMPACT_NODE_WIDTH = 160;

/** Compact node height - smaller for dense layouts */
export const COMPACT_NODE_HEIGHT = 48;

/** Maximum title characters for compact nodes */
export const COMPACT_TITLE_MAX_CHARS = 20;

// ============================================================================
// Glass Morphism Constants
// ============================================================================

/**
 * Base glass morphism surface - shared by all nodes
 * Matches Kanban card styling for visual consistency
 */
export const GLASS_SURFACE: GlassSurface = {
  background: "hsla(220 10% 14% / 0.85)",
  backdropFilter: "blur(12px) saturate(150%)",
  WebkitBackdropFilter: "blur(12px) saturate(150%)",
  border: "1px solid hsla(220 10% 100% / 0.06)",
  boxShadow: "0 2px 8px hsla(220 10% 0% / 0.25)",
};

// ============================================================================
// Priority Stripe Colors
// ============================================================================

/**
 * Priority colors for left border stripe
 * Matches Kanban card styling (from TaskCard.utils.ts)
 */
const PRIORITY_COLORS: Record<number, string> = {
  1: "hsl(0 70% 55%)",      // P1 Critical - Red
  2: "hsl(25 90% 55%)",     // P2 High - Deep orange
  3: "hsl(14 100% 60%)",    // P3 Medium - Accent orange (#ff6b35)
  4: "hsl(220 10% 35%)",    // P4 Low - Gray
};

/**
 * Get priority color for the left border stripe
 * Falls back to transparent if priority is undefined or out of range
 */
export function getPriorityStripeColor(priority: number | undefined): string {
  if (priority === undefined || priority < 1 || priority > 4) {
    return "transparent";
  }
  return PRIORITY_COLORS[priority] ?? "transparent";
}

export type StatusCategory =
  | "idle"
  | "blocked"
  | "executing"
  | "qa"
  | "review"
  | "merge"
  | "complete"
  | "terminal";

// ============================================================================
// Color Definitions by Category
// ============================================================================

/** Idle statuses: backlog, ready */
const IDLE_COLORS = {
  border: "hsl(220 10% 40%)",
  background: "hsla(220 10% 15% / 0.8)",
} as const;

/** Blocked status: blocked */
const BLOCKED_COLORS = {
  border: "hsl(45 90% 55%)",
  background: "hsla(45 90% 55% / 0.1)",
} as const;

/** Executing statuses: executing, re_executing */
const EXECUTING_COLORS = {
  border: "hsl(14 100% 55%)",
  background: "hsla(14 100% 55% / 0.15)",
  boxShadow: "0 0 12px hsla(14 100% 55% / 0.3)",
  animation: "var(--animation-executing-pulse)",
} as const;

/** QA statuses: qa_refining, qa_testing, qa_passed, qa_failed */
const QA_COLORS = {
  border: "hsl(280 60% 55%)",
  background: "hsla(280 60% 55% / 0.12)",
} as const;

/** Review statuses: pending_review, reviewing, review_passed, escalated, revision_needed */
const REVIEW_COLORS = {
  border: "hsl(220 80% 60%)",
  background: "hsla(220 80% 60% / 0.12)",
} as const;

/** Reviewing status: active review with pulse animation */
const REVIEWING_ANIMATION = "var(--animation-reviewing-pulse)";

/** Merge statuses: pending_merge, merging, merge_conflict */
const MERGE_COLORS = {
  border: "hsl(180 60% 50%)",
  background: "hsla(180 60% 50% / 0.12)",
} as const;

/** Complete statuses: approved, merged */
const COMPLETE_COLORS = {
  border: "hsl(145 60% 45%)",
  background: "hsla(145 60% 45% / 0.12)",
} as const;

/** Terminal statuses: failed, cancelled */
const TERMINAL_COLORS = {
  border: "hsl(0 70% 55%)",
  background: "hsla(0 70% 55% / 0.12)",
} as const;

// ============================================================================
// Status to Category Mapping
// ============================================================================

/**
 * Maps each internal status to its category
 */
export function getStatusCategory(status: InternalStatus): StatusCategory {
  switch (status) {
    case "backlog":
    case "ready":
      return "idle";
    case "blocked":
      return "blocked";
    case "executing":
    case "re_executing":
      return "executing";
    case "qa_refining":
    case "qa_testing":
    case "qa_passed":
    case "qa_failed":
      return "qa";
    case "pending_review":
    case "reviewing":
    case "review_passed":
    case "escalated":
    case "revision_needed":
      return "review";
    case "pending_merge":
    case "merging":
    case "merge_conflict":
      return "merge";
    case "approved":
    case "merged":
      return "complete";
    case "failed":
    case "cancelled":
    case "stopped":
      return "terminal";
    case "paused":
      return "blocked";
  }
}

// ============================================================================
// Style Getters
// ============================================================================

/**
 * Get the complete node style for a given status
 */
export function getNodeStyle(status: InternalStatus | string): NodeStyle {
  // Type guard for unknown status strings (e.g., from API before validation)
  const safeStatus = status as InternalStatus;

  switch (safeStatus) {
    // Idle
    case "backlog":
    case "ready":
      return {
        borderColor: IDLE_COLORS.border,
        backgroundColor: IDLE_COLORS.background,
      };

    // Blocked
    case "blocked":
      return {
        borderColor: BLOCKED_COLORS.border,
        backgroundColor: BLOCKED_COLORS.background,
      };

    // Executing
    case "executing":
    case "re_executing":
      return {
        borderColor: EXECUTING_COLORS.border,
        backgroundColor: EXECUTING_COLORS.background,
        boxShadow: EXECUTING_COLORS.boxShadow,
        animation: EXECUTING_COLORS.animation,
      };

    // QA
    case "qa_refining":
    case "qa_testing":
    case "qa_passed":
    case "qa_failed":
      return {
        borderColor: QA_COLORS.border,
        backgroundColor: QA_COLORS.background,
      };

    // Review
    case "pending_review":
    case "review_passed":
    case "escalated":
    case "revision_needed":
      return {
        borderColor: REVIEW_COLORS.border,
        backgroundColor: REVIEW_COLORS.background,
      };

    // Reviewing (active review with animation)
    case "reviewing":
      return {
        borderColor: REVIEW_COLORS.border,
        backgroundColor: REVIEW_COLORS.background,
        animation: REVIEWING_ANIMATION,
      };

    // Merge
    case "pending_merge":
    case "merging":
    case "merge_conflict":
      return {
        borderColor: MERGE_COLORS.border,
        backgroundColor: MERGE_COLORS.background,
      };

    // Complete
    case "approved":
    case "merged":
      return {
        borderColor: COMPLETE_COLORS.border,
        backgroundColor: COMPLETE_COLORS.background,
      };

    // Terminal
    case "failed":
    case "cancelled":
    case "stopped":
      return {
        borderColor: TERMINAL_COLORS.border,
        backgroundColor: TERMINAL_COLORS.background,
      };

    // Paused (like blocked, amber)
    case "paused":
      return {
        borderColor: BLOCKED_COLORS.border,
        backgroundColor: BLOCKED_COLORS.background,
      };

    // Fallback for unknown statuses
    default:
      return {
        borderColor: IDLE_COLORS.border,
        backgroundColor: IDLE_COLORS.background,
      };
  }
}

/**
 * Get only the border color for a status
 * Useful for MiniMap and edge styling
 */
export function getStatusBorderColor(status: InternalStatus | string): string {
  return getNodeStyle(status).borderColor;
}

/**
 * Get only the background color for a status
 */
export function getStatusBackground(status: InternalStatus | string): string {
  return getNodeStyle(status).backgroundColor;
}

// ============================================================================
// Legend Data (for GraphLegend component)
// ============================================================================

export interface LegendItem {
  status: InternalStatus;
  label: string;
  category: StatusCategory;
}

/**
 * Status groups for the legend, organized by category
 */
export const STATUS_LEGEND_GROUPS: Record<StatusCategory, LegendItem[]> = {
  idle: [
    { status: "backlog", label: "Backlog", category: "idle" },
    { status: "ready", label: "Ready", category: "idle" },
  ],
  blocked: [
    { status: "blocked", label: "Blocked", category: "blocked" },
    { status: "paused", label: "Paused", category: "blocked" },
  ],
  executing: [
    { status: "executing", label: "Executing", category: "executing" },
    { status: "re_executing", label: "Re-executing", category: "executing" },
  ],
  qa: [
    { status: "qa_refining", label: "QA Refining", category: "qa" },
    { status: "qa_testing", label: "QA Testing", category: "qa" },
    { status: "qa_passed", label: "QA Passed", category: "qa" },
    { status: "qa_failed", label: "QA Failed", category: "qa" },
  ],
  review: [
    { status: "pending_review", label: "Pending Review", category: "review" },
    { status: "reviewing", label: "Reviewing", category: "review" },
    { status: "review_passed", label: "Review Passed", category: "review" },
    { status: "escalated", label: "Escalated", category: "review" },
    { status: "revision_needed", label: "Revision Needed", category: "review" },
  ],
  merge: [
    { status: "pending_merge", label: "Pending Merge", category: "merge" },
    { status: "merging", label: "Merging", category: "merge" },
    { status: "merge_conflict", label: "Merge Conflict", category: "merge" },
  ],
  complete: [
    { status: "approved", label: "Approved", category: "complete" },
    { status: "merged", label: "Merged", category: "complete" },
  ],
  terminal: [
    { status: "failed", label: "Failed", category: "terminal" },
    { status: "cancelled", label: "Cancelled", category: "terminal" },
    { status: "stopped", label: "Stopped", category: "terminal" },
  ],
};

/**
 * Category display names for legend headers
 */
export const CATEGORY_LABELS: Record<StatusCategory, string> = {
  idle: "Idle",
  blocked: "Blocked",
  executing: "Executing",
  qa: "QA",
  review: "Review",
  merge: "Merge",
  complete: "Complete",
  terminal: "Terminal",
};

/**
 * Get the border color for a category (for legend group headers)
 */
export function getCategoryColor(category: StatusCategory): string {
  switch (category) {
    case "idle":
      return IDLE_COLORS.border;
    case "blocked":
      return BLOCKED_COLORS.border;
    case "executing":
      return EXECUTING_COLORS.border;
    case "qa":
      return QA_COLORS.border;
    case "review":
      return REVIEW_COLORS.border;
    case "merge":
      return MERGE_COLORS.border;
    case "complete":
      return COMPLETE_COLORS.border;
    case "terminal":
      return TERMINAL_COLORS.border;
  }
}
