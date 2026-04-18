/**
 * ProjectStatsCard - Displays project metrics summary
 *
 * Progressive unlock based on taskCount:
 * - 0 tasks: empty state "Complete tasks to see metrics."
 * - ≥1 task: task counts row visible
 * - ≥5 tasks: full card (quality + cycle time + collapsible estimates)
 * - ≥10 tasks: trend indicators (data-ready flag; prev-week data TBD)
 *
 * Design: RalphX design system — SF Pro, warm orange accent (#ff6b35),
 * dark theme, no purple gradients.
 */

import { useState } from "react";
import {
  CheckCircle,
  TrendingUp,
  GitMerge,
  Clock,
  Copy,
  Check,
} from "lucide-react";
import { useProjectStats } from "@/hooks/useProjectStats";
import { formatMinutesHuman } from "@/lib/formatters";
import { CollapsibleEstimates } from "./CollapsibleEstimates";
import type { CycleTimePhase, ProjectStats } from "@/types/project-stats";

// ============================================================================
// Sub-components
// ============================================================================

interface StatItemProps {
  label: string;
  value: string | number;
  sub?: string;
  icon?: React.ReactNode;
}

function StatItem({ label, value, sub, icon }: StatItemProps) {
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-1.5 text-[var(--text-muted)]">
        {icon && <span className="text-[var(--text-muted)]">{icon}</span>}
        <span className="text-xs uppercase tracking-wide">{label}</span>
      </div>
      <div className="flex items-baseline gap-1.5">
        <span
          className="text-2xl font-semibold text-[var(--text-primary)] tabular-nums"
          data-testid={`stat-value-${label.toLowerCase().replace(/\s+/g, "-")}`}
        >
          {value}
        </span>
        {sub && (
          <span className="text-xs text-[var(--text-muted)]">{sub}</span>
        )}
      </div>
    </div>
  );
}

interface RateBarProps {
  label: string;
  rate: number; // 0–1
  passCount: number;
  totalCount: number;
  color: string;
  testId: string;
}

function RateBar({ label, rate, passCount, totalCount, color, testId }: RateBarProps) {
  const pct = Math.round(rate * 100);
  return (
    <div className="flex flex-col gap-1.5" data-testid={testId}>
      <div className="flex items-center justify-between">
        <span className="text-xs text-[var(--text-muted)]">{label}</span>
        <span className="text-xs font-medium text-[var(--text-secondary)]">
          {pct}%
          <span className="ml-1 text-[var(--text-muted)]">
            ({passCount}/{totalCount})
          </span>
        </span>
      </div>
      <div
        className="h-1.5 w-full rounded-full"
        style={{ backgroundColor: "var(--overlay-weak)" }}
        role="progressbar"
        aria-valuenow={pct}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={`${label}: ${pct}%`}
      >
        <div
          className="h-full rounded-full transition-all duration-300"
          style={{ width: `${pct}%`, backgroundColor: color }}
        />
      </div>
    </div>
  );
}

interface CycleTimeBarProps {
  phase: CycleTimePhase;
  maxMinutes: number;
}

function CycleTimeBar({ phase, maxMinutes }: CycleTimeBarProps) {
  const pct = maxMinutes > 0 ? Math.round((phase.avgMinutes / maxMinutes) * 100) : 0;

  return (
    <div className="flex items-center gap-2 text-xs" data-testid={`cycle-phase-${phase.phase}`}>
      <span
        className="w-20 shrink-0 truncate text-[var(--text-muted)]"
        title={phase.phase}
      >
        {phase.phase}
      </span>
      <div
        className="flex-1 h-1 rounded-full"
        style={{ backgroundColor: "var(--overlay-weak)" }}
      >
        <div
          className="h-full rounded-full"
          style={{ width: `${pct}%`, backgroundColor: "var(--accent-primary)" }}
        />
      </div>
      <span className="w-16 text-right text-[var(--text-secondary)] shrink-0 tabular-nums">
        {formatMinutesHuman(phase.avgMinutes)}
      </span>
    </div>
  );
}

// ============================================================================
// Loading skeleton
// ============================================================================

function LoadingSkeleton() {
  return (
    <div
      className="rounded-xl p-4 space-y-4 animate-pulse"
      style={{ backgroundColor: "hsl(220 10% 10%)" }}
      data-testid="project-stats-loading"
    >
      {[...Array(3)].map((_, i) => (
        <div
          key={i}
          className="h-10 rounded"
          style={{ backgroundColor: "var(--overlay-weak)" }}
        />
      ))}
    </div>
  );
}

// ============================================================================
// Error state
// ============================================================================

function ErrorState({ message }: { message: string }) {
  return (
    <div
      className="rounded-xl p-4 text-sm text-[var(--text-muted)]"
      style={{ backgroundColor: "hsl(220 10% 10%)" }}
      data-testid="project-stats-error"
    >
      {message}
    </div>
  );
}

// ============================================================================
// Markdown generation
// ============================================================================

function generateMarkdown(stats: ProjectStats): string {
  const date = new Date().toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });

  const agentPct = Math.round(stats.agentSuccessRate * 100);
  const reviewPct = Math.round(stats.reviewPassRate * 100);

  const cycleTimeLine =
    stats.cycleTimeBreakdown.length > 0
      ? stats.cycleTimeBreakdown
          .map((p) => `${p.phase}: ${formatMinutesHuman(p.avgMinutes)}`)
          .join(", ")
      : "No data yet";

  const emeSection = stats.eme
    ? `\n### Estimated Manual Effort\n- **Estimate:** ~${stats.eme.lowHours}–${stats.eme.highHours} hours\n- *Based on task complexity analysis.*\n`
    : "";

  return `## Project Stats — ${date}

### Quality
- **Agent Success Rate:** ${agentPct}% (${stats.agentSuccessCount}/${stats.agentTotalCount} tasks)
- **Review Pass Rate:** ${reviewPct}% first-pass approval

### Throughput
- **Tasks Completed:** ${stats.tasksCompletedThisWeek} this week, ${stats.tasksCompletedThisMonth} this month
- **Avg Cycle Time:** ${cycleTimeLine}
${emeSection}
---
*Metrics computed locally from RalphX — your data never leaves your machine.*`;
}

// ============================================================================
// Main component
// ============================================================================

export interface ProjectStatsCardProps {
  projectId: string;
  className?: string;
}

export function ProjectStatsCard({ projectId, className = "" }: ProjectStatsCardProps) {
  const { data: stats, isLoading, isError } = useProjectStats(projectId);
  const [copied, setCopied] = useState(false);

  if (isLoading) {
    return <LoadingSkeleton />;
  }

  if (isError || !stats) {
    return <ErrorState message="Could not load project stats." />;
  }

  // Capture after null guards — TypeScript can't narrow through async closures
  const safeStats = stats;
  const showTaskMetrics = safeStats.taskCount >= 1;
  const showFullCard = safeStats.taskCount >= 5;
  const showEstimates = showFullCard && safeStats.eme !== null;

  // 0 tasks: empty state
  if (!showTaskMetrics) {
    return (
      <div
        className={`rounded-xl p-4 ${className}`}
        style={{ backgroundColor: "hsl(220 10% 10%)" }}
        data-testid="project-stats-card"
      >
        <div className="flex items-center gap-2 mb-3">
          <TrendingUp
            className="w-4 h-4"
            style={{ color: "var(--accent-primary)" }}
            aria-hidden="true"
          />
          <span className="text-sm font-medium text-[var(--text-primary)]">
            Project Stats
          </span>
        </div>
        <p
          className="text-sm text-[var(--text-muted)]"
          data-testid="project-stats-empty"
        >
          Complete tasks to see metrics.
        </p>
      </div>
    );
  }

  const maxCycleMinutes = safeStats.cycleTimeBreakdown.reduce(
    (acc, p) => Math.max(acc, p.avgMinutes),
    0
  );

  const hasCycleTimes = safeStats.cycleTimeBreakdown.length > 0;
  const hasRates = safeStats.agentTotalCount > 0 || safeStats.reviewTotalCount > 0;

  async function handleCopyMarkdown() {
    try {
      await navigator.clipboard.writeText(generateMarkdown(safeStats));
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard not available in this context
    }
  }

  return (
    <div
      className={`rounded-xl p-4 space-y-5 ${className}`}
      style={{ backgroundColor: "hsl(220 10% 10%)" }}
      data-testid="project-stats-card"
    >
      {/* Header */}
      <div className="flex items-center gap-2">
        <TrendingUp
          className="w-4 h-4"
          style={{ color: "var(--accent-primary)" }}
          aria-hidden="true"
        />
        <span className="text-sm font-medium text-[var(--text-primary)]">
          Project Stats
        </span>
      </div>

      {/* Task counts row (≥1 task) */}
      <div
        className="grid grid-cols-3 gap-4"
        style={{
          borderBottom: "1px solid var(--overlay-weak)",
          paddingBottom: "16px",
        }}
      >
        <StatItem
          label="Total"
          value={safeStats.taskCount}
          icon={<CheckCircle className="w-3.5 h-3.5" />}
        />
        <StatItem
          label="This Week"
          value={safeStats.tasksCompletedThisWeek}
          sub="done"
        />
        <StatItem
          label="Today"
          value={safeStats.tasksCompletedToday}
          sub="done"
        />
      </div>

      {/* Full card sections (≥5 tasks) */}
      {showFullCard && (
        <>
          {/* Quality rates */}
          {hasRates && (
            <div
              className="space-y-3"
              style={{
                borderBottom: hasCycleTimes
                  ? "1px solid var(--overlay-weak)"
                  : undefined,
                paddingBottom: hasCycleTimes ? "16px" : undefined,
              }}
            >
              <div className="flex items-center gap-1.5 text-[var(--text-muted)]">
                <GitMerge className="w-3.5 h-3.5" aria-hidden="true" />
                <span className="text-xs uppercase tracking-wide">Quality</span>
              </div>

              {safeStats.agentTotalCount > 0 && (
                <RateBar
                  label="Agent success"
                  rate={safeStats.agentSuccessRate}
                  passCount={safeStats.agentSuccessCount}
                  totalCount={safeStats.agentTotalCount}
                  color="var(--accent-primary)"
                  testId="agent-success-rate"
                />
              )}

              {safeStats.reviewTotalCount > 0 && (
                <RateBar
                  label="Review pass"
                  rate={safeStats.reviewPassRate}
                  passCount={safeStats.reviewPassCount}
                  totalCount={safeStats.reviewTotalCount}
                  color="hsl(145 60% 45%)"
                  testId="review-pass-rate"
                />
              )}
            </div>
          )}

          {/* Cycle time breakdown */}
          {hasCycleTimes && (
            <div className="space-y-2.5">
              <div className="flex items-center gap-1.5 text-[var(--text-muted)]">
                <Clock className="w-3.5 h-3.5" aria-hidden="true" />
                <span className="text-xs uppercase tracking-wide">
                  Cycle Time
                </span>
              </div>
              <div className="space-y-2">
                {safeStats.cycleTimeBreakdown.map((phase) => (
                  <CycleTimeBar
                    key={phase.phase}
                    phase={phase}
                    maxMinutes={maxCycleMinutes}
                  />
                ))}
              </div>
            </div>
          )}

          {/* Estimates (collapsible, only when ≥5 tasks AND eme non-null) */}
          {showEstimates && safeStats.eme && (
            <div
              style={{
                borderTop: "1px solid var(--overlay-weak)",
                paddingTop: "12px",
              }}
            >
              <CollapsibleEstimates eme={safeStats.eme} projectId={projectId} />
            </div>
          )}
        </>
      )}

      {/* Copy as Markdown button */}
      <div
        style={{
          borderTop: "1px solid var(--overlay-weak)",
          paddingTop: "12px",
        }}
      >
        <button
          onClick={handleCopyMarkdown}
          className="flex items-center gap-1.5 text-xs text-[var(--text-muted)] hover:text-[var(--text-secondary)] transition-colors w-full justify-end"
          data-testid="copy-markdown-button"
        >
          {copied ? (
            <>
              <Check
                className="w-3.5 h-3.5"
                style={{ color: "hsl(145 60% 45%)" }}
                aria-hidden="true"
              />
              <span style={{ color: "hsl(145 60% 45%)" }}>Copied!</span>
            </>
          ) : (
            <>
              <Copy className="w-3.5 h-3.5" aria-hidden="true" />
              <span>Copy as Markdown</span>
            </>
          )}
        </button>
      </div>
    </div>
  );
}
