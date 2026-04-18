import { GitPullRequest, GitMerge, GitPullRequestClosed, Pen } from "lucide-react";
import { statusTint, withAlpha } from "@/lib/theme-colors";

type PrStatus = "Draft" | "Open" | "Merged" | "Closed";

const STATUS_CONFIG = {
  Draft: {
    label: "Draft",
    icon: Pen,
    bg: withAlpha("var(--text-muted)", 15),
    color: "var(--text-muted)",
  },
  Open: {
    label: "Open",
    icon: GitPullRequest,
    bg: "var(--status-success-muted)",
    color: "var(--status-success)",
  },
  Merged: {
    label: "Merged",
    icon: GitMerge,
    // Violet is PR-only — the only non-palette tone we keep.
    bg: withAlpha("#af52de", 15),
    color: "#af52de",
  },
  Closed: {
    label: "Closed",
    icon: GitPullRequestClosed,
    bg: statusTint("error", 15),
    color: "var(--status-error)",
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
