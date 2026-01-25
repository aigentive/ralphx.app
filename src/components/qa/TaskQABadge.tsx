/**
 * TaskQABadge - Shows QA status on task cards
 *
 * Premium design with shadcn Badge and Lucide icons:
 * - pending: muted background with Clock icon
 * - preparing: amber/warning with spinning Loader2
 * - ready: blue/info with CheckCircle
 * - testing: accent with spinning Loader2
 * - passed: emerald/success with CheckCircle
 * - failed: red/error with XCircle
 * - skipped: muted with MinusCircle
 */

import { Clock, Loader2, CheckCircle, XCircle, MinusCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
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
  /** Use compact (icon-only) variant with tooltip */
  compact?: boolean;
}

/** Configuration for each display status */
interface StatusConfig {
  label: string;
  /** Tailwind classes for the badge background and text */
  colorClass: string;
  /** Lucide icon component */
  Icon: React.ComponentType<{ className?: string }>;
  /** Whether icon should spin */
  spin?: boolean;
}

const STATUS_CONFIG: Record<QADisplayStatus, StatusConfig> = {
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

/**
 * TaskQABadge component
 *
 * Displays QA status with appropriate color, icon, and label.
 * Hidden when task doesn't need QA.
 * Supports compact mode (icon-only with tooltip).
 */
export function TaskQABadge({
  needsQA,
  prepStatus,
  testStatus,
  className = "",
  compact = false,
}: TaskQABadgeProps) {
  // Don't render if task doesn't need QA
  if (!needsQA) {
    return null;
  }

  const displayStatus = deriveQADisplayStatus(prepStatus, testStatus);
  const config = STATUS_CONFIG[displayStatus];
  const { Icon, spin, label, colorClass } = config;

  // Compact variant: icon-only with tooltip
  if (compact) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Badge
              data-testid="task-qa-badge"
              data-status={displayStatus}
              variant="outline"
              className={cn(
                "p-1 border-0",
                colorClass,
                className
              )}
            >
              <Icon className={cn("w-3.5 h-3.5", spin && "animate-spin")} />
            </Badge>
          </TooltipTrigger>
          <TooltipContent>
            <p>{label}</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  // Full variant: icon + label
  return (
    <Badge
      data-testid="task-qa-badge"
      data-status={displayStatus}
      variant="outline"
      className={cn(
        "inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium border-0",
        colorClass,
        className
      )}
    >
      <Icon className={cn("w-3 h-3", spin && "animate-spin")} />
      {label}
    </Badge>
  );
}
