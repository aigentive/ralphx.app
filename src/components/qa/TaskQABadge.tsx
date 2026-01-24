/**
 * TaskQABadge - Shows QA status on task cards
 *
 * Displays QA status with color coding:
 * - pending: gray (--text-muted)
 * - preparing: yellow (--status-warning)
 * - ready: blue (--status-info)
 * - testing: purple (--accent-secondary)
 * - passed: green (--status-success)
 * - failed: red (--status-error)
 */

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

interface TaskQABadgeProps {
  /** Whether this task needs QA */
  needsQA: boolean;
  /** QA prep status */
  prepStatus?: QAPrepStatus;
  /** Overall QA test status */
  testStatus?: QAOverallStatus;
  /** Optional className for additional styling */
  className?: string;
}

/** Configuration for each display status */
const STATUS_CONFIG: Record<
  QADisplayStatus,
  { label: string; colorClass: string }
> = {
  pending: {
    label: "QA Pending",
    colorClass: "bg-[--text-muted] text-[--bg-base]",
  },
  preparing: {
    label: "Preparing",
    colorClass: "bg-[--status-warning] text-[--bg-base]",
  },
  ready: {
    label: "QA Ready",
    colorClass: "bg-[--status-info] text-[--bg-base]",
  },
  testing: {
    label: "Testing",
    colorClass: "bg-[--accent-secondary] text-[--bg-base]",
  },
  passed: {
    label: "Passed",
    colorClass: "bg-[--status-success] text-[--bg-base]",
  },
  failed: {
    label: "Failed",
    colorClass: "bg-[--status-error] text-[--bg-base]",
  },
  skipped: {
    label: "Skipped",
    colorClass: "bg-[--text-muted] text-[--bg-base]",
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

/**
 * TaskQABadge component
 *
 * Displays QA status with appropriate color and label.
 * Hidden when task doesn't need QA.
 */
export function TaskQABadge({
  needsQA,
  prepStatus,
  testStatus,
  className = "",
}: TaskQABadgeProps) {
  // Don't render if task doesn't need QA
  if (!needsQA) {
    return null;
  }

  const displayStatus = deriveQADisplayStatus(prepStatus, testStatus);
  const config = STATUS_CONFIG[displayStatus];

  return (
    <span
      data-testid="task-qa-badge"
      data-status={displayStatus}
      className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${config.colorClass} ${className}`}
    >
      {config.label}
    </span>
  );
}
