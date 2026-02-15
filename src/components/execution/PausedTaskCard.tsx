/**
 * PausedTaskCard - Compact two-line row for a paused task
 *
 * Line 1: icon | title | error category badge | resume/view actions
 * Line 2: error reason · retry countdown · resume attempts
 */

import { Clock, Play, ExternalLink, AlertTriangle, WifiOff, Server, ShieldAlert } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { getStatusIconConfig } from "@/types/status-icons";
import type { Task } from "@/types/task";

/** Provider error metadata stored in task.metadata JSON */
export interface ProviderErrorMetadata {
  category: "rate_limit" | "auth_error" | "server_error" | "network_error" | "overloaded";
  message: string;
  retry_after: string | null;
  previous_status: string;
  paused_at: string;
  auto_resumable: boolean;
  resume_attempts: number;
}

interface PausedTaskCardProps {
  task: Task;
  errorMeta: ProviderErrorMetadata;
  onResume: (taskId: string) => void;
  onViewDetails: (taskId: string) => void;
}

const MAX_RESUME_ATTEMPTS = 5;

/** Icon + color for each error category */
function getCategoryStyle(category: ProviderErrorMetadata["category"]): {
  icon: React.ComponentType<{ className?: string; style?: React.CSSProperties }>;
  color: string;
  bgColor: string;
  label: string;
} {
  switch (category) {
    case "rate_limit":
      return {
        icon: Clock,
        color: "hsl(45 90% 55%)",
        bgColor: "hsla(45 90% 55% / 0.15)",
        label: "Rate Limit",
      };
    case "auth_error":
      return {
        icon: ShieldAlert,
        color: "hsl(0 70% 55%)",
        bgColor: "hsla(0 70% 55% / 0.15)",
        label: "Auth Error",
      };
    case "server_error":
      return {
        icon: Server,
        color: "hsl(0 70% 55%)",
        bgColor: "hsla(0 70% 55% / 0.15)",
        label: "Server Error",
      };
    case "network_error":
      return {
        icon: WifiOff,
        color: "hsl(25 90% 55%)",
        bgColor: "hsla(25 90% 55% / 0.15)",
        label: "Network",
      };
    case "overloaded":
      return {
        icon: AlertTriangle,
        color: "hsl(45 90% 55%)",
        bgColor: "hsla(45 90% 55% / 0.15)",
        label: "Overloaded",
      };
  }
}

/** Format remaining time until retry */
function formatCountdown(retryAfter: string | null): string | null {
  if (!retryAfter) return null;

  const target = new Date(retryAfter).getTime();
  const now = Date.now();
  const diff = Math.max(0, Math.floor((target - now) / 1000));

  if (diff <= 0) return "Retrying soon...";

  const hours = Math.floor(diff / 3600);
  const mins = Math.floor((diff % 3600) / 60);
  const secs = diff % 60;

  if (hours > 0) return `${hours}h ${mins}m`;
  if (mins > 0) return `${mins}m ${secs}s`;
  return `${secs}s`;
}

export function PausedTaskCard({ task, errorMeta, onResume, onViewDetails }: PausedTaskCardProps) {
  const catStyle = getCategoryStyle(errorMeta.category);
  const pausedStyle = getStatusIconConfig("paused");
  const CatIcon = catStyle.icon;

  const handleResume = useCallback(() => onResume(task.id), [onResume, task.id]);
  const handleView = useCallback(() => onViewDetails(task.id), [onViewDetails, task.id]);

  // Live countdown ticker
  const [countdown, setCountdown] = useState(() => formatCountdown(errorMeta.retry_after));

  useEffect(() => {
    if (!errorMeta.retry_after) return;

    setCountdown(formatCountdown(errorMeta.retry_after));
    const interval = setInterval(() => {
      setCountdown(formatCountdown(errorMeta.retry_after));
    }, 1000);

    return () => clearInterval(interval);
  }, [errorMeta.retry_after]);

  const truncatedMessage =
    errorMeta.message.length > 60
      ? errorMeta.message.slice(0, 57) + "..."
      : errorMeta.message;

  return (
    <div
      data-testid={`paused-task-card-${task.id}`}
      className="px-2 py-1.5 rounded-md hover:bg-white/[0.04] transition-colors"
    >
      {/* Line 1: Icon | Title | Category Badge | Actions */}
      <div className="flex items-center gap-2">
        <CatIcon
          className="w-3.5 h-3.5 shrink-0"
          style={{ color: catStyle.color }}
        />
        <span
          className="flex-1 text-xs font-medium truncate min-w-0"
          style={{ color: "hsl(220 10% 88%)" }}
          title={task.title}
        >
          {task.title}
        </span>
        <span
          className="text-[10px] font-medium px-1.5 py-0.5 rounded shrink-0"
          style={{
            color: catStyle.color,
            backgroundColor: catStyle.bgColor,
          }}
        >
          {catStyle.label}
        </span>
        <div className="flex items-center shrink-0">
          <button
            data-testid={`resume-button-${task.id}`}
            onClick={handleResume}
            className="w-6 h-6 flex items-center justify-center rounded hover:bg-white/[0.08] transition-colors"
            style={{ color: pausedStyle.color }}
            title="Resume now"
          >
            <Play className="w-3 h-3" />
          </button>
          <button
            data-testid={`view-details-button-${task.id}`}
            onClick={handleView}
            className="w-6 h-6 flex items-center justify-center rounded hover:bg-white/[0.08] transition-colors"
            style={{ color: "hsl(220 10% 55%)" }}
            title="View details"
          >
            <ExternalLink className="w-3 h-3" />
          </button>
        </div>
      </div>

      {/* Line 2: Error reason · Countdown · Resume attempts */}
      <div
        className="flex items-center gap-1.5 mt-0.5 pl-[22px] text-[11px] min-w-0"
        style={{ color: "hsl(220 10% 50%)" }}
      >
        <span className="truncate min-w-0" title={errorMeta.message}>
          {truncatedMessage}
        </span>
        {countdown && (
          <>
            <span className="shrink-0" style={{ color: "hsl(220 10% 30%)" }}>·</span>
            <span className="shrink-0 tabular-nums" style={{ color: pausedStyle.color }}>
              {countdown}
            </span>
          </>
        )}
        <span className="shrink-0" style={{ color: "hsl(220 10% 30%)" }}>·</span>
        <span className="shrink-0 tabular-nums">
          {errorMeta.resume_attempts}/{MAX_RESUME_ATTEMPTS}
        </span>
        {errorMeta.auto_resumable && (
          <>
            <span className="shrink-0" style={{ color: "hsl(220 10% 30%)" }}>·</span>
            <span
              className="text-[10px] font-medium px-1 rounded shrink-0"
              style={{
                color: "hsl(145 60% 45%)",
                backgroundColor: "hsla(145 60% 45% / 0.15)",
              }}
            >
              Auto
            </span>
          </>
        )}
      </div>
    </div>
  );
}
