/**
 * TaskMetricsCard - Shows per-task engineering metrics in a detail view.
 *
 * Displays step count, review cycles, execution time, and derived complexity
 * tier. Fetches data via `useTaskMetrics` on mount (cached for 5 minutes).
 *
 * Complexity tier colours follow the warm-orange accent palette:
 * - Simple: green
 * - Medium: orange (accent)
 * - Complex: amber
 */

import { Loader2, ListChecks, RotateCcw, Timer, Zap } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useTaskMetrics } from "@/hooks/useTaskMetrics";
import { deriveComplexityTier, type ComplexityTier } from "@/api/task-metrics";
import { DetailCard } from "./DetailCard";
import { statusTint } from "@/lib/theme-colors";

// ── Complexity badge ─────────────────────────────────────────────────────────

const COMPLEXITY_STYLES: Record<
  ComplexityTier,
  { bg: string; color: string }
> = {
  Simple: {
    bg: statusTint("success", 12),
    color: "var(--status-success)",
  },
  Medium: {
    bg: statusTint("accent", 12),
    color: "var(--accent-primary)",
  },
  Complex: {
    bg: statusTint("warning", 12),
    color: "var(--status-warning)",
  },
};

function ComplexityBadge({ tier }: { tier: ComplexityTier }) {
  const styles = COMPLEXITY_STYLES[tier];
  return (
    <span
      data-testid="complexity-badge"
      className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium"
      style={{ backgroundColor: styles.bg, color: styles.color }}
    >
      <Zap className="w-3 h-3" />
      {tier}
    </span>
  );
}

// ── Stat row ─────────────────────────────────────────────────────────────────

function StatRow({
  icon: Icon,
  label,
  value,
}: {
  icon: LucideIcon;
  label: string;
  value: string;
}) {
  return (
    <div className="flex items-center gap-3">
      <div
        className="flex items-center justify-center w-7 h-7 rounded-lg shrink-0"
        style={{ backgroundColor: "var(--overlay-weak)" }}
      >
        <Icon className="w-3.5 h-3.5 text-text-primary/40" />
      </div>
      <div className="flex-1 min-w-0">
        <span className="text-[11px] uppercase tracking-wider text-text-primary/40 block">
          {label}
        </span>
        <span className="text-[13px] text-text-primary/70 font-medium">{value}</span>
      </div>
    </div>
  );
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function formatMinutes(minutes: number): string {
  if (minutes < 1) return "< 1 min";
  if (minutes < 60) return `${Math.round(minutes)} min`;
  const h = Math.floor(minutes / 60);
  const m = Math.round(minutes % 60);
  return m > 0 ? `${h}h ${m}m` : `${h}h`;
}

// ── Main component ────────────────────────────────────────────────────────────

interface TaskMetricsCardProps {
  taskId: string;
}

/**
 * TaskMetricsCard — fetches and displays per-task engineering metrics.
 *
 * Show this in MergedTaskDetail and WaitingTaskDetail where execution data
 * is meaningful. Do not render on cards — the badge is only visible after
 * the user opens the task detail (data is then cached for subsequent card renders).
 */
export function TaskMetricsCard({ taskId }: TaskMetricsCardProps) {
  const { data: metrics, isLoading, isError } = useTaskMetrics(taskId);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-4">
        <Loader2
          className="w-4 h-4 animate-spin text-text-primary/30"
        />
      </div>
    );
  }

  if (isError || !metrics) {
    return null;
  }

  const tier = deriveComplexityTier(metrics);

  const divider = (
    <div
      className="h-px"
      style={{ backgroundColor: "var(--overlay-weak)" }}
    />
  );

  return (
    <DetailCard>
      <div className="space-y-3">
        {/* Header: complexity tier */}
        <div className="flex items-center justify-between">
          <span className="text-[11px] uppercase tracking-wider text-text-primary/40">
            Complexity
          </span>
          <ComplexityBadge tier={tier} />
        </div>

        {divider}

        <StatRow
          icon={ListChecks}
          label="Steps"
          value={
            metrics.stepCount > 0
              ? `${metrics.completedStepCount} / ${metrics.stepCount} completed`
              : "No steps"
          }
        />

        {divider}

        <StatRow
          icon={RotateCcw}
          label="Reviews"
          value={
            metrics.reviewCount > 0
              ? `${metrics.reviewCount} cycle${metrics.reviewCount !== 1 ? "s" : ""}`
              : "No reviews"
          }
        />

        {divider}

        <StatRow
          icon={Timer}
          label="Execution time"
          value={
            metrics.executionMinutes > 0
              ? formatMinutes(metrics.executionMinutes)
              : "—"
          }
        />
      </div>
    </DetailCard>
  );
}
