/**
 * ReviewStatusBadge - Displays review status with appropriate styling
 * Colors: pending (orange), approved (green), changes_requested (orange), rejected (red)
 */

import type { ReviewStatus } from "@/types/review";

interface ReviewStatusBadgeProps {
  status: ReviewStatus;
}

const STATUS_CONFIG: Record<ReviewStatus, { label: string; color: string; icon: "clock" | "check" | "warning" | "x" }> = {
  pending: { label: "Pending", color: "var(--status-warning)", icon: "clock" },
  approved: { label: "Approved", color: "var(--status-success)", icon: "check" },
  changes_requested: { label: "Changes Requested", color: "var(--status-warning)", icon: "warning" },
  rejected: { label: "Rejected", color: "var(--status-error)", icon: "x" },
};

function ClockIcon() {
  return (
    <svg data-testid="icon-clock" width="12" height="12" viewBox="0 0 12 12" fill="none">
      <circle cx="6" cy="6" r="5" stroke="currentColor" strokeWidth="1.5" />
      <path d="M6 3V6L8 7.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg data-testid="icon-check" width="12" height="12" viewBox="0 0 12 12" fill="none">
      <path d="M10 3L4.5 8.5L2 6" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function WarningIcon() {
  return (
    <svg data-testid="icon-warning" width="12" height="12" viewBox="0 0 12 12" fill="none">
      <path d="M6 1L11 10H1L6 1Z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
      <path d="M6 4.5V6.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
      <circle cx="6" cy="8.25" r="0.75" fill="currentColor" />
    </svg>
  );
}

function XIcon() {
  return (
    <svg data-testid="icon-x" width="12" height="12" viewBox="0 0 12 12" fill="none">
      <path d="M9 3L3 9M3 3L9 9" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
    </svg>
  );
}

export function ReviewStatusBadge({ status }: ReviewStatusBadgeProps) {
  const config = STATUS_CONFIG[status];
  const Icon = { clock: ClockIcon, check: CheckIcon, warning: WarningIcon, x: XIcon }[config.icon];

  return (
    <span
      data-testid="review-status-badge"
      data-status={status}
      className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium"
      style={{ backgroundColor: config.color, color: "var(--bg-base)" }}
    >
      <Icon />
      {config.label}
    </span>
  );
}
