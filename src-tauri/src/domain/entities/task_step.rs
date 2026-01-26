// TaskStep entity - represents a step within a task execution
// Provides deterministic progress tracking for worker agents

use chrono::{DateTime, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};

use super::{TaskId, TaskStepId};

/// Status of a task step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStepStatus {
    /// Waiting to be worked on
    Pending,
    /// Currently being executed
    InProgress,
    /// Finished successfully
    Completed,
    /// Not applicable or deferred
    Skipped,
    /// Needs attention
    Failed,
    /// Task was cancelled
    Cancelled,
}

impl TaskStepStatus {
    /// Returns true if this is a terminal status (no further transitions)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStepStatus::Completed
                | TaskStepStatus::Skipped
                | TaskStepStatus::Failed
                | TaskStepStatus::Cancelled
        )
    }

    /// Converts status to database string representation
    pub fn to_db_string(&self) -> &'static str {
        match self {
            TaskStepStatus::Pending => "pending",
            TaskStepStatus::InProgress => "in_progress",
            TaskStepStatus::Completed => "completed",
            TaskStepStatus::Skipped => "skipped",
            TaskStepStatus::Failed => "failed",
            TaskStepStatus::Cancelled => "cancelled",
        }
    }

    /// Parses status from database string representation
    pub fn from_db_string(s: &str) -> Result<Self, String> {
        match s {
            "pending" => Ok(TaskStepStatus::Pending),
            "in_progress" => Ok(TaskStepStatus::InProgress),
            "completed" => Ok(TaskStepStatus::Completed),
            "skipped" => Ok(TaskStepStatus::Skipped),
            "failed" => Ok(TaskStepStatus::Failed),
            "cancelled" => Ok(TaskStepStatus::Cancelled),
            _ => Err(format!("Invalid TaskStepStatus: {}", s)),
        }
    }
}

/// A step within a task execution
/// Tracks discrete checkpoints that agents progress through
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    /// Unique identifier for this step
    pub id: TaskStepId,
    /// The task this step belongs to
    pub task_id: TaskId,
    /// Short title describing the step
    pub title: String,
    /// Optional longer description with details
    pub description: Option<String>,
    /// Current status of the step
    pub status: TaskStepStatus,
    /// Order within the task (0-indexed)
    pub sort_order: i32,
    /// Optional dependency - this step depends on another step completing first
    pub depends_on: Option<TaskStepId>,
    /// Who created this step (e.g., "user", "agent", "proposal", "system")
    pub created_by: String,
    /// Optional note about completion (used for skip/fail reasons or completion notes)
    pub completion_note: Option<String>,
    /// When the step was created
    pub created_at: DateTime<Utc>,
    /// When the step was last updated
    pub updated_at: DateTime<Utc>,
    /// When execution started (status became InProgress)
    pub started_at: Option<DateTime<Utc>>,
    /// When the step reached a terminal state
    pub completed_at: Option<DateTime<Utc>>,
}

impl TaskStep {
    /// Creates a new task step with the given task_id, title, sort_order, and created_by
    /// Uses sensible defaults:
    /// - status: Pending
    /// - timestamps set to now
    /// - no description, dependencies, or completion notes
    pub fn new(task_id: TaskId, title: String, sort_order: i32, created_by: String) -> Self {
        let now = Utc::now();
        Self {
            id: TaskStepId::new(),
            task_id,
            title,
            description: None,
            status: TaskStepStatus::Pending,
            sort_order,
            depends_on: None,
            created_by,
            completion_note: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
        }
    }

    /// Returns true if this step can be started
    /// A step can start if it's in Pending status
    pub fn can_start(&self) -> bool {
        self.status == TaskStepStatus::Pending
    }

    /// Returns true if this step is in a terminal state
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    /// Updates the updated_at timestamp to now
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Deserializes a TaskStep from a SQLite row
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let id: String = row.get(0)?;
        let task_id: String = row.get(1)?;
        let title: String = row.get(2)?;
        let description: Option<String> = row.get(3)?;
        let status_str: String = row.get(4)?;
        let sort_order: i32 = row.get(5)?;
        let depends_on_str: Option<String> = row.get(6)?;
        let created_by: String = row.get(7)?;
        let completion_note: Option<String> = row.get(8)?;
        let created_at_str: String = row.get(9)?;
        let updated_at_str: String = row.get(10)?;
        let started_at_str: Option<String> = row.get(11)?;
        let completed_at_str: Option<String> = row.get(12)?;

        let status = TaskStepStatus::from_db_string(&status_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))?;

        let depends_on = depends_on_str.map(TaskStepId::from_string);

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, Box::new(e)))?
            .with_timezone(&Utc);

        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(10, rusqlite::types::Type::Text, Box::new(e)))?
            .with_timezone(&Utc);

        let started_at = started_at_str
            .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
            .transpose()
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(11, rusqlite::types::Type::Text, Box::new(e)))?;

        let completed_at = completed_at_str
            .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
            .transpose()
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(12, rusqlite::types::Type::Text, Box::new(e)))?;

        Ok(Self {
            id: TaskStepId::from_string(id),
            task_id: TaskId::from_string(task_id),
            title,
            description,
            status,
            sort_order,
            depends_on,
            created_by,
            completion_note,
            created_at,
            updated_at,
            started_at,
            completed_at,
        })
    }
}

/// Summary of step progress for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepProgressSummary {
    /// The task this progress summary is for
    pub task_id: String,
    /// Total number of steps
    pub total: u32,
    /// Number of completed steps
    pub completed: u32,
    /// Number of in-progress steps
    pub in_progress: u32,
    /// Number of pending steps
    pub pending: u32,
    /// Number of skipped steps
    pub skipped: u32,
    /// Number of failed steps
    pub failed: u32,
    /// Current step being worked on (first InProgress step)
    pub current_step: Option<TaskStep>,
    /// Next step to be started (first Pending step)
    pub next_step: Option<TaskStep>,
    /// Percentage complete (completed + skipped) / total * 100
    pub percent_complete: f32,
}

impl StepProgressSummary {
    /// Calculate progress summary from a list of steps
    pub fn from_steps(task_id: &TaskId, steps: &[TaskStep]) -> Self {
        let total = steps.len() as u32;
        let completed = steps.iter().filter(|s| s.status == TaskStepStatus::Completed).count() as u32;
        let in_progress = steps.iter().filter(|s| s.status == TaskStepStatus::InProgress).count() as u32;
        let pending = steps.iter().filter(|s| s.status == TaskStepStatus::Pending).count() as u32;
        let skipped = steps.iter().filter(|s| s.status == TaskStepStatus::Skipped).count() as u32;
        let failed = steps.iter().filter(|s| s.status == TaskStepStatus::Failed).count() as u32;

        let current_step = steps
            .iter()
            .find(|s| s.status == TaskStepStatus::InProgress)
            .cloned();

        let next_step = steps
            .iter()
            .find(|s| s.status == TaskStepStatus::Pending)
            .cloned();

        let percent_complete = if total > 0 {
            ((completed + skipped) as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        Self {
            task_id: task_id.as_str().to_string(),
            total,
            completed,
            in_progress,
            pending,
            skipped,
            failed,
            current_step,
            next_step,
            percent_complete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_step_new_creates_with_defaults() {
        let task_id = TaskId::new();
        let step = TaskStep::new(
            task_id.clone(),
            "Test step".to_string(),
            0,
            "user".to_string(),
        );

        assert_eq!(step.task_id, task_id);
        assert_eq!(step.title, "Test step");
        assert_eq!(step.sort_order, 0);
        assert_eq!(step.created_by, "user");
        assert_eq!(step.status, TaskStepStatus::Pending);
        assert!(step.description.is_none());
        assert!(step.depends_on.is_none());
        assert!(step.completion_note.is_none());
        assert!(step.started_at.is_none());
        assert!(step.completed_at.is_none());
    }

    #[test]
    fn task_step_can_start_only_when_pending() {
        let task_id = TaskId::new();
        let mut step = TaskStep::new(
            task_id,
            "Test".to_string(),
            0,
            "user".to_string(),
        );

        assert!(step.can_start());

        step.status = TaskStepStatus::InProgress;
        assert!(!step.can_start());

        step.status = TaskStepStatus::Completed;
        assert!(!step.can_start());
    }

    #[test]
    fn task_step_is_terminal_for_final_states() {
        let task_id = TaskId::new();
        let mut step = TaskStep::new(
            task_id,
            "Test".to_string(),
            0,
            "user".to_string(),
        );

        assert!(!step.is_terminal());

        step.status = TaskStepStatus::InProgress;
        assert!(!step.is_terminal());

        step.status = TaskStepStatus::Completed;
        assert!(step.is_terminal());

        step.status = TaskStepStatus::Skipped;
        assert!(step.is_terminal());

        step.status = TaskStepStatus::Failed;
        assert!(step.is_terminal());

        step.status = TaskStepStatus::Cancelled;
        assert!(step.is_terminal());
    }

    #[test]
    fn task_step_status_to_db_string() {
        assert_eq!(TaskStepStatus::Pending.to_db_string(), "pending");
        assert_eq!(TaskStepStatus::InProgress.to_db_string(), "in_progress");
        assert_eq!(TaskStepStatus::Completed.to_db_string(), "completed");
        assert_eq!(TaskStepStatus::Skipped.to_db_string(), "skipped");
        assert_eq!(TaskStepStatus::Failed.to_db_string(), "failed");
        assert_eq!(TaskStepStatus::Cancelled.to_db_string(), "cancelled");
    }

    #[test]
    fn task_step_status_from_db_string() {
        assert_eq!(
            TaskStepStatus::from_db_string("pending").unwrap(),
            TaskStepStatus::Pending
        );
        assert_eq!(
            TaskStepStatus::from_db_string("in_progress").unwrap(),
            TaskStepStatus::InProgress
        );
        assert_eq!(
            TaskStepStatus::from_db_string("completed").unwrap(),
            TaskStepStatus::Completed
        );
        assert_eq!(
            TaskStepStatus::from_db_string("skipped").unwrap(),
            TaskStepStatus::Skipped
        );
        assert_eq!(
            TaskStepStatus::from_db_string("failed").unwrap(),
            TaskStepStatus::Failed
        );
        assert_eq!(
            TaskStepStatus::from_db_string("cancelled").unwrap(),
            TaskStepStatus::Cancelled
        );
        assert!(TaskStepStatus::from_db_string("invalid").is_err());
    }

    #[test]
    fn step_progress_summary_from_empty_steps() {
        let task_id = TaskId::new();
        let steps: Vec<TaskStep> = vec![];
        let summary = StepProgressSummary::from_steps(&task_id, &steps);

        assert_eq!(summary.total, 0);
        assert_eq!(summary.completed, 0);
        assert_eq!(summary.pending, 0);
        assert_eq!(summary.percent_complete, 0.0);
        assert!(summary.current_step.is_none());
        assert!(summary.next_step.is_none());
    }

    #[test]
    fn step_progress_summary_calculates_correctly() {
        let task_id = TaskId::new();
        let mut steps = vec![
            TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string()),
            TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string()),
            TaskStep::new(task_id.clone(), "Step 3".to_string(), 2, "user".to_string()),
            TaskStep::new(task_id.clone(), "Step 4".to_string(), 3, "user".to_string()),
        ];

        steps[0].status = TaskStepStatus::Completed;
        steps[1].status = TaskStepStatus::InProgress;
        steps[2].status = TaskStepStatus::Pending;
        steps[3].status = TaskStepStatus::Skipped;

        let summary = StepProgressSummary::from_steps(&task_id, &steps);

        assert_eq!(summary.total, 4);
        assert_eq!(summary.completed, 1);
        assert_eq!(summary.in_progress, 1);
        assert_eq!(summary.pending, 1);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.percent_complete, 50.0); // (1 completed + 1 skipped) / 4 * 100
        assert!(summary.current_step.is_some());
        assert_eq!(summary.current_step.unwrap().title, "Step 2");
        assert!(summary.next_step.is_some());
        assert_eq!(summary.next_step.unwrap().title, "Step 3");
    }

    #[test]
    fn step_progress_summary_handles_all_completed() {
        let task_id = TaskId::new();
        let mut steps = vec![
            TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string()),
            TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string()),
        ];

        steps[0].status = TaskStepStatus::Completed;
        steps[1].status = TaskStepStatus::Completed;

        let summary = StepProgressSummary::from_steps(&task_id, &steps);

        assert_eq!(summary.total, 2);
        assert_eq!(summary.completed, 2);
        assert_eq!(summary.percent_complete, 100.0);
        assert!(summary.current_step.is_none());
        assert!(summary.next_step.is_none());
    }
}
