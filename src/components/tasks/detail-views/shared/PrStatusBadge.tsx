import { GitPullRequest, GitMerge, GitPullRequestClosed, Pen } from "lucide-react";

type PrStatus = "Draft" | "Open" | "Merged" | "Closed";

const STATUS_CONFIG = {
  Draft: {
    label: "Draft",
    icon: Pen,
    bg: "rgba(142, 142, 147, 0.15)",
    color: "#8e8e93",
  },
  Open: {
    label: "Open",
    icon: GitPullRequest,
    bg: "rgba(52, 199, 89, 0.15)",
    color: "#34c759",
  },
  Merged: {
    label: "Merged",
    icon: GitMerge,
    bg: "rgba(175, 82, 222, 0.15)",
    color: "#af52de",
  },
  Closed: {
    label: "Closed",
    icon: GitPullRequestClosed,
    bg: "rgba(255, 69, 58, 0.15)",
    color: "#ff453a",
  },
} as const;

export function PrStatusBadge({ status }: { status: PrStatus }) {
  const config = STATUS_CONFIG[status];
  const Icon = config.icon;
  return (
    <span
      className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium"
      style={{ backgroundColor: config.bg, color: config.color }}
    >
      <Icon className="w-3 h-3" />
      {config.label}
    </span>
  );
}
