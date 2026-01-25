/**
 * ReviewStatusBadge - Displays review status with appropriate styling
 * Uses Lucide icons and semi-transparent backgrounds per design spec
 */

import { Clock, CheckCircle, AlertCircle, XCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { ReviewStatus } from "@/types/review";

interface ReviewStatusBadgeProps {
  status: ReviewStatus;
  className?: string;
}

const STATUS_CONFIG: Record<
  ReviewStatus,
  {
    label: string;
    bgClass: string;
    textClass: string;
    Icon: typeof Clock;
  }
> = {
  pending: {
    label: "Pending",
    bgClass: "bg-[var(--bg-hover)]",
    textClass: "text-[var(--text-secondary)]",
    Icon: Clock,
  },
  approved: {
    label: "Approved",
    bgClass: "bg-emerald-500/15",
    textClass: "text-[var(--status-success)]",
    Icon: CheckCircle,
  },
  changes_requested: {
    label: "Changes Requested",
    bgClass: "bg-amber-500/15",
    textClass: "text-[var(--status-warning)]",
    Icon: AlertCircle,
  },
  rejected: {
    label: "Rejected",
    bgClass: "bg-red-500/15",
    textClass: "text-[var(--status-error)]",
    Icon: XCircle,
  },
};

export function ReviewStatusBadge({ status, className }: ReviewStatusBadgeProps) {
  const config = STATUS_CONFIG[status];
  const { Icon } = config;

  return (
    <Badge
      data-testid="review-status-badge"
      data-status={status}
      variant="outline"
      className={cn(
        "inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium border-0",
        "rounded-[var(--radius-sm)]",
        config.bgClass,
        config.textClass,
        className
      )}
    >
      <Icon className="w-3 h-3" data-testid={`icon-${status === "pending" ? "clock" : status}`} />
      {config.label}
    </Badge>
  );
}
