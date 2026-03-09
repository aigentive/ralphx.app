/**
 * InsightsView - Project analytics with trend charts and effort estimation
 *
 * Design: macOS Tahoe - flat backgrounds, warm orange accent, SF Pro
 * - NO purple/blue accents
 * - NO borders or glows
 * - Two clearly separated sections: Observed Metrics + Estimates
 */

import { Download } from "lucide-react";
import { formatMinutesHuman } from "@/lib/formatters";
import { useProjectStore, selectActiveProject } from "@/stores/projectStore";
import { useProjectStats } from "@/hooks/useProjectStats";
import { useProjectTrends } from "@/hooks/useProjectTrends";
import { DetailCard } from "@/components/tasks/detail-views/shared/DetailCard";
import { SectionTitle } from "@/components/tasks/detail-views/shared/SectionTitle";
import type { ProjectStats, ProjectTrends } from "@/types/project-stats";
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

/** Terminal/non-pipeline states excluded from cycle time sum */
const TERMINAL_PHASES = new Set([
  "merged",
  "cancelled",
  "failed",
  "stopped",
  "paused",
  "blocked",
]);

function getAvgCycleTimeDisplay(stats: ProjectStats): string {
  const pipelinePhases = stats.cycleTimeBreakdown.filter(
    (p) => !TERMINAL_PHASES.has(p.phase),
  );
  if (pipelinePhases.length === 0) return "—";
  const total = pipelinePhases.reduce((sum, p) => sum + p.avgMinutes, 0);
  return formatMinutesHuman(total);
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
    <div
      className="flex flex-col flex-1 overflow-auto"
      style={{ backgroundColor: "hsl(220 10% 8%)" }}
    >
      <div className="flex flex-col gap-6 p-6 max-w-[1400px] w-full mx-auto">
        {/* Header */}
        <div className="flex flex-col gap-1">
          <h1
            className="text-[22px] font-semibold"
            style={{ color: "rgba(255,255,255,0.92)", fontFamily: "system-ui" }}
          >
            Insights
          </h1>
          <p className="text-[13px]" style={{ color: "rgba(255,255,255,0.4)" }}>
            Project analytics, trend analysis, and effort estimation
          </p>
        </div>

        {/* Section 1: Observed Metrics */}
        <section className="flex flex-col gap-4">
          <SectionTitle>Data — Observed Metrics</SectionTitle>

          {/* Stats row */}
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
            <StatCard
              label="Agent Success Rate"
              value={formatPercent(stats.agentSuccessRate)}
              sub={`${stats.agentSuccessCount} / ${stats.agentTotalCount} tasks`}
              tooltip="Percentage of tasks that completed successfully (merged) vs those that failed, were cancelled, or stopped. Higher = more reliable AI execution."
            />
            <StatCard
              label="Review Pass Rate"
              value={formatPercent(stats.reviewPassRate)}
              sub={`${stats.reviewPassCount} / ${stats.reviewTotalCount} reviews`}
              tooltip="Percentage of AI code reviews that passed on first attempt without requesting changes. Higher = better first-draft quality."
            />
            <StatCard
              label="Tasks This Week"
              value={String(stats.tasksCompletedThisWeek)}
              sub={`${stats.tasksCompletedToday} tasks today`}
              tooltip="Number of tasks that reached merged status in the last 7 days."
            />
            <StatCard
              label="Avg Task Duration"
              value={getAvgCycleTimeDisplay(stats)}
              sub="start to merge"
              tooltip="Average wall-clock time a task takes from entering the pipeline to merge completion. Includes queue time, AI execution, review, and merge stages. Lower is better — most time is typically spent waiting (queue/escalation), not in active execution."
            />
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
              <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
                <DetailCard>
                  <TrendChart
                    title="Weekly Throughput (tasks)"
                    data={trends.weeklyThroughput}
                  />
                </DetailCard>
                <DetailCard>
                  <TrendChart
                    title="Avg Cycle Time"
                    data={trends.weeklyCycleTime}
                    valueFormatter={(v) => formatMinutesHuman(v * 60)}
                  />
                </DetailCard>
              </div>
              <DetailCard>
                <TrendChart
                  title="Agent Success Rate (%)"
                  data={trends.weeklySuccessRate}
                  valueFormatter={(v) => `${Math.round(v * 100)}%`}
                  color="#34d399"
                />
              </DetailCard>
            </div>
          )}

          {/* Cycle time + column dwell time */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
            <CycleTimeBreakdown phases={stats.cycleTimeBreakdown} />
            <ColumnDwellTimeBreakdown dwellTimes={stats.columnDwellTimes} />
          </div>
        </section>

        {/* Divider */}
        <div className="flex items-center gap-3">
          <div
            className="h-px flex-1"
            style={{ backgroundColor: "rgba(255,255,255,0.08)" }}
          />
          <span
            className="text-[11px] font-semibold uppercase tracking-wider"
            style={{ color: "rgba(255,255,255,0.25)", letterSpacing: "0.08em" }}
          >
            Estimates
          </span>
          <div
            className="h-px flex-1"
            style={{ backgroundColor: "rgba(255,255,255,0.08)" }}
          />
        </div>

        {/* Section 2: Effort Estimation */}
        <section className="flex flex-col gap-4">
          <SectionTitle>Effort Estimation</SectionTitle>

          {showEme && stats.eme ? (
            <EffortEstimationPanel
              lowHours={stats.eme.lowHours}
              highHours={stats.eme.highHours}
              taskCount={stats.eme.taskCount}
              earliestTaskDate={stats.eme.earliestTaskDate}
              latestTaskDate={stats.eme.latestTaskDate}
              projectId={projectId}
            />
          ) : (
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
          )}
        </section>

        {/* Export row */}
        <div className="flex items-center gap-3 pt-2">
          <button
            onClick={() => exportJSON(stats, trends)}
            className="flex items-center gap-2 rounded-lg px-3 py-2 text-[12px] font-medium transition-colors"
            style={{
              backgroundColor: "hsl(220 10% 14%)",
              color: "rgba(255,255,255,0.7)",
            }}
          >
            <Download size={13} />
            Download JSON
          </button>
          <button
            onClick={() => exportCSV(trends)}
            className="flex items-center gap-2 rounded-lg px-3 py-2 text-[12px] font-medium transition-colors"
            style={{
              backgroundColor: "hsl(220 10% 14%)",
              color: "rgba(255,255,255,0.7)",
            }}
          >
            <Download size={13} />
            Download CSV
          </button>
          <CopyMarkdownButton stats={stats} />
        </div>
      </div>
    </div>
  );
}
