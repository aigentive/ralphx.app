/**
 * PriorityBadge - Visual indicator for priority levels
 * Colors follow design spec:
 * - Critical: Red (#ef4444)
 * - High: Orange (#ff6b35)
 * - Medium: Amber (#ffa94d)
 * - Low: Gray (#6b7280)
 */

import type { Priority } from "@/types/ideation";

interface PriorityConfig {
  backgroundColor: string;
  textColor: string;
  label: string;
}

const PRIORITY_CONFIG: Record<Priority, PriorityConfig> = {
  critical: {
    backgroundColor: "#ef4444",
    textColor: "#ffffff",
    label: "Critical",
  },
  high: {
    backgroundColor: "#ff6b35",
    textColor: "#1a1a1a",
    label: "High",
  },
  medium: {
    backgroundColor: "#ffa94d",
    textColor: "#1a1a1a",
    label: "Medium",
  },
  low: {
    backgroundColor: "#6b7280",
    textColor: "#ffffff",
    label: "Low",
  },
};

type BadgeSize = "compact" | "full";

interface PriorityBadgeProps {
  priority: Priority;
  size?: BadgeSize;
  className?: string;
}

export function PriorityBadge({
  priority,
  size = "compact",
  className = "",
}: PriorityBadgeProps) {
  const config = PRIORITY_CONFIG[priority];

  const sizeClasses =
    size === "compact"
      ? "text-xs px-1.5 py-0.5"
      : "text-sm px-2 py-1";

  return (
    <span
      data-testid="priority-badge"
      data-priority={priority}
      role="status"
      aria-label={`Priority: ${config.label}`}
      className={`inline-flex items-center justify-center rounded font-medium ${sizeClasses} ${className}`}
      style={{
        backgroundColor: config.backgroundColor,
        color: config.textColor,
      }}
    >
      {config.label}
    </span>
  );
}
