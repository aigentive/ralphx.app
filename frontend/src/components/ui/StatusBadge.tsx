/**
 * StatusBadge - Displays review or QA status with appropriate styling
 */

export type ReviewStatus = "ai_approved" | "human_approved" | "needs_changes";
export type QAStatus = "pending" | "preparing" | "ready" | "testing" | "passed" | "failed";

interface ReviewBadgeProps {
  type: "review";
  status: ReviewStatus;
}

interface QABadgeProps {
  type: "qa";
  status: QAStatus;
}

type StatusBadgeProps = ReviewBadgeProps | QABadgeProps;

const REVIEW_CONFIG: Record<ReviewStatus, { label: string; color: string; icon: string }> = {
  ai_approved: { label: "AI Approved", color: "var(--status-success)", icon: "check" },
  human_approved: { label: "Human Approved", color: "var(--status-info)", icon: "double-check" },
  needs_changes: { label: "Needs Changes", color: "var(--status-warning)", icon: "warning" },
};

const QA_CONFIG: Record<QAStatus, { label: string; color: string }> = {
  pending: { label: "Pending", color: "var(--text-muted)" },
  preparing: { label: "Preparing", color: "var(--status-warning)" },
  ready: { label: "Ready", color: "var(--status-info)" },
  testing: { label: "Testing", color: "var(--accent-secondary)" },
  passed: { label: "Passed", color: "var(--status-success)" },
  failed: { label: "Failed", color: "var(--status-error)" },
};

function CheckIcon() {
  return (
    <svg data-testid="icon-check" width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
      <path d="M10 3L4.5 8.5L2 6" stroke="currentColor" strokeWidth="2" fill="none" />
    </svg>
  );
}

function DoubleCheckIcon() {
  return (
    <svg data-testid="icon-double-check" width="14" height="12" viewBox="0 0 14 12" fill="currentColor">
      <path d="M8 3L3.5 8.5L1 6" stroke="currentColor" strokeWidth="2" fill="none" />
      <path d="M13 3L7.5 8.5L6 7" stroke="currentColor" strokeWidth="2" fill="none" />
    </svg>
  );
}

function WarningIcon() {
  return (
    <svg data-testid="icon-warning" width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
      <path d="M6 1L11 10H1L6 1Z" stroke="currentColor" strokeWidth="1.5" fill="none" />
      <line x1="6" y1="4" x2="6" y2="7" stroke="currentColor" strokeWidth="1.5" />
      <circle cx="6" cy="8.5" r="0.75" fill="currentColor" />
    </svg>
  );
}

export function StatusBadge(props: StatusBadgeProps) {
  if (props.type === "review") {
    const config = REVIEW_CONFIG[props.status];
    return (
      <span
        data-testid="status-badge"
        data-status={props.status}
        className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium"
        style={{ backgroundColor: config.color, color: "var(--bg-base)" }}
      >
        {config.icon === "check" && <CheckIcon />}
        {config.icon === "double-check" && <DoubleCheckIcon />}
        {config.icon === "warning" && <WarningIcon />}
        {config.label}
      </span>
    );
  }

  const config = QA_CONFIG[props.status];
  return (
    <span
      data-testid="status-badge"
      data-status={props.status}
      className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium"
      style={{ backgroundColor: config.color, color: "var(--bg-base)" }}
    >
      {config.label}
    </span>
  );
}
