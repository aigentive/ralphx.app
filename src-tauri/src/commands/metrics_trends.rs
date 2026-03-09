// Private SQL query helpers for weekly trend data.
// Called by compute_project_trends in metrics_commands.rs.

use rusqlite::params;

use crate::commands::metrics_types::{ProjectTrends, WeeklyDataPoint};
use crate::error::{AppError, AppResult};

// ─── Week start helpers ───────────────────────────────────────────────────────

/// Compute the SQLite weekday target for `date(x, 'weekday N', '-6 days')`.
/// Given a desired week start day (0=Sunday .. 6=Saturday), returns N such that
/// `date(x, 'weekday N', '-6 days')` yields the most recent occurrence of that day.
fn weekday_target(week_start_day: u8) -> u8 {
    (week_start_day + 6) % 7
}

/// Validate week_start_day is in range 0..=6.
fn validate_week_start_day(week_start_day: u8) -> AppResult<()> {
    if week_start_day > 6 {
        return Err(AppError::Database(format!(
            "week_start_day must be 0-6, got {}",
            week_start_day
        )));
    }
    Ok(())
}

// ─── Trend queries ────────────────────────────────────────────────────────────

/// Weekly throughput: count of tasks merged per week, last 12 weeks.
/// Uses a recursive CTE to generate all 12 weeks so empty weeks appear as 0.
pub(crate) fn query_weekly_throughput(
    conn: &rusqlite::Connection,
    project_id: &str,
    week_start_day: u8,
) -> AppResult<Vec<WeeklyDataPoint>> {
    validate_week_start_day(week_start_day)?;
    let wt = weekday_target(week_start_day);

    let sql = format!(
        "WITH RECURSIVE weeks(week_start) AS (
          SELECT date('now', 'weekday {wt}', '-6 days', '-364 days')
          UNION ALL
          SELECT date(week_start, '+7 days')
          FROM weeks WHERE week_start < date('now', 'weekday {wt}', '-6 days')
        )
        SELECT
          w.week_start,
          COALESCE(COUNT(t.id), 0) as completed_count
        FROM weeks w
        LEFT JOIN tasks t ON
          t.project_id = ?1
          AND t.internal_status = 'merged'
          AND date(t.updated_at) >= w.week_start
          AND date(t.updated_at) < date(w.week_start, '+7 days')
        WHERE w.week_start <= date('now')
        GROUP BY w.week_start
        ORDER BY w.week_start"
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| AppError::Database(e.to_string()))?;
    let rows = stmt
        .query_map(params![project_id], |row| {
            let week_start: String = row.get(0)?;
            let completed_count: i64 = row.get(1)?;
            Ok(WeeklyDataPoint {
                week_start,
                value: completed_count as f64,
                sample_size: completed_count,
            })
        })
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut points = Vec::new();
    for row in rows {
        points.push(row.map_err(|e| AppError::Database(e.to_string()))?);
    }
    // Trim leading zero-value weeks so the chart starts at the first week with data
    let first_nonzero = points.iter().position(|p| p.value > 0.0);
    if let Some(idx) = first_nonzero {
        points = points.split_off(idx);
    } else {
        points.clear();
    }
    Ok(points)
}

/// Weekly average cycle time in hours for merged tasks, last 12 weeks.
pub(crate) fn query_weekly_cycle_time(
    conn: &rusqlite::Connection,
    project_id: &str,
    week_start_day: u8,
) -> AppResult<Vec<WeeklyDataPoint>> {
    validate_week_start_day(week_start_day)?;
    let wt = weekday_target(week_start_day);

    let sql = format!(
        "WITH merged_tasks AS (
            SELECT id, updated_at FROM tasks
            WHERE project_id = ?1
              AND internal_status = 'merged'
              AND updated_at >= datetime('now', '-365 days')
        ),
        transitions AS (
            SELECT
                h.task_id,
                h.to_status,
                h.created_at,
                LAG(h.created_at) OVER (PARTITION BY h.task_id ORDER BY h.created_at) AS prev_at,
                LAG(h.to_status)  OVER (PARTITION BY h.task_id ORDER BY h.created_at) AS prev_status
            FROM task_state_history h
            WHERE h.task_id IN (SELECT id FROM merged_tasks)
        ),
        task_exec_hours AS (
            SELECT
                tr.task_id,
                SUM((julianday(tr.created_at) - julianday(tr.prev_at)) * 24.0) AS exec_hours
            FROM transitions tr
            WHERE tr.prev_at IS NOT NULL
              AND tr.prev_status IN ('executing', 're_executing')
            GROUP BY tr.task_id
        )
        SELECT
          date(mt.updated_at, 'weekday {wt}', '-6 days') as week_start,
          AVG(te.exec_hours) as avg_hours,
          COUNT(*) as sample_size
        FROM merged_tasks mt
        JOIN task_exec_hours te ON te.task_id = mt.id
        GROUP BY week_start
        HAVING week_start <= date('now')
        ORDER BY week_start"
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| AppError::Database(e.to_string()))?;
    let rows = stmt
        .query_map(params![project_id], |row| {
            let week_start: String = row.get(0)?;
            let avg_hours: Option<f64> = row.get(1)?;
            let sample_size: i64 = row.get(2)?;
            Ok(WeeklyDataPoint {
                week_start,
                value: avg_hours.unwrap_or(0.0),
                sample_size,
            })
        })
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut points = Vec::new();
    for row in rows {
        points.push(row.map_err(|e| AppError::Database(e.to_string()))?);
    }
    Ok(points)
}

/// Weekly average pipeline cycle time in hours for merged tasks, last 12 weeks.
/// Unlike `query_weekly_cycle_time` which only counts executing/re_executing phases,
/// this sums ALL non-terminal phase durations (excludes merged/cancelled/failed/stopped/paused/blocked).
pub(crate) fn query_weekly_pipeline_cycle_time(
    conn: &rusqlite::Connection,
    project_id: &str,
    week_start_day: u8,
) -> AppResult<Vec<WeeklyDataPoint>> {
    validate_week_start_day(week_start_day)?;
    let wt = weekday_target(week_start_day);

    let sql = format!(
        "WITH merged_tasks AS (
            SELECT id, updated_at FROM tasks
            WHERE project_id = ?1
              AND internal_status = 'merged'
              AND updated_at >= datetime('now', '-365 days')
        ),
        transitions AS (
            SELECT
                h.task_id,
                h.to_status,
                h.created_at,
                LAG(h.created_at) OVER (PARTITION BY h.task_id ORDER BY h.created_at) AS prev_at,
                LAG(h.to_status)  OVER (PARTITION BY h.task_id ORDER BY h.created_at) AS prev_status
            FROM task_state_history h
            WHERE h.task_id IN (SELECT id FROM merged_tasks)
        ),
        task_pipeline_hours AS (
            SELECT
                tr.task_id,
                SUM((julianday(tr.created_at) - julianday(tr.prev_at)) * 24.0) AS pipeline_hours
            FROM transitions tr
            WHERE tr.prev_at IS NOT NULL
              AND tr.prev_status NOT IN ('merged', 'cancelled', 'failed', 'stopped', 'paused', 'blocked')
            GROUP BY tr.task_id
        )
        SELECT
          date(mt.updated_at, 'weekday {wt}', '-6 days') as week_start,
          AVG(te.pipeline_hours) as avg_hours,
          COUNT(*) as sample_size
        FROM merged_tasks mt
        JOIN task_pipeline_hours te ON te.task_id = mt.id
        GROUP BY week_start
        HAVING week_start <= date('now')
        ORDER BY week_start"
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| AppError::Database(e.to_string()))?;
    let rows = stmt
        .query_map(params![project_id], |row| {
            let week_start: String = row.get(0)?;
            let avg_hours: Option<f64> = row.get(1)?;
            let sample_size: i64 = row.get(2)?;
            Ok(WeeklyDataPoint {
                week_start,
                value: avg_hours.unwrap_or(0.0),
                sample_size,
            })
        })
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut points = Vec::new();
    for row in rows {
        points.push(row.map_err(|e| AppError::Database(e.to_string()))?);
    }
    Ok(points)
}

/// Weekly success rate: percentage of merged vs total terminal tasks, last 12 weeks.
pub(crate) fn query_weekly_success_rate(
    conn: &rusqlite::Connection,
    project_id: &str,
    week_start_day: u8,
) -> AppResult<Vec<WeeklyDataPoint>> {
    validate_week_start_day(week_start_day)?;
    let wt = weekday_target(week_start_day);

    let sql = format!(
        "SELECT
          date(t.updated_at, 'weekday {wt}', '-6 days') as week_start,
          CAST(SUM(CASE WHEN t.internal_status = 'merged' THEN 1 ELSE 0 END) AS FLOAT) /
            NULLIF(COUNT(*), 0) as success_rate,
          COUNT(*) as sample_size
        FROM tasks t
        WHERE t.project_id = ?1
          AND t.internal_status IN ('merged', 'failed', 'cancelled', 'stopped')
          AND t.updated_at >= datetime('now', '-365 days')
        GROUP BY week_start
        HAVING week_start <= date('now')
        ORDER BY week_start"
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| AppError::Database(e.to_string()))?;
    let rows = stmt
        .query_map(params![project_id], |row| {
            let week_start: String = row.get(0)?;
            let success_rate: Option<f64> = row.get(1)?;
            let sample_size: i64 = row.get(2)?;
            Ok(WeeklyDataPoint {
                week_start,
                value: success_rate.unwrap_or(0.0),
                sample_size,
            })
        })
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut points = Vec::new();
    for row in rows {
        points.push(row.map_err(|e| AppError::Database(e.to_string()))?);
    }
    Ok(points)
}

// ─── Orchestrator ─────────────────────────────────────────────────────────────

/// Run all trend queries synchronously inside a single `db.run` closure.
/// `week_start_day`: 0=Sunday, 1=Monday, ..., 6=Saturday.
pub fn compute_project_trends(
    conn: &rusqlite::Connection,
    project_id: &str,
    week_start_day: u8,
) -> AppResult<ProjectTrends> {
    let weekly_throughput = query_weekly_throughput(conn, project_id, week_start_day)?;
    let weekly_cycle_time = query_weekly_cycle_time(conn, project_id, week_start_day)?;
    let weekly_pipeline_cycle_time = query_weekly_pipeline_cycle_time(conn, project_id, week_start_day)?;
    let weekly_success_rate = query_weekly_success_rate(conn, project_id, week_start_day)?;

    Ok(ProjectTrends {
        weekly_throughput,
        weekly_cycle_time,
        weekly_pipeline_cycle_time,
        weekly_success_rate,
    })
}
