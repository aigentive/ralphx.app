use crate::infrastructure::sqlite::SqliteTaskRepository;
use crate::domain::entities::{InternalStatus, ProjectId, Task, TaskId};
use crate::domain::repositories::TaskRepository;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};
use rusqlite::Connection;

fn setup_test_db() -> Connection {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    // Insert a test project (required for foreign key)
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('test-project', 'Test Project', '/test/path')",
        [],
    )
    .unwrap();
    conn
}

// Note: Tests use Task::new() which initializes source_proposal_id and plan_artifact_id to None
// No test changes needed - field handling is already tested via entity tests

fn create_test_task(title: &str) -> Task {
    Task::new_with_category(
        ProjectId::from_string("test-project".to_string()),
        title.to_string(),
        "feature".to_string(),
    )
}

// ==================== CRUD TESTS ====================

#[tokio::test]
async fn test_create_inserts_task_and_returns_it() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let task = create_test_task("Test Task");

    let result = repo.create(task.clone()).await;

    assert!(result.is_ok());
    let created = result.unwrap();
    assert_eq!(created.id, task.id);
    assert_eq!(created.title, "Test Task");
}

#[tokio::test]
async fn test_get_by_id_retrieves_task_correctly() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let task = create_test_task("Test Task");

    repo.create(task.clone()).await.unwrap();
    let result = repo.get_by_id(&task.id).await;

    assert!(result.is_ok());
    let found = result.unwrap();
    assert!(found.is_some());
    let found_task = found.unwrap();
    assert_eq!(found_task.id, task.id);
    assert_eq!(found_task.title, "Test Task");
    assert_eq!(found_task.category, "feature");
}

#[tokio::test]
async fn test_get_by_id_returns_none_for_nonexistent() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let id = TaskId::new();

    let result = repo.get_by_id(&id).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_get_by_project_returns_sorted_tasks() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    // Create tasks with different priorities
    let mut task1 = create_test_task("Low Priority");
    task1.priority = 1;

    let mut task2 = create_test_task("High Priority");
    task2.priority = 10;

    let mut task3 = create_test_task("Medium Priority");
    task3.priority = 5;

    repo.create(task1.clone()).await.unwrap();
    repo.create(task2.clone()).await.unwrap();
    repo.create(task3.clone()).await.unwrap();

    let result = repo.get_by_project(&project_id).await;

    assert!(result.is_ok());
    let tasks = result.unwrap();
    assert_eq!(tasks.len(), 3);
    // Should be sorted by priority DESC
    assert_eq!(tasks[0].title, "High Priority");
    assert_eq!(tasks[1].title, "Medium Priority");
    assert_eq!(tasks[2].title, "Low Priority");
}

#[tokio::test]
async fn test_update_modifies_task_fields() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let mut task = create_test_task("Original Title");

    repo.create(task.clone()).await.unwrap();

    task.title = "Updated Title".to_string();
    task.priority = 99;
    task.description = Some("New description".to_string());

    let update_result = repo.update(&task).await;
    assert!(update_result.is_ok());

    let found = repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(found.title, "Updated Title");
    assert_eq!(found.priority, 99);
    assert_eq!(found.description, Some("New description".to_string()));
}

#[tokio::test]
async fn test_delete_removes_task_from_database() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let task = create_test_task("To Delete");

    repo.create(task.clone()).await.unwrap();

    let delete_result = repo.delete(&task.id).await;
    assert!(delete_result.is_ok());

    let found = repo.get_by_id(&task.id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_create_and_retrieve_preserves_all_fields() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);

    let mut task = create_test_task("Full Task");
    task.description = Some("A description".to_string());
    task.priority = 42;
    task.internal_status = InternalStatus::Ready;

    repo.create(task.clone()).await.unwrap();
    let found = repo.get_by_id(&task.id).await.unwrap().unwrap();

    assert_eq!(found.id, task.id);
    assert_eq!(found.project_id, task.project_id);
    assert_eq!(found.category, task.category);
    assert_eq!(found.title, task.title);
    assert_eq!(found.description, task.description);
    assert_eq!(found.priority, task.priority);
    assert_eq!(found.internal_status, task.internal_status);
}

#[tokio::test]
async fn test_get_by_project_returns_empty_for_no_tasks() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let result = repo.get_by_project(&project_id).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_by_project_only_returns_matching_project() {
    let conn = setup_test_db();

    // Add another project
    {
        let lock = conn;
        lock.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('other-project', 'Other', '/other')",
            [],
        )
        .unwrap();

        let repo = SqliteTaskRepository::new(lock);

        let task1 = create_test_task("Task 1");
        let task2 = Task::new_with_category(
            ProjectId::from_string("other-project".to_string()),
            "Task 2".to_string(),
            "feature".to_string(),
        );

        repo.create(task1).await.unwrap();
        repo.create(task2).await.unwrap();

        let project_id = ProjectId::from_string("test-project".to_string());
        let result = repo.get_by_project(&project_id).await;

        assert!(result.is_ok());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Task 1");
    }
}

// ==================== STATUS OPERATION TESTS ====================

#[tokio::test]
async fn test_persist_status_change_updates_task_status() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let task = create_test_task("Test Task");

    repo.create(task.clone()).await.unwrap();

    let result = repo
        .persist_status_change(
            &task.id,
            InternalStatus::Backlog,
            InternalStatus::Ready,
            "user",
        )
        .await;

    assert!(result.is_ok());

    // Verify task status was updated
    let found = repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(found.internal_status, InternalStatus::Ready);
}

#[tokio::test]
async fn test_persist_status_change_creates_history_record() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let task = create_test_task("Test Task");

    repo.create(task.clone()).await.unwrap();

    repo.persist_status_change(
        &task.id,
        InternalStatus::Backlog,
        InternalStatus::Ready,
        "system",
    )
    .await
    .unwrap();

    let history = repo.get_status_history(&task.id).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].from, InternalStatus::Backlog);
    assert_eq!(history[0].to, InternalStatus::Ready);
    assert_eq!(history[0].trigger, "system");
}

#[tokio::test]
async fn test_status_change_and_history_are_atomic() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let task = create_test_task("Test Task");

    repo.create(task.clone()).await.unwrap();

    // Make multiple status changes
    repo.persist_status_change(
        &task.id,
        InternalStatus::Backlog,
        InternalStatus::Ready,
        "user",
    )
    .await
    .unwrap();

    repo.persist_status_change(
        &task.id,
        InternalStatus::Ready,
        InternalStatus::Executing,
        "agent",
    )
    .await
    .unwrap();

    // Verify both status and history are consistent
    let found = repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(found.internal_status, InternalStatus::Executing);

    let history = repo.get_status_history(&task.id).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].from, InternalStatus::Ready);
    assert_eq!(history[1].to, InternalStatus::Executing);
}

#[tokio::test]
async fn test_get_status_history_returns_transitions_in_order() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let task = create_test_task("Test Task");

    repo.create(task.clone()).await.unwrap();

    // Create a sequence of transitions
    repo.persist_status_change(
        &task.id,
        InternalStatus::Backlog,
        InternalStatus::Ready,
        "step1",
    )
    .await
    .unwrap();

    repo.persist_status_change(
        &task.id,
        InternalStatus::Ready,
        InternalStatus::Executing,
        "step2",
    )
    .await
    .unwrap();

    repo.persist_status_change(
        &task.id,
        InternalStatus::Executing,
        InternalStatus::QaRefining,
        "step3",
    )
    .await
    .unwrap();

    let history = repo.get_status_history(&task.id).await.unwrap();

    assert_eq!(history.len(), 3);
    // Should be in chronological order (oldest first)
    assert_eq!(history[0].trigger, "step1");
    assert_eq!(history[1].trigger, "step2");
    assert_eq!(history[2].trigger, "step3");
}

#[tokio::test]
async fn test_get_status_history_returns_empty_for_no_transitions() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let task = create_test_task("Test Task");

    repo.create(task.clone()).await.unwrap();

    let history = repo.get_status_history(&task.id).await.unwrap();
    assert!(history.is_empty());
}

#[tokio::test]
async fn test_get_by_status_filters_correctly() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let mut task1 = create_test_task("Backlog Task");
    task1.internal_status = InternalStatus::Backlog;

    let mut task2 = create_test_task("Ready Task 1");
    task2.internal_status = InternalStatus::Ready;

    let mut task3 = create_test_task("Ready Task 2");
    task3.internal_status = InternalStatus::Ready;

    let mut task4 = create_test_task("Executing Task");
    task4.internal_status = InternalStatus::Executing;

    repo.create(task1).await.unwrap();
    repo.create(task2).await.unwrap();
    repo.create(task3).await.unwrap();
    repo.create(task4).await.unwrap();

    let ready_tasks = repo
        .get_by_status(&project_id, InternalStatus::Ready)
        .await
        .unwrap();

    assert_eq!(ready_tasks.len(), 2);
    assert!(ready_tasks.iter().all(|t| t.internal_status == InternalStatus::Ready));
}

#[tokio::test]
async fn test_get_by_status_returns_empty_for_no_matches() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let task = create_test_task("Backlog Task");
    repo.create(task).await.unwrap();

    let ready_tasks = repo
        .get_by_status(&project_id, InternalStatus::Ready)
        .await
        .unwrap();

    assert!(ready_tasks.is_empty());
}

// ==================== BLOCKER OPERATION TESTS ====================

#[tokio::test]
async fn test_add_blocker_creates_relationship() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);

    let task1 = create_test_task("Blocked Task");
    let task2 = create_test_task("Blocker Task");

    repo.create(task1.clone()).await.unwrap();
    repo.create(task2.clone()).await.unwrap();

    let result = repo.add_blocker(&task1.id, &task2.id).await;
    assert!(result.is_ok());

    let blockers = repo.get_blockers(&task1.id).await.unwrap();
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0].id, task2.id);
}

#[tokio::test]
async fn test_resolve_blocker_removes_relationship() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);

    let task1 = create_test_task("Blocked Task");
    let task2 = create_test_task("Blocker Task");

    repo.create(task1.clone()).await.unwrap();
    repo.create(task2.clone()).await.unwrap();
    repo.add_blocker(&task1.id, &task2.id).await.unwrap();

    repo.resolve_blocker(&task1.id, &task2.id).await.unwrap();

    let blockers = repo.get_blockers(&task1.id).await.unwrap();
    assert!(blockers.is_empty());
}

#[tokio::test]
async fn test_get_blockers_returns_blocking_tasks() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);

    let task1 = create_test_task("Blocked Task");
    let task2 = create_test_task("Blocker 1");
    let task3 = create_test_task("Blocker 2");

    repo.create(task1.clone()).await.unwrap();
    repo.create(task2.clone()).await.unwrap();
    repo.create(task3.clone()).await.unwrap();

    repo.add_blocker(&task1.id, &task2.id).await.unwrap();
    repo.add_blocker(&task1.id, &task3.id).await.unwrap();

    let blockers = repo.get_blockers(&task1.id).await.unwrap();
    assert_eq!(blockers.len(), 2);
}

#[tokio::test]
async fn test_get_dependents_returns_dependent_tasks() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);

    let blocker = create_test_task("Blocker");
    let dep1 = create_test_task("Dependent 1");
    let dep2 = create_test_task("Dependent 2");

    repo.create(blocker.clone()).await.unwrap();
    repo.create(dep1.clone()).await.unwrap();
    repo.create(dep2.clone()).await.unwrap();

    // dep1 and dep2 are blocked by blocker
    repo.add_blocker(&dep1.id, &blocker.id).await.unwrap();
    repo.add_blocker(&dep2.id, &blocker.id).await.unwrap();

    let dependents = repo.get_dependents(&blocker.id).await.unwrap();
    assert_eq!(dependents.len(), 2);
}

#[tokio::test]
async fn test_get_next_executable_excludes_blocked_tasks() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let mut blocked_task = create_test_task("Blocked Ready");
    blocked_task.internal_status = InternalStatus::Ready;
    blocked_task.priority = 10; // Higher priority

    let mut unblocked_task = create_test_task("Unblocked Ready");
    unblocked_task.internal_status = InternalStatus::Ready;
    unblocked_task.priority = 1; // Lower priority

    let blocker = create_test_task("Blocker");

    repo.create(blocked_task.clone()).await.unwrap();
    repo.create(unblocked_task.clone()).await.unwrap();
    repo.create(blocker.clone()).await.unwrap();

    repo.add_blocker(&blocked_task.id, &blocker.id).await.unwrap();

    // Should return unblocked task even though blocked has higher priority
    let next = repo.get_next_executable(&project_id).await.unwrap();
    assert!(next.is_some());
    assert_eq!(next.unwrap().id, unblocked_task.id);
}

#[tokio::test]
async fn test_get_next_executable_returns_highest_priority_ready() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let mut low = create_test_task("Low");
    low.internal_status = InternalStatus::Ready;
    low.priority = 1;

    let mut high = create_test_task("High");
    high.internal_status = InternalStatus::Ready;
    high.priority = 10;

    repo.create(low).await.unwrap();
    repo.create(high.clone()).await.unwrap();

    let next = repo.get_next_executable(&project_id).await.unwrap();
    assert!(next.is_some());
    assert_eq!(next.unwrap().id, high.id);
}

#[tokio::test]
async fn test_get_next_executable_returns_none_when_no_ready_tasks() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let task = create_test_task("Backlog Task"); // Default status is Backlog
    repo.create(task).await.unwrap();

    let next = repo.get_next_executable(&project_id).await.unwrap();
    assert!(next.is_none());
}

// ==================== ARCHIVE OPERATION TESTS ====================

#[tokio::test]
async fn test_archive_sets_archived_at() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let task = create_test_task("Task to Archive");

    repo.create(task.clone()).await.unwrap();

    let archived = repo.archive(&task.id).await.unwrap();
    assert!(archived.archived_at.is_some());

    let found = repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert!(found.archived_at.is_some());
}

#[tokio::test]
async fn test_restore_clears_archived_at() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let task = create_test_task("Task to Archive and Restore");

    repo.create(task.clone()).await.unwrap();
    repo.archive(&task.id).await.unwrap();

    let restored = repo.restore(&task.id).await.unwrap();
    assert!(restored.archived_at.is_none());

    let found = repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert!(found.archived_at.is_none());
}

#[tokio::test]
async fn test_get_archived_count_returns_correct_count() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let task1 = create_test_task("Task 1");
    let task2 = create_test_task("Task 2");
    let task3 = create_test_task("Task 3");

    repo.create(task1.clone()).await.unwrap();
    repo.create(task2.clone()).await.unwrap();
    repo.create(task3.clone()).await.unwrap();

    // Archive two tasks
    repo.archive(&task1.id).await.unwrap();
    repo.archive(&task2.id).await.unwrap();

    let count = repo.get_archived_count(&project_id).await.unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_get_by_project_filtered_excludes_archived_by_default() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let task1 = create_test_task("Active Task");
    let task2 = create_test_task("Archived Task");

    repo.create(task1.clone()).await.unwrap();
    repo.create(task2.clone()).await.unwrap();
    repo.archive(&task2.id).await.unwrap();

    let active_tasks = repo
        .get_by_project_filtered(&project_id, false)
        .await
        .unwrap();

    assert_eq!(active_tasks.len(), 1);
    assert_eq!(active_tasks[0].title, "Active Task");
}

#[tokio::test]
async fn test_get_by_project_filtered_includes_archived_when_requested() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let task1 = create_test_task("Active Task");
    let task2 = create_test_task("Archived Task");

    repo.create(task1.clone()).await.unwrap();
    repo.create(task2.clone()).await.unwrap();
    repo.archive(&task2.id).await.unwrap();

    let all_tasks = repo
        .get_by_project_filtered(&project_id, true)
        .await
        .unwrap();

    assert_eq!(all_tasks.len(), 2);
}

#[tokio::test]
async fn test_archive_and_restore_updates_updated_at() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let task = create_test_task("Task");

    repo.create(task.clone()).await.unwrap();
    let original = repo.get_by_id(&task.id).await.unwrap().unwrap();

    // Archive
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    repo.archive(&task.id).await.unwrap();
    let archived = repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert!(archived.updated_at > original.updated_at);

    // Restore
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    repo.restore(&task.id).await.unwrap();
    let restored = repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert!(restored.updated_at > archived.updated_at);
}

// ==================== SEARCH OPERATION TESTS ====================

#[tokio::test]
async fn test_search_by_title() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let task1 = create_test_task("Implement authentication");
    let task2 = create_test_task("Add user login");
    let task3 = create_test_task("Fix database bug");

    repo.create(task1.clone()).await.unwrap();
    repo.create(task2.clone()).await.unwrap();
    repo.create(task3.clone()).await.unwrap();

    // Search for "auth" - should match "authentication"
    let results = repo.search(&project_id, "auth", false).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, task1.id);
}

#[tokio::test]
async fn test_search_by_description() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let mut task1 = create_test_task("Task One");
    task1.description = Some("This task implements authentication".to_string());

    let mut task2 = create_test_task("Task Two");
    task2.description = Some("This task adds logging".to_string());

    repo.create(task1.clone()).await.unwrap();
    repo.create(task2.clone()).await.unwrap();

    // Search for "authentication" - should match description
    let results = repo.search(&project_id, "authentication", false).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, task1.id);
}

#[tokio::test]
async fn test_search_case_insensitive() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let task = create_test_task("Add USER Authentication");
    repo.create(task.clone()).await.unwrap();

    // Search with lowercase - should match
    let results = repo.search(&project_id, "user", false).await.unwrap();
    assert_eq!(results.len(), 1);

    // Search with uppercase - should also match
    let results = repo.search(&project_id, "USER", false).await.unwrap();
    assert_eq!(results.len(), 1);

    // Search with mixed case - should also match
    let results = repo.search(&project_id, "UsEr", false).await.unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_search_returns_no_results_for_no_match() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let task = create_test_task("Add user login");
    repo.create(task.clone()).await.unwrap();

    // Search for something that doesn't exist
    let results = repo.search(&project_id, "nonexistent", false).await.unwrap();
    assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_search_excludes_archived_by_default() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let task1 = create_test_task("Active authentication task");
    let task2 = create_test_task("Archived authentication task");

    repo.create(task1.clone()).await.unwrap();
    repo.create(task2.clone()).await.unwrap();
    repo.archive(&task2.id).await.unwrap();

    // Search without including archived - should only find active task
    let results = repo.search(&project_id, "authentication", false).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, task1.id);
}

#[tokio::test]
async fn test_search_includes_archived_when_requested() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let task1 = create_test_task("Active authentication task");
    let task2 = create_test_task("Archived authentication task");

    repo.create(task1.clone()).await.unwrap();
    repo.create(task2.clone()).await.unwrap();
    repo.archive(&task2.id).await.unwrap();

    // Search with including archived - should find both tasks
    let results = repo.search(&project_id, "authentication", true).await.unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_search_matches_partial_strings() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);
    let project_id = ProjectId::from_string("test-project".to_string());

    let task = create_test_task("Implement user authentication system");
    repo.create(task.clone()).await.unwrap();

    // Search for partial match
    let results = repo.search(&project_id, "authen", false).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, task.id);
}

// ==================== BLOCKED REASON TESTS ====================

#[tokio::test]
async fn test_create_preserves_blocked_reason() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);

    let mut task = create_test_task("Blocked Task");
    task.internal_status = InternalStatus::Blocked;
    task.blocked_reason = Some("Waiting for API design".to_string());

    repo.create(task.clone()).await.unwrap();
    let found = repo.get_by_id(&task.id).await.unwrap().unwrap();

    assert_eq!(found.blocked_reason, Some("Waiting for API design".to_string()));
    assert_eq!(found.internal_status, InternalStatus::Blocked);
}

#[tokio::test]
async fn test_update_preserves_blocked_reason() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);

    let mut task = create_test_task("Task");
    repo.create(task.clone()).await.unwrap();

    // Update to blocked with reason
    task.internal_status = InternalStatus::Blocked;
    task.blocked_reason = Some("Waiting for dependency".to_string());
    repo.update(&task).await.unwrap();

    let found = repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert_eq!(found.blocked_reason, Some("Waiting for dependency".to_string()));
    assert_eq!(found.internal_status, InternalStatus::Blocked);
}

#[tokio::test]
async fn test_update_clears_blocked_reason() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);

    let mut task = create_test_task("Task");
    task.internal_status = InternalStatus::Blocked;
    task.blocked_reason = Some("Waiting for something".to_string());
    repo.create(task.clone()).await.unwrap();

    // Unblock - clear the reason
    task.internal_status = InternalStatus::Ready;
    task.blocked_reason = None;
    repo.update(&task).await.unwrap();

    let found = repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert!(found.blocked_reason.is_none());
    assert_eq!(found.internal_status, InternalStatus::Ready);
}

#[tokio::test]
async fn test_blocked_reason_defaults_to_none() {
    let conn = setup_test_db();
    let repo = SqliteTaskRepository::new(conn);

    let task = create_test_task("Normal Task");
    repo.create(task.clone()).await.unwrap();

    let found = repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert!(found.blocked_reason.is_none());
}
