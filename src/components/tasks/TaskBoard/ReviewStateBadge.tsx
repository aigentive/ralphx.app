/**
 * ReviewStateBadge - Icon-only badge for review-related task states
 *
 * Displays a styled icon indicating the current review state:
 * - revision_needed: Orange retry icon
 * - pending_review: Neutral clock icon
 * - reviewing: Blue spinner (active)
 * - review_passed: Green checkmark
 * - re_executing: Orange spinner (active)
 *
 * Active states show a spinning loader icon.
 */

import { Clock, RotateCcw, CheckCircle, Loader2 } from "lucide-react";

export interface ReviewStateBadgeProps {
  /** The internal status of the task */
  status: string;
  /** Number of revision attempts (optional, for re_executing state) */
  revisionCount?: number | undefined;
}

/**
 * Renders an icon-only badge for review-related task states
 * Active states (reviewing, re_executing) show a spinner
 */
export function ReviewStateBadge({ status, revisionCount }: ReviewStateBadgeProps) {
  switch (status) {
    case "revision_needed":
      return (
        <div
          data-testid="review-state-badge-revision"
          className="flex items-center justify-center w-5 h-5 rounded"
          style={{
            backgroundColor: "rgba(245, 158, 11, 0.2)",
            color: "var(--status-warning)",
          }}
          title="Revision needed"
        >
          <RotateCcw className="w-3 h-3" />
        </div>
      );

    case "pending_review":
      return (
        <div
          data-testid="review-state-badge-pending"
          className="flex items-center justify-center w-5 h-5 rounded"
          style={{
            backgroundColor: "rgba(255, 255, 255, 0.1)",
            color: "var(--text-secondary)",
          }}
          title="Pending review"
        >
          <Clock className="w-3 h-3" />
        </div>
      );

    case "reviewing":
      return (
        <div
          data-testid="review-state-badge-reviewing"
          className="flex items-center justify-center w-5 h-5 rounded"
          style={{
            backgroundColor: "rgba(59, 130, 246, 0.2)",
            color: "var(--status-info)",
          }}
          title="Reviewing"
        >
          <Loader2 className="w-3 h-3 animate-spin" />
        </div>
      );

    case "review_passed":
      return (
        <div
          data-testid="review-state-badge-passed"
          className="flex items-center justify-center w-5 h-5 rounded"
          style={{
            backgroundColor: "rgba(16, 185, 129, 0.2)",
            color: "var(--status-success)",
          }}
          title="AI Approved"
        >
          <CheckCircle className="w-3 h-3" />
        </div>
      );

    case "re_executing":
      return (
        <div
          data-testid="review-state-badge-re-executing"
          className="flex items-center justify-center w-5 h-5 rounded"
          style={{
            backgroundColor: "rgba(245, 158, 11, 0.2)",
            color: "var(--status-warning)",
          }}
          title={revisionCount !== undefined ? `Attempt #${revisionCount + 1}` : "Revising"}
        >
          <Loader2 className="w-3 h-3 animate-spin" />
        </div>
      );

    default:
      return null;
  }
}
