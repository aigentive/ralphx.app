/**
 * InsightsView - Project analytics dashboard with effort estimation
 *
 * Design: macOS Tahoe - flat backgrounds, warm orange accent, SF Pro
 * - NO purple/blue accents
 * - NO borders or glows
 * - Two-column dashboard: metrics left, EME sticky right (>=1200px)
 */

import { useMemo } from "react";
import { Download } from "lucide-react";
import { formatMinutesHuman } from "@/lib/formatters";
import { useProjectStore, selectActiveProject } from "@/stores/projectStore";
import { useProjectStats } from "@/hooks/useProjectStats";
import { useProjectTrends } from "@/hooks/useProjectTrends";
import { DetailCard } from "@/components/tasks/detail-views/shared/DetailCard";
import type { ProjectStats, ProjectTrends, WeeklyDataPoint } from "@/types/project-stats";
import {
  formatCSV,
  formatJSONExport,
  shouldShowTrends,
  shouldShowEme,
} from "@/lib/insights-export";
import { StatCard } from "./insights/StatCard";
import { TrendChart } from "./insights/TrendChart";
import { EffortEstimationPanel } from "./insights/EffortEstimationPanel";
import {
  CycleTimeBreakdown,
  ColumnDwellTimeBreakdown,
  CopyMarkdownButton,
} from "./insights/MetricsDetails";

// ============================================================================
// Helpers
// ============================================================================

function formatPercent(value: number): string {
  return `${Math.round(value * 100)}%`;
}

function downloadFile(content: string, filename: string, mimeType: string): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

function exportJSON(stats: ProjectStats, trends: ProjectTrends): void {
  const date = new Date().toISOString().slice(0, 10);
  const content = JSON.stringify(formatJSONExport(stats, trends), null, 2);
  downloadFile(content, `ralphx-insights-${date}.json`, "application/json");
}

function exportCSV(trends: ProjectTrends): void {
  const date = new Date().toISOString().slice(0, 10);
  const csv = formatCSV(trends);
  downloadFile(csv, `ralphx-insights-${date}.csv`, "text/csv");
}

function isCurrentWeek(weekStart: string): boolean {
  const now = new Date();
  const dayOfWeek = now.getUTCDay(); // 0=Sunday
  const sunday = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate() - dayOfWeek));
  return weekStart === sunday.toISOString().slice(0, 10);
}

function getWeekLabel(data: WeeklyDataPoint[]): string {
  if (data.length === 0) return "this week";
  const last = data[data.length - 1]!;
  return isCurrentWeek(last.weekStart) ? "this week" : "latest";
}

function getAvgPipelineTimeDisplay(stats: ProjectStats): string {
  if (stats.avgPipelineMinutes == null) return "—";
  return formatMinutesHuman(stats.avgPipelineMinutes);
}

// ============================================================================
// EME Panel (right column / inline depending on breakpoint)
// ============================================================================

function EmeSection({
  stats,
  showEme,
  projectId,
}: {
  stats: ProjectStats;
  showEme: boolean;
  projectId: string;
}) {
  if (showEme && stats.eme) {
    return (
      <EffortEstimationPanel
        lowHours={stats.eme.lowHours}
        highHours={stats.eme.highHours}
        taskCount={stats.eme.taskCount}
        earliestTaskDate={stats.eme.earliestTaskDate}
        latestTaskDate={stats.eme.latestTaskDate}
        projectId={projectId}
      />
    );
  }

  return (
    <DetailCard>
      <div className="flex flex-col gap-1">
        <p
          className="text-[13px] font-medium"
          style={{ color: "rgba(255,255,255,0.7)" }}
        >
          Effort estimation unlocks after 5 completed tasks
        </p>
        <p className="text-[12px]" style={{ color: "rgba(255,255,255,0.35)" }}>
          {stats.taskCount} of 5 tasks completed — keep going!
        </p>
      </div>
    </DetailCard>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function InsightsView() {
  const project = useProjectStore(selectActiveProject);
  const projectId = project?.id;

  const statsQuery = useProjectStats(projectId);
  const trendsQuery = useProjectTrends(projectId);

  // No active project
  if (!projectId) {
    return (
      <div
        className="flex flex-1 items-center justify-center"
        style={{ color: "rgba(255,255,255,0.3)" }}
      >
        <p className="text-[14px]">Select a project to view insights</p>
      </div>
    );
  }

  // Loading
  if (statsQuery.isLoading || trendsQuery.isLoading) {
    return (
      <div
        className="flex flex-1 items-center justify-center"
        style={{ color: "rgba(255,255,255,0.3)" }}
      >
        <p className="text-[14px]">Loading insights...</p>
      </div>
    );
  }

  // Error
  if (statsQuery.error ?? trendsQuery.error) {
    return (
      <div
        className="flex flex-1 items-center justify-center"
        style={{ color: "rgba(255,255,255,0.3)" }}
      >
        <p className="text-[14px]">Failed to load insights. Try again.</p>
      </div>
    );
  }

  const stats = statsQuery.data;
  const trends = trendsQuery.data;

  if (!stats || !trends) {
    return null;
  }

  const hasEnoughForTrends = shouldShowTrends(stats.taskCount);
  const showEme = shouldShowEme(stats.taskCount, stats.eme !== null);

  return (
    <InsightsContent
      stats={stats}
      trends={trends}
      projectId={projectId}
      hasEnoughForTrends={hasEnoughForTrends}
      showEme={showEme}
    />
  );
}

function InsightsContent({
  stats,
  trends,
  projectId,
  hasEnoughForTrends,
  showEme,
}: {
  stats: ProjectStats;
  trends: ProjectTrends;
  projectId: string;
  hasEnoughForTrends: boolean;
  showEme: boolean;
}) {
  const throughputWeekLabel = useMemo(() => getWeekLabel(trends.weeklyThroughput), [trends.weeklyThroughput]);
  const isThisWeek = throughputWeekLabel === "this week";

  const throughputHeader = useMemo(() => {
    if (trends.weeklyThroughput.length === 0) return undefined;
    const last = trends.weeklyThroughput[trends.weeklyThroughput.length - 1]!;
    return `${last.value} ${getWeekLabel(trends.weeklyThroughput)}`;
  }, [trends.weeklyThroughput]);

  const cycleTimeHeader = useMemo(() => {
    if (trends.weeklyCycleTime.length === 0) return undefined;
    const last = trends.weeklyCycleTime[trends.weeklyCycleTime.length - 1]!;
    return `${formatMinutesHuman(last.value * 60)} ${getWeekLabel(trends.weeklyCycleTime)}`;
  }, [trends.weeklyCycleTime]);

  const successRateHeader = useMemo(() => {
    if (trends.weeklySuccessRate.length === 0) return undefined;
    const last = trends.weeklySuccessRate[trends.weeklySuccessRate.length - 1]!;
    return `${Math.round(last.value * 100)}% ${getWeekLabel(trends.weeklySuccessRate)}`;
  }, [trends.weeklySuccessRate]);

  const showSuccessRateTrend = useMemo(() => {
    if (!hasEnoughForTrends) return false;
    const rates = trends.weeklySuccessRate.map((d) => d.value);
    if (rates.length === 0) return false;
    const avg = rates.reduce((a, b) => a + b, 0) / rates.length;
    if (avg < 0.95) return true;
    const variance = rates.reduce((sum, r) => sum + (r - avg) ** 2, 0) / rates.length;
    return Math.sqrt(variance) > 0.03;
  }, [hasEnoughForTrends, trends.weeklySuccessRate]);

  return (
    <div
      className="flex flex-col flex-1 overflow-auto"
      style={{ backgroundColor: "hsl(220 10% 8%)" }}
    >
      <div className="flex flex-col gap-6 p-6 max-w-[1400px] w-full mx-auto">
        {/* Header with export buttons */}
        <div className="flex items-start justify-between gap-4">
          <div className="flex flex-col gap-1">
            <h1
              className="text-[22px] font-semibold"
              style={{ color: "rgba(255,255,255,0.92)", fontFamily: "system-ui" }}
            >
              Insights
            </h1>
            <p className="text-[13px]" style={{ color: "rgba(255,255,255,0.4)" }}>
              Project analytics and effort estimation
            </p>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <CopyMarkdownButton stats={stats} />
            <button
              onClick={() => exportJSON(stats, trends)}
              className="flex items-center gap-2 rounded-lg px-3 py-2 text-[12px] font-medium transition-colors"
              style={{
                backgroundColor: "hsl(220 10% 14%)",
                color: "rgba(255,255,255,0.7)",
              }}
              title="Download JSON"
            >
              <Download size={13} />
              <span className="hidden min-[800px]:inline">JSON</span>
            </button>
            <button
              onClick={() => exportCSV(trends)}
              className="flex items-center gap-2 rounded-lg px-3 py-2 text-[12px] font-medium transition-colors"
              style={{
                backgroundColor: "hsl(220 10% 14%)",
                color: "rgba(255,255,255,0.7)",
              }}
              title="Download CSV"
            >
              <Download size={13} />
              <span className="hidden min-[800px]:inline">CSV</span>
            </button>
          </div>
        </div>

        {/* Two-column dashboard: metrics left, EME sticky right at >=1200px */}
        <div className="grid grid-cols-1 min-[1200px]:grid-cols-[1fr_320px] gap-6">
          {/* Left column: all metrics */}
          <div className="flex flex-col gap-4">
            {/* Stat cards — reordered: Tasks → Success → Cycle Time → Review */}
            <div className="grid grid-cols-2 min-[800px]:grid-cols-4 gap-3">
              <StatCard
                label={isThisWeek ? "Tasks This Week" : "Tasks (latest week)"}
                value={String(
                  trends.weeklyThroughput.length > 0
                    ? trends.weeklyThroughput[trends.weeklyThroughput.length - 1]!.value
                    : stats.tasksCompletedThisWeek
                )}
                sub={`${stats.tasksCompletedThisWeek} last 7 days · ${stats.tasksCompletedToday} today`}
                tooltip={isThisWeek
                  ? "Tasks merged this calendar week (Sun–Sat, UTC). The 'last 7 days' count uses a rolling window and may differ."
                  : "Tasks merged in the most recent week with data. No tasks merged in the current calendar week yet."}
              />
              <StatCard
                label="Agent Success Rate"
                value={formatPercent(stats.agentSuccessRate)}
                sub={`${stats.agentSuccessCount} / ${stats.agentTotalCount} tasks · all time`}
                tooltip="Percentage of tasks that completed successfully (merged) vs those that failed, were cancelled, or stopped. Higher = more reliable AI execution."
              />
              <StatCard
                label="Avg Pipeline Time"
                value={getAvgPipelineTimeDisplay(stats)}
                sub="start to merge · last 90 days"
                tooltip="Average wall-clock time a task takes from entering the pipeline to merge completion. Includes queue time, AI execution, review, and merge stages. Lower is better — most time is typically spent waiting (queue/escalation), not in active execution."
              />
              <StatCard
                label="Review Pass Rate"
                value={formatPercent(stats.reviewPassRate)}
                sub={`${stats.reviewPassCount} / ${stats.reviewTotalCount} reviews · all time`}
                tooltip="Percentage of AI code reviews that passed on first attempt without requesting changes. Higher = better first-draft quality."
              />
            </div>

            {/* EME panel — medium breakpoint (800-1199px): inline between stats and charts */}
            <div className="block min-[1200px]:hidden">
              <EmeSection stats={stats} showEme={showEme} projectId={projectId} />
            </div>

            {/* Trend charts */}
            {!hasEnoughForTrends ? (
              <DetailCard>
                <div className="flex flex-col gap-1">
                  <p
                    className="text-[13px] font-medium"
                    style={{ color: "rgba(255,255,255,0.7)" }}
                  >
                    Trend charts unlock after 10 completed tasks
                  </p>
                  <p className="text-[12px]" style={{ color: "rgba(255,255,255,0.35)" }}>
                    {stats.taskCount} of 10 tasks completed
                  </p>
                </div>
              </DetailCard>
            ) : (
              <div className="flex flex-col gap-3">
                <div className="grid grid-cols-1 min-[800px]:grid-cols-2 gap-3">
                  <DetailCard>
                    <TrendChart
                      title="Weekly Throughput (tasks)"
                      data={trends.weeklyThroughput}
                      {...(throughputHeader !== undefined && { currentValue: throughputHeader })}
                      timeWindow="Last 12 months"
                    />
                  </DetailCard>
                  <DetailCard>
                    <TrendChart
                      title="Execution Time"
                      data={trends.weeklyCycleTime}
                      valueFormatter={(v) => formatMinutesHuman(v * 60)}
                      primaryLabel="AI execution"
                      secondaryData={trends.weeklyPipelineCycleTime}
                      secondaryLabel="Pipeline (start → merge)"
                      secondaryValueFormatter={(v) => formatMinutesHuman(v * 60)}
                      {...(cycleTimeHeader !== undefined && { currentValue: cycleTimeHeader })}
                      timeWindow="Last 12 months"
                    />
                  </DetailCard>
                </div>
                {showSuccessRateTrend && (
                  <DetailCard>
                    <TrendChart
                      title="Agent Success Rate (%)"
                      data={trends.weeklySuccessRate}
                      valueFormatter={(v) => `${Math.round(v * 100)}%`}
                      color="#34d399"
                      {...(successRateHeader !== undefined && { currentValue: successRateHeader })}
                      timeWindow="Last 12 months"
                    />
                  </DetailCard>
                )}
              </div>
            )}

            {/* Breakdowns */}
            <div className="flex flex-col gap-3">
              <CycleTimeBreakdown phases={stats.cycleTimeBreakdown} />
              <ColumnDwellTimeBreakdown dwellTimes={stats.columnDwellTimes} />
            </div>
          </div>

          {/* Right column: EME sticky (only visible at >=1200px) */}
          <div className="hidden min-[1200px]:block min-[1200px]:sticky min-[1200px]:top-6 min-[1200px]:self-start min-[1200px]:max-h-[calc(100vh-48px)] min-[1200px]:overflow-y-auto">
            <EmeSection stats={stats} showEme={showEme} projectId={projectId} />
          </div>
        </div>
      </div>
    </div>
  );
}
