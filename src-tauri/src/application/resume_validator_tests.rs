use super::*;
use crate::domain::entities::{ProjectId, Task};
use crate::domain::services::MemoryRunningAgentRegistry;

fn create_test_validator() -> ResumeValidator {
    let registry: Arc<dyn RunningAgentRegistry> = Arc::new(MemoryRunningAgentRegistry::new());
    ResumeValidator::new(registry)
}

fn create_test_task() -> Task {
    Task::new(ProjectId::new(), "Test Task".to_string())
}

fn create_test_project() -> Project {
    Project::new("Test Project".to_string(), "/tmp/test".to_string())
}

#[test]
fn test_validation_result_new_is_valid() {
    let result = ResumeValidationResult::new();
    assert!(result.is_valid);
    assert!(result.warnings.is_empty());
    assert!(result.errors.is_empty());
}

#[test]
fn test_validation_result_with_warning() {
    let result = ResumeValidationResult::new().with_warning("Test warning");
    assert!(result.is_valid);
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0], "Test warning");
}

#[test]
fn test_validation_result_with_error() {
    let result = ResumeValidationResult::new().with_error("Test error");
    assert!(!result.is_valid);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0], "Test error");
}

#[test]
fn test_validation_result_merge() {
    let mut result1 = ResumeValidationResult::new().with_warning("Warning 1");
    let result2 = ResumeValidationResult::new()
        .with_warning("Warning 2")
        .with_error("Error 1");

    result1.merge(&result2);

    assert!(!result1.is_valid);
    assert_eq!(result1.warnings.len(), 2);
    assert_eq!(result1.errors.len(), 1);
}

#[tokio::test]
async fn test_validate_task_without_branch() {
    let validator = create_test_validator();
    let task = create_test_task();
    let project = create_test_project();

    let result = validator.validate(&task, &project, None).await.unwrap();

    // Task without branch should validate (no git isolation)
    assert!(result.is_valid);
}

#[tokio::test]
async fn test_cleanup_orphan_agents_no_agents() {
    let registry: Arc<dyn RunningAgentRegistry> = Arc::new(MemoryRunningAgentRegistry::new());
    let validator = ResumeValidator::new(Arc::clone(&registry));
    let task = create_test_task();

    let result = validator.cleanup_orphan_agents(&task).await;

    assert!(result.is_valid);
    assert!(result.warnings.is_empty());
}

#[tokio::test]
async fn test_cleanup_orphan_agents_with_running_agent() {
    let registry: Arc<dyn RunningAgentRegistry> = Arc::new(MemoryRunningAgentRegistry::new());
    let validator = ResumeValidator::new(Arc::clone(&registry));
    let task = create_test_task();

    // Register a running agent
    let key = RunningAgentKey::new("task_execution", task.id.as_str());
    registry
        .register(
            key.clone(),
            12345,
            "conv-123".to_string(),
            "run-123".to_string(),
            None,
            None,
        )
        .await;

    let result = validator.cleanup_orphan_agents(&task).await;

    assert!(result.is_valid);
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].contains("Stopped 1 orphan agent"));

    // Agent should be unregistered
    assert!(!registry.is_running(&key).await);
}

#[tokio::test]
async fn test_cleanup_multiple_orphan_agents() {
    let registry: Arc<dyn RunningAgentRegistry> = Arc::new(MemoryRunningAgentRegistry::new());
    let validator = ResumeValidator::new(Arc::clone(&registry));
    let task = create_test_task();

    // Register multiple agents for different contexts
    for context_type in &["task_execution", "review"] {
        let key = RunningAgentKey::new(*context_type, task.id.as_str());
        registry
            .register(
                key,
                12345,
                "conv-123".to_string(),
                "run-123".to_string(),
                None,
                None,
            )
            .await;
    }

    let result = validator.cleanup_orphan_agents(&task).await;

    assert!(result.is_valid);
    assert!(result.warnings[0].contains("Stopped 2 orphan agent"));
}

#[test]
fn test_truncate_status_output_short() {
    let validator = create_test_validator();
    let status = "M file1.txt\nM file2.txt";
    let truncated = validator.truncate_status_output(status);
    assert_eq!(truncated, status);
}

#[test]
fn test_truncate_status_output_long() {
    let validator = create_test_validator();
    let lines: Vec<String> = (0..20).map(|i| format!("M file{}.txt", i)).collect();
    let status = lines.join("\n");
    let truncated = validator.truncate_status_output(&status);

    assert!(truncated.contains("... and 10 more files"));
    assert!(!truncated.contains("file19.txt"));
}
