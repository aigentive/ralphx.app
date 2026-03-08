// Private SQL query helpers for weekly trend data.
// Called by compute_project_trends in metrics_commands.rs.

use rusqlite::params;

use crate::commands::metrics_types::{ProjectTrends, WeeklyDataPoint};
use crate::error::{AppError, AppResult};

// ─── Trend queries ────────────────────────────────────────────────────────────

/// Weekly throughput: count of tasks merged per week, last 12 weeks.
/// Uses a recursive CTE to generate all 12 weeks so empty weeks appear as 0.
pub(crate) fn query_weekly_throughput(
    conn: &rusqlite::Connection,
    project_id: &str,
) -> AppResult<Vec<WeeklyDataPoint>> {
    let sql = "
        WITH RECURSIVE weeks(week_start) AS (
          SELECT date('now', 'weekday 0', '-84 days')
          UNION ALL
          SELECT date(week_start, '+7 days')
          FROM weeks WHERE week_start < date('now', 'weekday 0')
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
        GROUP BY w.week_start
        ORDER BY w.week_start
    ";

    let mut stmt = conn.prepare(sql).map_err(|e| AppError::Database(e.to_string()))?;
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
    Ok(points)
}

/// Weekly average cycle time in hours for merged tasks, last 12 weeks.
pub(crate) fn query_weekly_cycle_time(
    conn: &rusqlite::Connection,
    project_id: &str,
) -> AppResult<Vec<WeeklyDataPoint>> {
    let sql = "
        SELECT
          date(t.updated_at, 'weekday 0', '-6 days') as week_start,
          AVG((julianday(t.updated_at) - julianday(t.created_at)) * 24) as avg_hours,
          COUNT(*) as sample_size
        FROM tasks t
        WHERE t.project_id = ?1
          AND t.internal_status = 'merged'
          AND t.updated_at >= datetime('now', '-84 days')
        GROUP BY week_start
        ORDER BY week_start
    ";

    let mut stmt = conn.prepare(sql).map_err(|e| AppError::Database(e.to_string()))?;
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
) -> AppResult<Vec<WeeklyDataPoint>> {
    let sql = "
        SELECT
          date(t.updated_at, 'weekday 0', '-6 days') as week_start,
          CAST(SUM(CASE WHEN t.internal_status = 'merged' THEN 1 ELSE 0 END) AS FLOAT) /
            NULLIF(COUNT(*), 0) * 100 as success_rate,
          COUNT(*) as sample_size
        FROM tasks t
        WHERE t.project_id = ?1
          AND t.internal_status IN ('merged', 'failed', 'cancelled', 'stopped')
          AND t.updated_at >= datetime('now', '-84 days')
        GROUP BY week_start
        ORDER BY week_start
    ";

    let mut stmt = conn.prepare(sql).map_err(|e| AppError::Database(e.to_string()))?;
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
pub fn compute_project_trends(
    conn: &rusqlite::Connection,
    project_id: &str,
) -> AppResult<ProjectTrends> {
    let weekly_throughput = query_weekly_throughput(conn, project_id)?;
    let weekly_cycle_time = query_weekly_cycle_time(conn, project_id)?;
    let weekly_success_rate = query_weekly_success_rate(conn, project_id)?;

    Ok(ProjectTrends {
        weekly_throughput,
        weekly_cycle_time,
        weekly_success_rate,
    })
}
