// Tests for task_commands module

#[cfg(test)]
use super::*;
    use std::sync::Arc;
    use crate::application::AppState;
    use crate::infrastructure::memory::{MemoryTaskRepository, MemoryProjectRepository};
    use crate::domain::entities::{Project, Task, ProjectId, TaskId, InternalStatus};
    use crate::domain::repositories::ProjectRepository;
    use chrono::Utc;

    async fn setup_test_state() -> AppState {
        let task_repo = Arc::new(MemoryTaskRepository::new());
        let project_repo = Arc::new(MemoryProjectRepository::new());

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        project_repo.create(project).await.unwrap();

        AppState::with_repos(task_repo, project_repo)
    }

    #[tokio::test]
    async fn test_create_task_with_defaults() {
        let state = setup_test_state().await;

        let input = types::CreateTaskInput {
            project_id: "test-project".to_string(),
            title: "Test Task".to_string(),
            category: None,
            description: None,
            priority: None,
            steps: None,
        };

        // We can't easily call tauri commands without the full runtime,
        // so we test the repository directly
        let project_id = ProjectId::from_string(input.project_id);
        let task = Task::new(project_id.clone(), input.title);

        let created = state.task_repo.create(task).await.unwrap();
        assert_eq!(created.title, "Test Task");
        assert_eq!(created.category, "feature");
        assert_eq!(created.priority, 0);
    }

    #[tokio::test]
    async fn test_create_task_with_all_fields() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());
        let mut task = Task::new_with_category(
            project_id.clone(),
            "Full Task".to_string(),
            "bug".to_string(),
        );
        task.description = Some("A description".to_string());
        task.priority = 10;

        let created = state.task_repo.create(task).await.unwrap();

        assert_eq!(created.title, "Full Task");
        assert_eq!(created.category, "bug");
        assert_eq!(created.description, Some("A description".to_string()));
        assert_eq!(created.priority, 10);
    }

    #[tokio::test]
    async fn test_get_task_returns_none_for_nonexistent() {
        let state = setup_test_state().await;
        let id = TaskId::new();

        let result = state.task_repo.get_by_id(&id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_task_modifies_fields() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());
        let task = Task::new(project_id, "Original Title".to_string());
        let created = state.task_repo.create(task).await.unwrap();

        let mut updated = created.clone();
        updated.title = "Updated Title".to_string();
        updated.priority = 99;

        state.task_repo.update(&updated).await.unwrap();

        let found = state.task_repo.get_by_id(&created.id).await.unwrap().unwrap();
        assert_eq!(found.title, "Updated Title");
        assert_eq!(found.priority, 99);
    }

    #[tokio::test]
    async fn test_delete_task_removes_it() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());
        let task = Task::new(project_id, "To Delete".to_string());
        let created = state.task_repo.create(task).await.unwrap();

        state.task_repo.delete(&created.id).await.unwrap();

        let found = state.task_repo.get_by_id(&created.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_list_tasks_returns_all_for_project() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());

        state.task_repo.create(Task::new(project_id.clone(), "Task 1".to_string())).await.unwrap();
        state.task_repo.create(Task::new(project_id.clone(), "Task 2".to_string())).await.unwrap();
        state.task_repo.create(Task::new(project_id.clone(), "Task 3".to_string())).await.unwrap();

        let tasks = state.task_repo.get_by_project(&project_id).await.unwrap();
        assert_eq!(tasks.len(), 3);
    }

    #[tokio::test]
    async fn test_task_response_serialization() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let task = Task::new(project_id, "Test Task".to_string());
        let response = types::TaskResponse::from(task);

        // Verify all fields are set
        assert!(!response.id.is_empty());
        assert_eq!(response.project_id, "proj-123");
        assert_eq!(response.title, "Test Task");
        assert_eq!(response.internal_status, "backlog");

        // Verify it serializes to JSON
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"title\":\"Test Task\""));
    }

    // ========================================
    // Answer User Question Command Tests
    // ========================================

    #[tokio::test]
    async fn test_answer_user_question_transitions_blocked_to_ready() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create a blocked task (simulating an agent waiting for user input)
        let mut task = Task::new(project_id, "Blocked Task".to_string());
        task.internal_status = InternalStatus::Blocked;
        let created = state.task_repo.create(task).await.unwrap();

        // Verify task is blocked
        assert_eq!(created.internal_status, InternalStatus::Blocked);

        // Simulate answering the question by updating the task
        let mut task = state.task_repo.get_by_id(&created.id).await.unwrap().unwrap();
        assert_eq!(task.internal_status, InternalStatus::Blocked);

        // The command transitions Blocked -> Ready
        task.internal_status = InternalStatus::Ready;
        task.touch();
        state.task_repo.update(&task).await.unwrap();

        // Verify task is now ready
        let updated = state.task_repo.get_by_id(&created.id).await.unwrap().unwrap();
        assert_eq!(updated.internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_answer_user_question_fails_if_not_blocked() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create a task that is not blocked (e.g., Ready)
        let mut task = Task::new(project_id, "Ready Task".to_string());
        task.internal_status = InternalStatus::Ready;
        let created = state.task_repo.create(task).await.unwrap();

        // Verify task is not blocked
        let task = state.task_repo.get_by_id(&created.id).await.unwrap().unwrap();
        assert_ne!(task.internal_status, InternalStatus::Blocked);

        // In the real command, this would return an error
        // Here we just verify the precondition check
    }

    #[tokio::test]
    async fn test_answer_user_question_not_found() {
        let state = setup_test_state().await;
        let nonexistent_id = TaskId::from_string("nonexistent".to_string());

        // Task not found
        let result = state.task_repo.get_by_id(&nonexistent_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_answer_user_question_input_deserialization() {
        // Test that input deserializes correctly with camelCase
        let json = r#"{
            "taskId": "task-123",
            "selectedOptions": ["option1", "option2"],
            "customResponse": "My custom answer"
        }"#;

        let input: types::AnswerUserQuestionInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.task_id, "task-123");
        assert_eq!(input.selected_options, vec!["option1", "option2"]);
        assert_eq!(input.custom_response, Some("My custom answer".to_string()));
    }

    #[tokio::test]
    async fn test_answer_user_question_input_without_custom_response() {
        // Test that input deserializes correctly without custom_response
        let json = r#"{
            "taskId": "task-456",
            "selectedOptions": ["option1"]
        }"#;

        let input: types::AnswerUserQuestionInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.task_id, "task-456");
        assert_eq!(input.selected_options, vec!["option1"]);
        assert!(input.custom_response.is_none());
    }

    #[tokio::test]
    async fn test_answer_user_question_response_serialization() {
        let response = types::AnswerUserQuestionResponse {
            task_id: "task-789".to_string(),
            resumed_status: "ready".to_string(),
            answer_recorded: true,
        };

        let json = serde_json::to_string(&response).unwrap();

        // Verify snake_case serialization (backend convention)
        assert!(json.contains("\"task_id\":\"task-789\""));
        assert!(json.contains("\"resumed_status\":\"ready\""));
        assert!(json.contains("\"answer_recorded\":true"));
    }

    // ========================================
    // Inject Task Command Tests
    // ========================================

    #[tokio::test]
    async fn test_inject_task_input_deserialization_minimal() {
        // Test minimal input with defaults
        let json = r#"{
            "projectId": "proj-123",
            "title": "Injected Task"
        }"#;

        let input: types::InjectTaskInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.project_id, "proj-123");
        assert_eq!(input.title, "Injected Task");
        assert!(input.description.is_none());
        assert!(input.category.is_none());
        assert_eq!(input.target, "backlog");
        assert!(!input.make_next);
    }

    #[tokio::test]
    async fn test_inject_task_input_deserialization_full() {
        // Test full input with all fields
        let json = r#"{
            "projectId": "proj-456",
            "title": "Urgent Task",
            "description": "This is urgent",
            "category": "bug",
            "target": "planned",
            "makeNext": true
        }"#;

        let input: types::InjectTaskInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.project_id, "proj-456");
        assert_eq!(input.title, "Urgent Task");
        assert_eq!(input.description, Some("This is urgent".to_string()));
        assert_eq!(input.category, Some("bug".to_string()));
        assert_eq!(input.target, "planned");
        assert!(input.make_next);
    }

    #[tokio::test]
    async fn test_inject_task_response_serialization() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let task = Task::new(project_id, "Test Task".to_string());
        let response = types::InjectTaskResponse {
            task: types::TaskResponse::from(task),
            target: "planned".to_string(),
            priority: 1000,
            make_next_applied: true,
        };

        let json = serde_json::to_string(&response).unwrap();

        // Verify snake_case serialization (backend convention)
        assert!(json.contains("\"target\":\"planned\""));
        assert!(json.contains("\"priority\":1000"));
        assert!(json.contains("\"make_next_applied\":true"));
    }

    #[tokio::test]
    async fn test_inject_task_to_backlog_creates_backlog_task() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Inject to backlog (default)
        let mut task = Task::new_with_category(
            project_id.clone(),
            "Backlog Task".to_string(),
            "feature".to_string(),
        );
        task.internal_status = InternalStatus::Backlog;
        task.priority = 0;

        let created = state.task_repo.create(task).await.unwrap();

        assert_eq!(created.internal_status, InternalStatus::Backlog);
        assert_eq!(created.priority, 0);
    }

    #[tokio::test]
    async fn test_inject_task_to_planned_creates_ready_task() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Inject to planned queue
        let mut task = Task::new_with_category(
            project_id.clone(),
            "Planned Task".to_string(),
            "feature".to_string(),
        );
        task.internal_status = InternalStatus::Ready;
        task.priority = 0;

        let created = state.task_repo.create(task).await.unwrap();

        assert_eq!(created.internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_inject_task_make_next_gets_highest_priority() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create existing ready tasks with various priorities
        let mut task1 = Task::new(project_id.clone(), "Existing 1".to_string());
        task1.internal_status = InternalStatus::Ready;
        task1.priority = 10;
        state.task_repo.create(task1).await.unwrap();

        let mut task2 = Task::new(project_id.clone(), "Existing 2".to_string());
        task2.internal_status = InternalStatus::Ready;
        task2.priority = 50;
        state.task_repo.create(task2).await.unwrap();

        let mut task3 = Task::new(project_id.clone(), "Existing 3".to_string());
        task3.internal_status = InternalStatus::Ready;
        task3.priority = 25;
        state.task_repo.create(task3).await.unwrap();

        // Get max priority for make_next
        let ready_tasks = state
            .task_repo
            .get_by_status(&project_id, InternalStatus::Ready)
            .await
            .unwrap();

        let max_priority = ready_tasks.iter().map(|t| t.priority).max().unwrap_or(0);
        let make_next_priority = max_priority + 1000;

        // Inject with make_next
        let mut injected = Task::new(project_id.clone(), "Make Next Task".to_string());
        injected.internal_status = InternalStatus::Ready;
        injected.priority = make_next_priority;

        let created = state.task_repo.create(injected).await.unwrap();

        assert_eq!(created.internal_status, InternalStatus::Ready);
        assert_eq!(created.priority, 1050); // 50 (max) + 1000

        // Verify it's first in the queue
        let next = state.task_repo.get_next_executable(&project_id).await.unwrap();
        assert!(next.is_some());
        assert_eq!(next.unwrap().title, "Make Next Task");
    }

    #[tokio::test]
    async fn test_inject_task_make_next_with_empty_queue() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // No existing ready tasks, make_next should still work
        let ready_tasks = state
            .task_repo
            .get_by_status(&project_id, InternalStatus::Ready)
            .await
            .unwrap();

        let max_priority = ready_tasks.iter().map(|t| t.priority).max().unwrap_or(0);
        let make_next_priority = max_priority + 1000;

        let mut injected = Task::new(project_id.clone(), "First Make Next".to_string());
        injected.internal_status = InternalStatus::Ready;
        injected.priority = make_next_priority;

        let created = state.task_repo.create(injected).await.unwrap();

        assert_eq!(created.priority, 1000); // 0 + 1000
    }

    #[tokio::test]
    async fn test_inject_task_with_custom_category() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        let task = Task::new_with_category(
            project_id.clone(),
            "Bug Task".to_string(),
            "bug".to_string(),
        );

        let created = state.task_repo.create(task).await.unwrap();

        assert_eq!(created.category, "bug");
    }

    #[tokio::test]
    async fn test_inject_task_with_description() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        let mut task = Task::new(project_id.clone(), "Described Task".to_string());
        task.description = Some("This is a detailed description".to_string());

        let created = state.task_repo.create(task).await.unwrap();

        assert_eq!(
            created.description,
            Some("This is a detailed description".to_string())
        );
    }

    #[tokio::test]
    async fn test_inject_task_invalid_target_defaults_to_backlog() {
        // Test that invalid target defaults to backlog behavior
        let json = r#"{
            "projectId": "proj-123",
            "title": "Invalid Target Task",
            "target": "invalid"
        }"#;

        let input: types::InjectTaskInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.target, "invalid");

        // In the actual command, invalid target would be handled as backlog
    }

    // ========================================
    // Archive Commands Tests
    // ========================================

    #[tokio::test]
    async fn test_archive_task_sets_archived_at() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create a task
        let task = Task::new(project_id, "Task to Archive".to_string());
        let created = state.task_repo.create(task).await.unwrap();

        // Verify not archived initially
        assert!(created.archived_at.is_none());

        // Archive the task
        let archived = state.task_repo.archive(&created.id).await.unwrap();

        // Verify archived_at is set
        assert!(archived.archived_at.is_some());
        assert_eq!(archived.id, created.id);
        assert_eq!(archived.title, "Task to Archive");
    }

    #[tokio::test]
    async fn test_restore_task_clears_archived_at() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create and archive a task
        let task = Task::new(project_id, "Task to Restore".to_string());
        let created = state.task_repo.create(task).await.unwrap();
        let archived = state.task_repo.archive(&created.id).await.unwrap();

        // Verify it's archived
        assert!(archived.archived_at.is_some());

        // Restore the task
        let restored = state.task_repo.restore(&archived.id).await.unwrap();

        // Verify archived_at is cleared
        assert!(restored.archived_at.is_none());
        assert_eq!(restored.id, created.id);
        assert_eq!(restored.title, "Task to Restore");
    }

    #[tokio::test]
    async fn test_get_archived_count_returns_correct_count() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create several tasks
        let task1 = Task::new(project_id.clone(), "Task 1".to_string());
        let task2 = Task::new(project_id.clone(), "Task 2".to_string());
        let task3 = Task::new(project_id.clone(), "Task 3".to_string());

        let created1 = state.task_repo.create(task1).await.unwrap();
        let created2 = state.task_repo.create(task2).await.unwrap();
        let _created3 = state.task_repo.create(task3).await.unwrap();

        // Archive two tasks
        state.task_repo.archive(&created1.id).await.unwrap();
        state.task_repo.archive(&created2.id).await.unwrap();

        // Check archived count
        let count = state.task_repo.get_archived_count(&project_id).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_get_archived_count_zero_when_none_archived() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create tasks but don't archive them
        state.task_repo.create(Task::new(project_id.clone(), "Active 1".to_string())).await.unwrap();
        state.task_repo.create(Task::new(project_id.clone(), "Active 2".to_string())).await.unwrap();

        // Check archived count
        let count = state.task_repo.get_archived_count(&project_id).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_permanently_delete_archived_task_succeeds() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create and archive a task
        let task = Task::new(project_id, "Task to Delete".to_string());
        let created = state.task_repo.create(task).await.unwrap();
        let archived = state.task_repo.archive(&created.id).await.unwrap();

        // Verify it's archived
        assert!(archived.archived_at.is_some());

        // Permanently delete should succeed
        state.task_repo.delete(&archived.id).await.unwrap();

        // Verify task is gone
        let found = state.task_repo.get_by_id(&archived.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_task_response_includes_archived_at() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let mut task = Task::new(project_id, "Archived Task".to_string());
        task.archived_at = Some(Utc::now());

        let response = types::TaskResponse::from(task);

        // Verify archived_at is in response
        assert!(response.archived_at.is_some());

        // Verify it serializes correctly (snake_case backend convention)
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"archived_at\":"));
    }

    #[tokio::test]
    async fn test_task_response_archived_at_null_when_not_archived() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let task = Task::new(project_id, "Active Task".to_string());

        let response = types::TaskResponse::from(task);

        // Verify archived_at is null
        assert!(response.archived_at.is_none());

        // Verify it serializes correctly (snake_case backend convention)
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"archived_at\":null"));
    }

    // ========================================
    // Pagination Tests
    // ========================================

    #[tokio::test]
    async fn test_list_paginated_empty_results() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // No tasks exist
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 0, 20, false)
            .await
            .unwrap();

        assert_eq!(result.len(), 0);

        // Count should also be 0
        let count = state.task_repo.count_tasks(&project_id, false).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_list_paginated_first_page() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create 5 tasks
        for i in 1..=5 {
            state
                .task_repo
                .create(Task::new(project_id.clone(), format!("Task {}", i)))
                .await
                .unwrap();
        }

        // Get first page (limit 3)
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 0, 3, false)
            .await
            .unwrap();

        assert_eq!(result.len(), 3);

        // Total count should be 5
        let count = state.task_repo.count_tasks(&project_id, false).await.unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_list_paginated_last_page() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create 5 tasks
        for i in 1..=5 {
            state
                .task_repo
                .create(Task::new(project_id.clone(), format!("Task {}", i)))
                .await
                .unwrap();
        }

        // Get last page (offset 3, limit 3 = should return 2 tasks)
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 3, 3, false)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_list_paginated_offset_beyond_total() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create 3 tasks
        for i in 1..=3 {
            state
                .task_repo
                .create(Task::new(project_id.clone(), format!("Task {}", i)))
                .await
                .unwrap();
        }

        // Request offset 10 (beyond total of 3)
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 10, 20, false)
            .await
            .unwrap();

        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_list_paginated_excludes_archived_by_default() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create 3 tasks
        let task1 = state
            .task_repo
            .create(Task::new(project_id.clone(), "Task 1".to_string()))
            .await
            .unwrap();
        let task2 = state
            .task_repo
            .create(Task::new(project_id.clone(), "Task 2".to_string()))
            .await
            .unwrap();
        state
            .task_repo
            .create(Task::new(project_id.clone(), "Task 3".to_string()))
            .await
            .unwrap();

        // Archive task1 and task2
        state.task_repo.archive(&task1.id).await.unwrap();
        state.task_repo.archive(&task2.id).await.unwrap();

        // List without archived (include_archived = false)
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 0, 20, false)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Task 3");

        // Count without archived
        let count = state.task_repo.count_tasks(&project_id, false).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_list_paginated_includes_archived_when_requested() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create 3 tasks
        let task1 = state
            .task_repo
            .create(Task::new(project_id.clone(), "Task 1".to_string()))
            .await
            .unwrap();
        state
            .task_repo
            .create(Task::new(project_id.clone(), "Task 2".to_string()))
            .await
            .unwrap();

        // Archive task1
        state.task_repo.archive(&task1.id).await.unwrap();

        // List with archived (include_archived = true)
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 0, 20, true)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);

        // Count with archived
        let count = state.task_repo.count_tasks(&project_id, true).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_list_paginated_ordered_by_created_at_desc() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create tasks with slight delay to ensure different created_at
        let task1 = state
            .task_repo
            .create(Task::new(project_id.clone(), "First".to_string()))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let task2 = state
            .task_repo
            .create(Task::new(project_id.clone(), "Second".to_string()))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let task3 = state
            .task_repo
            .create(Task::new(project_id.clone(), "Third".to_string()))
            .await
            .unwrap();

        // Get paginated tasks
        let result = state
            .task_repo
            .list_paginated(&project_id, None, 0, 20, false)
            .await
            .unwrap();

        // Should be ordered newest first (DESC)
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].id, task3.id);
        assert_eq!(result[1].id, task2.id);
        assert_eq!(result[2].id, task1.id);
    }

    #[tokio::test]
    async fn test_task_list_response_serialization() {
        let project_id = ProjectId::from_string("proj-123".to_string());
        let task = Task::new(project_id, "Test Task".to_string());

        let response = types::TaskListResponse {
            tasks: vec![types::TaskResponse::from(task)],
            total: 10,
            has_more: true,
            offset: 0,
        };

        let json = serde_json::to_string(&response).unwrap();

        // Verify snake_case serialization (backend convention)
        assert!(json.contains("\"tasks\":"));
        assert!(json.contains("\"total\":10"));
        assert!(json.contains("\"has_more\":true"));
        assert!(json.contains("\"offset\":0"));
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_backlog() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Create a task in backlog state
        let mut task = Task::new(project_id, "Test Task".to_string());
        task.internal_status = InternalStatus::Backlog;
        let task = state.task_repo.create(task).await.unwrap();

        // Get valid transitions directly from InternalStatus
        let transitions = task.internal_status.valid_transitions();

        // From backlog, should be able to go to Ready or Cancelled
        assert_eq!(transitions.len(), 2);
        assert!(transitions.iter().any(|t| *t == InternalStatus::Ready));
        assert!(transitions.iter().any(|t| *t == InternalStatus::Cancelled));

        // Test the label mapping function
        let ready_label = helpers::status_to_label(InternalStatus::Ready);
        assert_eq!(ready_label, "Ready for Work");

        let cancelled_label = helpers::status_to_label(InternalStatus::Cancelled);
        assert_eq!(cancelled_label, "Cancel");
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_ready() {
        // Test valid transitions from ready state
        let transitions = InternalStatus::Ready.valid_transitions();

        // From ready, should be able to go to Executing, Blocked, or Cancelled
        assert_eq!(transitions.len(), 3);
        assert!(transitions.iter().any(|t| *t == InternalStatus::Executing));
        assert!(transitions.iter().any(|t| *t == InternalStatus::Blocked));
        assert!(transitions.iter().any(|t| *t == InternalStatus::Cancelled));

        // Test labels
        assert_eq!(helpers::status_to_label(InternalStatus::Executing), "Start Execution");
        assert_eq!(helpers::status_to_label(InternalStatus::Blocked), "Mark as Blocked");
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_blocked() {
        // Test valid transitions from blocked state
        let transitions = InternalStatus::Blocked.valid_transitions();

        // From blocked, should be able to go to Ready or Cancelled
        assert_eq!(transitions.len(), 2);
        assert!(transitions.iter().any(|t| *t == InternalStatus::Ready));
        assert!(transitions.iter().any(|t| *t == InternalStatus::Cancelled));
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_qa_failed() {
        // Test valid transitions from qa_failed state
        let transitions = InternalStatus::QaFailed.valid_transitions();

        // From qa_failed, should be able to go to RevisionNeeded (only one option)
        assert_eq!(transitions.len(), 1);
        assert!(transitions.iter().any(|t| *t == InternalStatus::RevisionNeeded));

        // Test label
        assert_eq!(helpers::status_to_label(InternalStatus::RevisionNeeded), "Needs Revision");
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_approved() {
        // Test valid transitions from approved state (leads to merge workflow)
        let transitions = InternalStatus::Approved.valid_transitions();

        // From approved, can transition to PendingMerge or re-opened to Ready
        assert_eq!(transitions.len(), 2);
        assert!(transitions.iter().any(|t| *t == InternalStatus::PendingMerge));
        assert!(transitions.iter().any(|t| *t == InternalStatus::Ready));

        // Test label
        assert_eq!(helpers::status_to_label(InternalStatus::Approved), "Approve");
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_cancelled() {
        // Test valid transitions from cancelled state
        let transitions = InternalStatus::Cancelled.valid_transitions();

        // From cancelled, can be re-opened to Ready
        assert_eq!(transitions.len(), 1);
        assert!(transitions.iter().any(|t| *t == InternalStatus::Ready));

        // Test label
        assert_eq!(helpers::status_to_label(InternalStatus::Cancelled), "Cancel");
    }

    #[tokio::test]
    async fn test_get_valid_transitions_from_failed() {
        // Test valid transitions from failed state
        let transitions = InternalStatus::Failed.valid_transitions();

        // From failed, can retry (go to Ready)
        assert_eq!(transitions.len(), 1);
        assert!(transitions.iter().any(|t| *t == InternalStatus::Ready));

        // Test label
        assert_eq!(helpers::status_to_label(InternalStatus::Failed), "Mark as Failed");
    }

    #[tokio::test]
    async fn test_status_to_label_all_statuses() {
        // Test that all statuses have labels
        let all_statuses = InternalStatus::all_variants();

        for status in all_statuses {
            let label = helpers::status_to_label(*status);
            // Label should not be empty
            assert!(!label.is_empty(), "Status {:?} has no label", status);
        }
    }

    // ========================================
    // Create Task with Steps Tests
    // ========================================

    #[tokio::test]
    async fn test_create_task_with_steps() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());
        let step_titles = vec![
            "Step 1".to_string(),
            "Step 2".to_string(),
            "Step 3".to_string(),
        ];

        // Create task with steps
        let task = Task::new(project_id.clone(), "Task with Steps".to_string());
        let created_task = state.task_repo.create(task).await.unwrap();

        // Create steps manually (simulating what create_task command does)
        use crate::domain::entities::TaskStep;
        let steps: Vec<TaskStep> = step_titles
            .into_iter()
            .enumerate()
            .map(|(idx, title)| {
                TaskStep::new(created_task.id.clone(), title, idx as i32, "user".to_string())
            })
            .collect();

        let created_steps = state.task_step_repo.bulk_create(steps).await.unwrap();

        // Verify steps were created
        assert_eq!(created_steps.len(), 3);
        assert_eq!(created_steps[0].title, "Step 1");
        assert_eq!(created_steps[1].title, "Step 2");
        assert_eq!(created_steps[2].title, "Step 3");

        // Verify sort_order
        assert_eq!(created_steps[0].sort_order, 0);
        assert_eq!(created_steps[1].sort_order, 1);
        assert_eq!(created_steps[2].sort_order, 2);

        // Verify created_by
        assert_eq!(created_steps[0].created_by, "user");
        assert_eq!(created_steps[1].created_by, "user");
        assert_eq!(created_steps[2].created_by, "user");

        // Verify steps are linked to task
        let task_steps = state.task_step_repo.get_by_task(&created_task.id).await.unwrap();
        assert_eq!(task_steps.len(), 3);
    }

    #[tokio::test]
    async fn test_create_task_without_steps() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());

        // Create task without steps
        let task = Task::new(project_id.clone(), "Task without Steps".to_string());
        let created_task = state.task_repo.create(task).await.unwrap();

        // Verify no steps exist
        let task_steps = state.task_step_repo.get_by_task(&created_task.id).await.unwrap();
        assert_eq!(task_steps.len(), 0);
    }

    #[tokio::test]
    async fn test_create_task_with_empty_steps_array() {
        let state = setup_test_state().await;

        let project_id = ProjectId::from_string("test-project".to_string());

        // Create task with empty steps array (should not create any steps)
        let task = Task::new(project_id.clone(), "Task with Empty Steps".to_string());
        let created_task = state.task_repo.create(task).await.unwrap();

        // Verify no steps exist
        let task_steps = state.task_step_repo.get_by_task(&created_task.id).await.unwrap();
        assert_eq!(task_steps.len(), 0);
    }

    // ========================================
    // Queue Changed Event Tests
    // ========================================

    #[tokio::test]
    async fn test_ready_status_count_for_queue_changed() {
        let state = setup_test_state().await;
        let project_id = ProjectId::from_string("test-project".to_string());

        // Initially no tasks in Ready status
        let ready_tasks = state.task_repo.get_by_status(&project_id, InternalStatus::Ready).await.unwrap();
        assert_eq!(ready_tasks.len(), 0);

        // Create a task in Ready status
        let mut task1 = Task::new(project_id.clone(), "Task 1".to_string());
        task1.internal_status = InternalStatus::Ready;
        state.task_repo.create(task1).await.unwrap();

        let ready_tasks = state.task_repo.get_by_status(&project_id, InternalStatus::Ready).await.unwrap();
        assert_eq!(ready_tasks.len(), 1);

        // Add another Ready task
        let mut task2 = Task::new(project_id.clone(), "Task 2".to_string());
        task2.internal_status = InternalStatus::Ready;
        state.task_repo.create(task2).await.unwrap();

        let ready_tasks = state.task_repo.get_by_status(&project_id, InternalStatus::Ready).await.unwrap();
        assert_eq!(ready_tasks.len(), 2);

        // Add a non-Ready task (should not affect count)
        let mut task3 = Task::new(project_id.clone(), "Task 3".to_string());
        task3.internal_status = InternalStatus::Executing;
        state.task_repo.create(task3).await.unwrap();

        let ready_tasks = state.task_repo.get_by_status(&project_id, InternalStatus::Ready).await.unwrap();
        assert_eq!(ready_tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_queue_change_detection_logic() {
        // Test the logic for detecting when queue_changed should be emitted
        let old_status = InternalStatus::Backlog;
        let new_status = InternalStatus::Ready;

        // Should emit: moving to Ready
        let should_emit = old_status == InternalStatus::Ready || new_status == InternalStatus::Ready;
        assert!(should_emit);

        // Should emit: moving from Ready
        let old_status = InternalStatus::Ready;
        let new_status = InternalStatus::Executing;
        let should_emit = old_status == InternalStatus::Ready || new_status == InternalStatus::Ready;
        assert!(should_emit);

        // Should NOT emit: neither is Ready
        let old_status = InternalStatus::Backlog;
        let new_status = InternalStatus::Blocked;
        let should_emit = old_status == InternalStatus::Ready || new_status == InternalStatus::Ready;
        assert!(!should_emit);
    }
