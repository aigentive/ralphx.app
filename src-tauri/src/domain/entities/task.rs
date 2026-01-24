// Task entity - represents a task in RalphX
// Contains task metadata, status, and timestamps

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{InternalStatus, ProjectId, TaskId};

/// A task managed by RalphX
/// Tasks belong to a project and have an internal status that follows the state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for this task
    pub id: TaskId,
    /// The project this task belongs to
    pub project_id: ProjectId,
    /// Category of the task (e.g., "feature", "bug", "setup", "testing")
    pub category: String,
    /// Short title describing the task
    pub title: String,
    /// Optional longer description with details
    pub description: Option<String>,
    /// Priority for ordering (higher = more important, 0 = default)
    pub priority: i32,
    /// Current internal status (follows state machine)
    pub internal_status: InternalStatus,
    /// When the task was created
    pub created_at: DateTime<Utc>,
    /// When the task was last updated
    pub updated_at: DateTime<Utc>,
    /// When execution started (first time status became Executing)
    pub started_at: Option<DateTime<Utc>>,
    /// When the task was completed (status became Approved)
    pub completed_at: Option<DateTime<Utc>>,
}

impl Task {
    /// Creates a new task with the given project_id and title
    /// Uses sensible defaults:
    /// - category: "feature"
    /// - internal_status: Backlog
    /// - priority: 0
    /// - timestamps set to now
    pub fn new(project_id: ProjectId, title: String) -> Self {
        let now = Utc::now();
        Self {
            id: TaskId::new(),
            project_id,
            category: "feature".to_string(),
            title,
            description: None,
            priority: 0,
            internal_status: InternalStatus::Backlog,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
        }
    }

    /// Creates a new task with a specific category
    pub fn new_with_category(project_id: ProjectId, title: String, category: String) -> Self {
        let mut task = Self::new(project_id, title);
        task.category = category;
        task
    }

    /// Updates the updated_at timestamp to now
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Sets the description and updates the timestamp
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.touch();
    }

    /// Sets the priority and updates the timestamp
    pub fn set_priority(&mut self, priority: i32) {
        self.priority = priority;
        self.touch();
    }

    /// Returns true if this task is in a terminal state (Approved, Failed, or Cancelled)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.internal_status,
            InternalStatus::Approved | InternalStatus::Failed | InternalStatus::Cancelled
        )
    }

    /// Returns true if this task is currently being worked on
    pub fn is_active(&self) -> bool {
        matches!(
            self.internal_status,
            InternalStatus::Executing
                | InternalStatus::ExecutionDone
                | InternalStatus::QaRefining
                | InternalStatus::QaTesting
                | InternalStatus::PendingReview
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Task Creation Tests =====

    #[test]
    fn task_new_creates_with_defaults() {
        let project_id = ProjectId::new();
        let task = Task::new(project_id.clone(), "Test Task".to_string());

        assert_eq!(task.title, "Test Task");
        assert_eq!(task.project_id, project_id);
        assert_eq!(task.category, "feature");
        assert!(task.description.is_none());
        assert_eq!(task.priority, 0);
        assert_eq!(task.internal_status, InternalStatus::Backlog);
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn task_new_generates_unique_id() {
        let project_id = ProjectId::new();
        let task1 = Task::new(project_id.clone(), "Task 1".to_string());
        let task2 = Task::new(project_id, "Task 2".to_string());

        assert_ne!(task1.id, task2.id);
    }

    #[test]
    fn task_new_sets_timestamps() {
        let before = Utc::now();
        let task = Task::new(ProjectId::new(), "Test".to_string());
        let after = Utc::now();

        assert!(task.created_at >= before);
        assert!(task.created_at <= after);
        assert!(task.updated_at >= before);
        assert!(task.updated_at <= after);
        assert_eq!(task.created_at, task.updated_at);
    }

    #[test]
    fn task_new_defaults_to_backlog_status() {
        let task = Task::new(ProjectId::new(), "Test".to_string());
        assert_eq!(task.internal_status, InternalStatus::Backlog);
    }

    #[test]
    fn task_new_with_category_sets_category() {
        let task = Task::new_with_category(
            ProjectId::new(),
            "Bug Fix".to_string(),
            "bug".to_string(),
        );

        assert_eq!(task.category, "bug");
        assert_eq!(task.title, "Bug Fix");
        assert_eq!(task.internal_status, InternalStatus::Backlog);
    }

    // ===== Task Method Tests =====

    #[test]
    fn task_touch_updates_timestamp() {
        let mut task = Task::new(ProjectId::new(), "Test".to_string());
        let original_updated = task.updated_at;
        let original_created = task.created_at;

        // Small delay to ensure time difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        task.touch();

        assert_eq!(task.created_at, original_created);
        assert!(task.updated_at > original_updated);
    }

    #[test]
    fn task_set_description_updates_and_touches() {
        let mut task = Task::new(ProjectId::new(), "Test".to_string());
        let original_updated = task.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        task.set_description(Some("A detailed description".to_string()));

        assert_eq!(task.description, Some("A detailed description".to_string()));
        assert!(task.updated_at > original_updated);
    }

    #[test]
    fn task_set_description_to_none() {
        let mut task = Task::new(ProjectId::new(), "Test".to_string());
        task.description = Some("Initial description".to_string());

        task.set_description(None);

        assert!(task.description.is_none());
    }

    #[test]
    fn task_set_priority_updates_and_touches() {
        let mut task = Task::new(ProjectId::new(), "Test".to_string());
        let original_updated = task.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        task.set_priority(10);

        assert_eq!(task.priority, 10);
        assert!(task.updated_at > original_updated);
    }

    #[test]
    fn task_set_priority_negative() {
        let mut task = Task::new(ProjectId::new(), "Test".to_string());
        task.set_priority(-5);
        assert_eq!(task.priority, -5);
    }

    // ===== Task State Helper Tests =====

    #[test]
    fn task_is_terminal_for_approved() {
        let mut task = Task::new(ProjectId::new(), "Test".to_string());
        task.internal_status = InternalStatus::Approved;
        assert!(task.is_terminal());
    }

    #[test]
    fn task_is_terminal_for_failed() {
        let mut task = Task::new(ProjectId::new(), "Test".to_string());
        task.internal_status = InternalStatus::Failed;
        assert!(task.is_terminal());
    }

    #[test]
    fn task_is_terminal_for_cancelled() {
        let mut task = Task::new(ProjectId::new(), "Test".to_string());
        task.internal_status = InternalStatus::Cancelled;
        assert!(task.is_terminal());
    }

    #[test]
    fn task_is_not_terminal_for_active_states() {
        let mut task = Task::new(ProjectId::new(), "Test".to_string());

        for status in &[
            InternalStatus::Backlog,
            InternalStatus::Ready,
            InternalStatus::Executing,
            InternalStatus::ExecutionDone,
            InternalStatus::QaRefining,
            InternalStatus::QaTesting,
            InternalStatus::QaPassed,
            InternalStatus::QaFailed,
            InternalStatus::PendingReview,
            InternalStatus::RevisionNeeded,
            InternalStatus::Blocked,
        ] {
            task.internal_status = *status;
            assert!(
                !task.is_terminal(),
                "{:?} should not be terminal",
                status
            );
        }
    }

    #[test]
    fn task_is_active_for_working_states() {
        let mut task = Task::new(ProjectId::new(), "Test".to_string());

        let active_states = [
            InternalStatus::Executing,
            InternalStatus::ExecutionDone,
            InternalStatus::QaRefining,
            InternalStatus::QaTesting,
            InternalStatus::PendingReview,
        ];

        for status in &active_states {
            task.internal_status = *status;
            assert!(task.is_active(), "{:?} should be active", status);
        }
    }

    #[test]
    fn task_is_not_active_for_idle_states() {
        let mut task = Task::new(ProjectId::new(), "Test".to_string());

        let idle_states = [
            InternalStatus::Backlog,
            InternalStatus::Ready,
            InternalStatus::Blocked,
            InternalStatus::QaPassed,
            InternalStatus::QaFailed,
            InternalStatus::RevisionNeeded,
            InternalStatus::Approved,
            InternalStatus::Failed,
            InternalStatus::Cancelled,
        ];

        for status in &idle_states {
            task.internal_status = *status;
            assert!(!task.is_active(), "{:?} should not be active", status);
        }
    }

    // ===== Task Serialization Tests =====

    #[test]
    fn task_serializes_to_json() {
        let task = Task::new(ProjectId::from_string("proj-123".to_string()), "JSON Test".to_string());
        let json = serde_json::to_string(&task).expect("Should serialize");

        assert!(json.contains("\"title\":\"JSON Test\""));
        assert!(json.contains("\"project_id\":\"proj-123\""));
        assert!(json.contains("\"category\":\"feature\""));
        assert!(json.contains("\"internal_status\":\"backlog\""));
        assert!(json.contains("\"priority\":0"));
    }

    #[test]
    fn task_deserializes_from_json() {
        let json = r#"{
            "id": "task-id-123",
            "project_id": "proj-id-456",
            "category": "bug",
            "title": "Fix the bug",
            "description": "Detailed description here",
            "priority": 5,
            "internal_status": "executing",
            "created_at": "2025-01-24T12:00:00Z",
            "updated_at": "2025-01-24T13:00:00Z",
            "started_at": "2025-01-24T12:30:00Z",
            "completed_at": null
        }"#;

        let task: Task = serde_json::from_str(json).expect("Should deserialize");

        assert_eq!(task.id.as_str(), "task-id-123");
        assert_eq!(task.project_id.as_str(), "proj-id-456");
        assert_eq!(task.category, "bug");
        assert_eq!(task.title, "Fix the bug");
        assert_eq!(task.description, Some("Detailed description here".to_string()));
        assert_eq!(task.priority, 5);
        assert_eq!(task.internal_status, InternalStatus::Executing);
        assert!(task.started_at.is_some());
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn task_deserializes_with_null_optionals() {
        let json = r#"{
            "id": "task-id",
            "project_id": "proj-id",
            "category": "feature",
            "title": "Minimal",
            "description": null,
            "priority": 0,
            "internal_status": "backlog",
            "created_at": "2025-01-24T12:00:00Z",
            "updated_at": "2025-01-24T12:00:00Z",
            "started_at": null,
            "completed_at": null
        }"#;

        let task: Task = serde_json::from_str(json).expect("Should deserialize");

        assert!(task.description.is_none());
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn task_roundtrip_serialization() {
        let mut original = Task::new(ProjectId::new(), "Roundtrip".to_string());
        original.description = Some("Test description".to_string());
        original.priority = 42;
        original.internal_status = InternalStatus::Ready;

        let json = serde_json::to_string(&original).expect("Should serialize");
        let restored: Task = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(original.id, restored.id);
        assert_eq!(original.project_id, restored.project_id);
        assert_eq!(original.category, restored.category);
        assert_eq!(original.title, restored.title);
        assert_eq!(original.description, restored.description);
        assert_eq!(original.priority, restored.priority);
        assert_eq!(original.internal_status, restored.internal_status);
    }

    // ===== Task Clone Tests =====

    #[test]
    fn task_clone_works() {
        let original = Task::new(ProjectId::new(), "Clone Test".to_string());
        let cloned = original.clone();

        assert_eq!(original.id, cloned.id);
        assert_eq!(original.title, cloned.title);
        assert_eq!(original.project_id, cloned.project_id);
        assert_eq!(original.internal_status, cloned.internal_status);
    }

    #[test]
    fn task_clone_is_independent() {
        let original = Task::new(ProjectId::new(), "Independent".to_string());
        let mut cloned = original.clone();

        cloned.touch();

        // Original should be unchanged
        assert_ne!(original.updated_at, cloned.updated_at);
    }

    // ===== Task Category Tests =====

    #[test]
    fn task_supports_various_categories() {
        let project_id = ProjectId::new();
        let categories = ["feature", "bug", "setup", "testing", "refactor", "docs"];

        for category in &categories {
            let task = Task::new_with_category(
                project_id.clone(),
                format!("{} task", category),
                category.to_string(),
            );
            assert_eq!(task.category, *category);
        }
    }

    // ===== Debug Format Test =====

    #[test]
    fn task_debug_format_works() {
        let task = Task::new(ProjectId::new(), "Debug Test".to_string());
        let debug = format!("{:?}", task);

        assert!(debug.contains("Task"));
        assert!(debug.contains("Debug Test"));
        assert!(debug.contains("feature"));
        assert!(debug.contains("Backlog"));
    }
}
