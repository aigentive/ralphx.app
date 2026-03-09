// Response types shared across metrics commands.
// Extracted from metrics_commands.rs to keep that file under the 500-line limit.

use serde::Serialize;

/// Average time spent in each pipeline phase, derived from LAG() window function.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CycleTimePhase {
    /// Internal status label (e.g. "ready", "executing", "pending_review")
    pub phase: String,
    /// Average minutes spent in this phase across sampled tasks
    pub avg_minutes: f64,
    /// Number of task-transitions that contributed to this average
    pub sample_size: i64,
}

/// Estimated Manual Effort range (low..high hours).
/// Only populated when ≥5 tasks are completed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmeEstimate {
    pub low_hours: f64,
    pub high_hours: f64,
    /// Number of merged tasks used in the estimate
    pub task_count: i64,
    /// ISO date of the earliest merged task in the sample
    pub earliest_task_date: Option<String>,
    /// ISO date of the most recent merged task in the sample
    pub latest_task_date: Option<String>,
}

/// All project metrics returned by the `get_project_stats` command.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStats {
    // ── Throughput ──────────────────────────────────────────────────────────
    /// Total non-archived tasks in the project (used by frontend threshold logic)
    pub task_count: i64,
    pub tasks_completed_today: i64,
    pub tasks_completed_this_week: i64,
    pub tasks_completed_this_month: i64,

    // ── Quality ─────────────────────────────────────────────────────────────
    /// merged / (merged + failed + cancelled + stopped), 0.0 when denominator is 0
    pub agent_success_rate: f64,
    pub agent_success_count: i64,
    pub agent_total_count: i64,

    /// approved / (approved + changes_requested), 0.0 when denominator is 0
    pub review_pass_rate: f64,
    pub review_pass_count: i64,
    pub review_total_count: i64,

    // ── Cycle time ──────────────────────────────────────────────────────────
    /// Per-phase averages over the last 90 days (merged tasks only)
    pub cycle_time_breakdown: Vec<CycleTimePhase>,

    // ── Column dwell time ──────────────────────────────────────────────────
    /// Average time tasks spend in each Kanban column (last 90 days, merged tasks only)
    pub column_dwell_times: Vec<ColumnDwellTime>,

    // ── EME (Estimated Manual Effort) ────────────────────────────────────────
    /// None when < 5 merged tasks exist (insufficient sample)
    pub eme: Option<EmeEstimate>,
}

/// A single weekly data point for trend charts.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyDataPoint {
    /// ISO date string "YYYY-MM-DD" representing the start of the week (Sunday)
    pub week_start: String,
    /// The metric value for this week
    pub value: f64,
    /// Number of tasks/data points that contributed to this value
    pub sample_size: i64,
}

/// Time-series trend data returned by the `get_project_trends` command.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTrends {
    /// Count of tasks merged per week, last 12 weeks
    pub weekly_throughput: Vec<WeeklyDataPoint>,
    /// Average cycle time in hours for merged tasks per week, last 12 weeks
    pub weekly_cycle_time: Vec<WeeklyDataPoint>,
    /// Percentage of merged vs total terminal tasks per week, last 12 weeks
    pub weekly_success_rate: Vec<WeeklyDataPoint>,
}

/// Average dwell time per Kanban column, derived from task_state_history transitions.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnDwellTime {
    /// Kanban column id (e.g. "ready", "in_progress", "in_review", "merge", "done")
    pub column_id: String,
    /// Human-readable column name
    pub column_name: String,
    /// Average minutes tasks spent in this column
    pub avg_minutes: f64,
    /// Number of task-transitions that contributed to this average
    pub sample_size: i64,
}

/// Per-column task distribution metric for the Kanban board.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMetric {
    /// Kanban column id (e.g. "backlog", "ready", "in_progress", "in_review", "done")
    pub column_id: String,
    /// Human-readable column name
    pub column_name: String,
    /// Number of non-archived tasks currently in this column
    pub task_count: i64,
    /// Average age of tasks in this column in hours (0 when task_count is 0)
    pub avg_age_hours: f64,
}

/// Per-task metrics returned by `get_task_metrics`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskMetrics {
    /// Total steps (all statuses)
    pub step_count: i64,
    /// Steps with status = 'completed'
    pub completed_step_count: i64,
    /// Number of review cycles for this task
    pub review_count: i64,
    /// Approved reviews
    pub approved_review_count: i64,
    /// Time spent in 'executing' or 're_executing' phases, in minutes (0 when no history)
    pub execution_minutes: f64,
    /// Total elapsed time from task creation to now (or merge), in hours
    pub total_age_hours: f64,
}
