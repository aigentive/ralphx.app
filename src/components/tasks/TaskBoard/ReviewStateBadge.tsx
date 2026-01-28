/**
 * ReviewStateBadge - Badge component for review-related task states
 *
 * Displays a styled badge indicating the current review state of a task:
 * - revision_needed: Orange badge with retry icon
 * - pending_review: Neutral badge with clock icon
 * - reviewing: Blue badge with animated spinner
 * - review_passed: Green badge with checkmark
 * - re_executing: Orange badge with attempt count
 */

import { Clock, RotateCcw, CheckCircle, Loader2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";

export interface ReviewStateBadgeProps {
  /** The internal status of the task */
  status: string;
  /** Number of revision attempts (optional, for re_executing state) */
  revisionCount?: number | undefined;
}

/**
 * Renders a badge for review-related task states
 * Intended to be positioned in top-right corner of the task card
 */
export function ReviewStateBadge({ status, revisionCount }: ReviewStateBadgeProps) {
  switch (status) {
    case "revision_needed":
      return (
        <Badge
          data-testid="review-state-badge-revision"
          className="text-[9px] px-1.5 py-px flex items-center gap-1"
          style={{
            backgroundColor: "rgba(245, 158, 11, 0.2)", // amber/warning background
            color: "var(--status-warning)",
            border: "none",
          }}
        >
          <RotateCcw className="w-2.5 h-2.5" />
          <span>Revision</span>
        </Badge>
      );

    case "pending_review":
      return (
        <Badge
          data-testid="review-state-badge-pending"
          className="text-[9px] px-1.5 py-px flex items-center gap-1"
          style={{
            backgroundColor: "rgba(255, 255, 255, 0.1)", // neutral background
            color: "var(--text-secondary)",
            border: "none",
          }}
        >
          <Clock className="w-2.5 h-2.5" />
          <span>Pending</span>
        </Badge>
      );

    case "reviewing":
      return (
        <Badge
          data-testid="review-state-badge-reviewing"
          className="text-[9px] px-1.5 py-px flex items-center gap-1 badge-reviewing"
          style={{
            backgroundColor: "rgba(59, 130, 246, 0.2)", // blue background
            color: "var(--status-info)",
            border: "none",
          }}
        >
          <Loader2 className="w-2.5 h-2.5 animate-spin" />
          <span>Reviewing</span>
        </Badge>
      );

    case "review_passed":
      return (
        <Badge
          data-testid="review-state-badge-passed"
          className="text-[9px] px-1.5 py-px flex items-center gap-1"
          style={{
            backgroundColor: "rgba(16, 185, 129, 0.2)", // green/success background
            color: "var(--status-success)",
            border: "none",
          }}
        >
          <CheckCircle className="w-2.5 h-2.5" />
          <span>AI Approved</span>
        </Badge>
      );

    case "re_executing":
      return (
        <Badge
          data-testid="review-state-badge-re-executing"
          className="text-[9px] px-1.5 py-px flex items-center gap-1"
          style={{
            backgroundColor: "rgba(245, 158, 11, 0.2)", // amber/warning background
            color: "var(--status-warning)",
            border: "none",
          }}
        >
          <RotateCcw className="w-2.5 h-2.5" />
          <span>{revisionCount !== undefined ? `Attempt #${revisionCount + 1}` : "Revising"}</span>
        </Badge>
      );

    default:
      return null;
  }
}
