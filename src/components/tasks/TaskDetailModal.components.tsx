/**
 * TaskDetailModal sub-components
 */

import { Badge } from "@/components/ui/badge";
import type { InternalStatus } from "@/types/task";
import { Bot, User, Wrench } from "lucide-react";
import { PRIORITY_COLORS, STATUS_CONFIG, DEFAULT_PRIORITY_COLOR } from "./TaskDetailModal.constants";

export function PriorityBadge({ priority }: { priority: number }) {
  const colors = PRIORITY_COLORS[priority] ?? DEFAULT_PRIORITY_COLOR;
  return (
    <span
      data-testid="task-detail-priority"
      className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono font-medium"
      style={{ backgroundColor: colors.bg, color: colors.text }}
    >
      P{priority}
    </span>
  );
}

export function StatusBadge({ status }: { status: InternalStatus }) {
  const config = STATUS_CONFIG[status];
  return (
    <Badge
      data-testid="task-detail-status"
      data-status={status}
      className="rounded px-1.5 py-0.5 text-[10px] font-medium border-0"
      style={{ backgroundColor: config.bg, color: config.text }}
    >
      {config.label}
    </Badge>
  );
}

export function ReviewCard({
  reviewerType,
  status,
}: {
  reviewerType: "ai" | "human";
  status: string;
}) {
  const Icon = reviewerType === "ai" ? Bot : User;
  const label = reviewerType === "ai" ? "AI Review" : "Human Review";

  const defaultStatusColor = { bg: "rgba(255,255,255,0.05)", text: "rgba(255,255,255,0.5)" };
  const statusColors: Record<string, { bg: string; text: string }> = {
    pending: defaultStatusColor,
    approved: {
      bg: "rgba(16, 185, 129, 0.15)",
      text: "var(--status-success)",
    },
    changes_requested: {
      bg: "rgba(245, 158, 11, 0.15)",
      text: "var(--status-warning)",
    },
    rejected: { bg: "rgba(239, 68, 68, 0.15)", text: "var(--status-error)" },
  };

  const statusColor = statusColors[status] ?? defaultStatusColor;

  return (
    <div
      data-testid={`review-item-${reviewerType}`}
      className="flex items-center justify-between p-2.5 rounded-lg"
      style={{
        background: "linear-gradient(180deg, rgba(28,28,28,0.9) 0%, rgba(22,22,22,0.95) 100%)",
        border: "1px solid rgba(255,255,255,0.06)",
      }}
    >
      <div className="flex items-center gap-2">
        <Icon className="w-3.5 h-3.5 text-white/50" />
        <span className="text-[13px] font-medium text-white/80">
          {label}
        </span>
      </div>
      <Badge
        className="rounded px-1.5 py-0.5 text-[10px] font-medium border-0 capitalize"
        style={{ backgroundColor: statusColor.bg, color: statusColor.text }}
      >
        {status.replace("_", " ")}
      </Badge>
    </div>
  );
}

export function FixTaskIndicator({ count }: { count: number }) {
  const label = count === 1 ? "1 fix task" : `${count} fix tasks`;
  return (
    <div
      data-testid="fix-task-indicator"
      className="flex items-center gap-2 text-sm mt-3"
      style={{ color: "var(--status-warning)" }}
    >
      <Wrench className="w-4 h-4" />
      <span>{label}</span>
    </div>
  );
}

export function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-[13px] font-medium mb-2.5 text-white/80">
      {children}
    </h3>
  );
}
