/**
 * Project stats types
 *
 * These types are used by the ProjectStatsCard component and the
 * useProjectStats hook. The Tauri backend exposes get_project_stats
 * which returns data shaped like ProjectStats.
 *
 * All fields are camelCase — backend uses #[serde(rename_all = "camelCase")].
 */

import { z } from "zod";

// ============================================================================
// Schemas
// ============================================================================

/**
 * Average time spent in a single pipeline phase (from LAG() window queries)
 */
export const CycleTimePhaseSchema = z.object({
  phase: z.string(),
  avgMinutes: z.number(),
  sampleSize: z.number(),
});

/**
 * Estimated Manual Effort range (low..high hours)
 * Only present when ≥5 tasks are merged
 */
export const EmeEstimateSchema = z.object({
  lowHours: z.number(),
  highHours: z.number(),
  taskCount: z.number(),
  earliestTaskDate: z.string().nullable(),
  latestTaskDate: z.string().nullable(),
});

export const ColumnDwellTimeSchema = z.object({
  columnId: z.string(),
  columnName: z.string(),
  avgMinutes: z.number(),
  sampleSize: z.number(),
});

export const ProjectStatsSchema = z.object({
  taskCount: z.number(),
  tasksCompletedToday: z.number(),
  tasksCompletedThisWeek: z.number(),
  tasksCompletedThisMonth: z.number(),
  agentSuccessRate: z.number(),
  agentSuccessCount: z.number(),
  agentTotalCount: z.number(),
  reviewPassRate: z.number(),
  reviewPassCount: z.number(),
  reviewTotalCount: z.number(),
  cycleTimeBreakdown: z.array(CycleTimePhaseSchema),
  columnDwellTimes: z.array(ColumnDwellTimeSchema),
  eme: EmeEstimateSchema.nullable(),
});

export const WeeklyDataPointSchema = z.object({
  weekStart: z.string(),
  value: z.number(),
  sampleSize: z.number(),
});

export const ProjectTrendsSchema = z.object({
  weeklyThroughput: z.array(WeeklyDataPointSchema),
  weeklyCycleTime: z.array(WeeklyDataPointSchema),
  weeklySuccessRate: z.array(WeeklyDataPointSchema),
});

// ============================================================================
// Types
// ============================================================================

export type CycleTimePhase = z.infer<typeof CycleTimePhaseSchema>;
export type ColumnDwellTime = z.infer<typeof ColumnDwellTimeSchema>;
export type EmeEstimate = z.infer<typeof EmeEstimateSchema>;
export type ProjectStats = z.infer<typeof ProjectStatsSchema>;
export type WeeklyDataPoint = z.infer<typeof WeeklyDataPointSchema>;
export type ProjectTrends = z.infer<typeof ProjectTrendsSchema>;

// ============================================================================
// Metrics config
// ============================================================================

/**
 * Per-project calibration config for EME (Estimated Manual Effort) computation.
 * Persisted via get_metrics_config / save_metrics_config Tauri commands.
 */
export const MetricsConfigSchema = z.object({
  simpleBaseHours: z.number().min(0.5).max(40),
  mediumBaseHours: z.number().min(0.5).max(40),
  complexBaseHours: z.number().min(0.5).max(40),
  calendarFactor: z.number().min(1).max(3),
  workingDaysPerWeek: z.number().int().min(1).max(7),
});

export type MetricsConfig = z.infer<typeof MetricsConfigSchema>;

export const DEFAULT_METRICS_CONFIG: MetricsConfig = {
  simpleBaseHours: 1,
  mediumBaseHours: 2,
  complexBaseHours: 4,
  calendarFactor: 1.3,
  workingDaysPerWeek: 5,
};
