/**
 * Configuration constants and utilities for TaskQABadge component
 */

import { Clock, Loader2, CheckCircle, XCircle, MinusCircle } from "lucide-react";
import type { QAOverallStatus } from "@/types/qa";
import type { QAPrepStatus } from "@/types/qa-config";

/** Combined QA display status for the badge */
export type QADisplayStatus =
  | "pending"
  | "preparing"
  | "ready"
  | "testing"
  | "passed"
  | "failed"
  | "skipped";

/** Configuration for each display status */
export interface StatusConfig {
  label: string;
  /** Tailwind classes for the badge background and text */
  colorClass: string;
  /** Lucide icon component */
  Icon: React.ComponentType<{ className?: string }>;
  /** Whether icon should spin */
  spin?: boolean;
}

export const STATUS_CONFIG: Record<QADisplayStatus, StatusConfig> = {
  pending: {
    label: "QA Pending",
    colorClass: "bg-[var(--bg-hover)] text-[var(--text-muted)]",
    Icon: Clock,
  },
  preparing: {
    label: "Preparing",
    colorClass: "bg-amber-500/15 text-[var(--status-warning)]",
    Icon: Loader2,
    spin: true,
  },
  ready: {
    label: "QA Ready",
    colorClass: "bg-blue-500/15 text-[var(--status-info)]",
    Icon: CheckCircle,
  },
  testing: {
    label: "Testing",
    colorClass: "bg-[var(--accent-muted)] text-[var(--accent-primary)]",
    Icon: Loader2,
    spin: true,
  },
  passed: {
    label: "Passed",
    colorClass: "bg-emerald-500/15 text-[var(--status-success)]",
    Icon: CheckCircle,
  },
  failed: {
    label: "Failed",
    colorClass: "bg-red-500/15 text-[var(--status-error)]",
    Icon: XCircle,
  },
  skipped: {
    label: "Skipped",
    colorClass: "bg-[var(--bg-hover)] text-[var(--text-muted)]",
    Icon: MinusCircle,
  },
};

/**
 * Derive the display status from prep and test status
 */
export function deriveQADisplayStatus(
  prepStatus?: QAPrepStatus,
  testStatus?: QAOverallStatus
): QADisplayStatus {
  // If we have test status, prioritize it
  if (testStatus) {
    switch (testStatus) {
      case "passed":
        return "passed";
      case "failed":
        return "failed";
      case "running":
        return "testing";
      case "pending":
        // Fall through to check prep status
        break;
    }
  }

  // Check prep status
  if (prepStatus) {
    switch (prepStatus) {
      case "running":
        return "preparing";
      case "completed":
        return "ready";
      case "failed":
        return "failed";
      case "pending":
        return "pending";
    }
  }

  return "pending";
}
