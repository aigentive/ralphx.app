use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::entities::{IdeationSession, InternalStatus, StepProgressSummary, Task};
use crate::repositories::StatusTransition;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningProcess {
    pub task_id: String,
    pub title: String,
    pub internal_status: String,
    pub step_progress: Option<StepProgressSummary>,
    pub elapsed_seconds: Option<i64>,
    pub trigger_origin: Option<String>,
    pub task_branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningIdeationSession {
    pub session_id: String,
    pub title: String,
    pub elapsed_seconds: Option<i64>,
    pub team_mode: Option<String>,
    pub is_generating: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningProcessesResponse {
    pub processes: Vec<RunningProcess>,
    pub ideation_sessions: Vec<RunningIdeationSession>,
}

pub fn ideation_session_title(title: Option<&str>) -> String {
    title.unwrap_or("Untitled Session").to_string()
}

pub fn elapsed_seconds_since(timestamp: DateTime<Utc>, now: DateTime<Utc>) -> i64 {
    now.signed_duration_since(timestamp).num_seconds()
}

pub fn elapsed_seconds_for_status(
    history: &[StatusTransition],
    current_status: InternalStatus,
    now: DateTime<Utc>,
) -> Option<i64> {
    history
        .iter()
        .rev()
        .find(|transition| transition.to == current_status)
        .map(|transition| elapsed_seconds_since(transition.timestamp, now))
}

pub fn build_running_ideation_session(
    session_id: String,
    session: &IdeationSession,
    is_generating: bool,
    now: DateTime<Utc>,
) -> RunningIdeationSession {
    RunningIdeationSession {
        session_id,
        title: ideation_session_title(session.title.as_deref()),
        elapsed_seconds: Some(elapsed_seconds_since(session.created_at, now)),
        team_mode: session.team_mode.clone(),
        is_generating,
    }
}

pub fn build_running_process(
    task: &Task,
    step_progress: Option<StepProgressSummary>,
    elapsed_seconds: Option<i64>,
    trigger_origin: Option<String>,
) -> RunningProcess {
    RunningProcess {
        task_id: task.id.as_str().to_string(),
        title: task.title.clone(),
        internal_status: task.internal_status.as_str().to_string(),
        step_progress,
        elapsed_seconds,
        trigger_origin,
        task_branch: task.task_branch.clone(),
    }
}

#[cfg(test)]
#[path = "running_views_tests.rs"]
mod tests;
