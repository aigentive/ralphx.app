/**
 * status-icons.ts - Shared status styling configuration
 *
 * Single source of truth for status colors, icons, and borders used by:
 * - TaskCard (Kanban board) - icons and borders
 * - TaskNode (Graph view) - icons
 * - TaskStatusBadge - icons
 *
 * Each status gets: icon, color, background opacity, label, and optional border highlight
 */

import {
  Clock,
  Play,
  Ban,
  Loader2,
  RotateCcw,
  CheckCircle,
  AlertTriangle,
  AlertCircle,
  GitMerge,
  GitPullRequest,
  XCircle,
  XOctagon,
  Archive,
  FileText,
} from "lucide-react";
import type { InternalStatus } from "./status";

// ============================================================================
// Types
// ============================================================================

export interface StatusIconConfig {
  /** Lucide icon component */
  icon: React.ComponentType<{ className?: string }>;
  /** Icon/text color (HSL or CSS variable) - used for icons AND left border */
  color: string;
  /** Background opacity (0-1 as string, e.g., "0.2") */
  bgOpacity: string;
  /** Short display label */
  label: string;
  /** Whether icon should spin (for active states) */
  animate?: boolean;
}

// ============================================================================
// Status Icon Configuration
// ============================================================================

export const STATUS_ICON_CONFIG: Record<InternalStatus, StatusIconConfig> = {
  // === Idle States ===
  backlog: {
    icon: FileText,
    color: "hsl(220 10% 55%)",
    bgOpacity: "0.15",
    label: "Draft",
  },
  ready: {
    icon: Play,
    color: "hsl(220 80% 60%)",
    bgOpacity: "0.15",
    label: "Ready",
  },

  // === Blocked ===
  blocked: {
    icon: Ban,
    color: "hsl(45 90% 55%)",
    bgOpacity: "0.2",
    label: "Blocked",
  },

  // === Execution States ===
  executing: {
    icon: Loader2,
    color: "hsl(14 100% 55%)",
    bgOpacity: "0.2",
    label: "Executing",
    animate: true,
  },
  re_executing: {
    icon: Loader2,
    color: "hsl(45 90% 55%)",
    bgOpacity: "0.2",
    label: "Revising",
    animate: true,
  },

  // === QA States ===
  qa_refining: {
    icon: Loader2,
    color: "hsl(280 60% 55%)",
    bgOpacity: "0.2",
    label: "QA",
    animate: true,
  },
  qa_testing: {
    icon: Loader2,
    color: "hsl(280 60% 55%)",
    bgOpacity: "0.2",
    label: "Testing",
    animate: true,
  },
  qa_passed: {
    icon: CheckCircle,
    color: "hsl(280 60% 55%)",
    bgOpacity: "0.2",
    label: "QA ✓",
  },
  qa_failed: {
    icon: XCircle,
    color: "hsl(280 60% 55%)",
    bgOpacity: "0.2",
    label: "QA ✗",
  },

  // === Review States ===
  pending_review: {
    icon: Clock,
    color: "hsl(220 80% 60%)",
    bgOpacity: "0.2",
    label: "Pending",
  },
  reviewing: {
    icon: Loader2,
    color: "hsl(220 80% 60%)",
    bgOpacity: "0.2",
    label: "Reviewing",
    animate: true,
  },
  review_passed: {
    icon: CheckCircle,
    color: "hsl(145 60% 45%)",
    bgOpacity: "0.2",
    label: "Approved",
  },
  escalated: {
    icon: AlertTriangle,
    color: "hsl(45 90% 55%)",
    bgOpacity: "0.2",
    label: "Escalated",
  },
  revision_needed: {
    icon: RotateCcw,
    color: "hsl(45 90% 55%)",
    bgOpacity: "0.2",
    label: "Revision",
  },

  // === Merge States ===
  pending_merge: {
    icon: GitPullRequest,
    color: "hsl(180 60% 50%)",
    bgOpacity: "0.2",
    label: "Merge",
  },
  merging: {
    icon: Loader2,
    color: "hsl(180 60% 50%)",
    bgOpacity: "0.2",
    label: "Merging",
    animate: true,
  },
  merge_conflict: {
    icon: AlertCircle,
    color: "hsl(0 70% 55%)",
    bgOpacity: "0.2",
    label: "Conflict",
  },

  // === Complete States ===
  approved: {
    icon: CheckCircle,
    color: "hsl(145 60% 45%)",
    bgOpacity: "0.2",
    label: "Done",
  },
  merged: {
    icon: GitMerge,
    color: "hsl(145 60% 45%)",
    bgOpacity: "0.2",
    label: "Merged",
  },

  // === Terminal States ===
  failed: {
    icon: XOctagon,
    color: "hsl(0 70% 55%)",
    bgOpacity: "0.2",
    label: "Failed",
  },
  cancelled: {
    icon: XCircle,
    color: "hsl(220 10% 50%)",
    bgOpacity: "0.15",
    label: "Cancelled",
  },
};

// ============================================================================
// Special Icon Configs (not tied to InternalStatus)
// ============================================================================

/** Archived task icon config */
export const ARCHIVED_ICON_CONFIG: StatusIconConfig = {
  icon: Archive,
  color: "hsl(220 10% 45%)",
  bgOpacity: "0.1",
  label: "Archived",
};

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Get icon config for a status with fallback
 */
export function getStatusIconConfig(status: InternalStatus | string): StatusIconConfig {
  const config = STATUS_ICON_CONFIG[status as InternalStatus];
  if (config) return config;

  // Fallback for unknown statuses
  return {
    icon: Clock,
    color: "hsl(220 10% 55%)",
    bgOpacity: "0.15",
    label: status,
  };
}

/**
 * Check if a status icon should animate (spin)
 */
export function shouldAnimateIcon(status: InternalStatus | string): boolean {
  const config = STATUS_ICON_CONFIG[status as InternalStatus];
  return config?.animate ?? false;
}
