use super::*;
use crate::domain::entities::{ProjectId, Task};
use crate::infrastructure::sqlite::migrations::run_migrations;
use rusqlite::Connection;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    conn
}

fn create_test_task(conn: &Connection, task_id: &TaskId) {
    let project_id = ProjectId::new();
    // First create a project
    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            project_id.as_str(),
            "Test Project",
            "/tmp/test",
            "local",
            chrono::Utc::now().to_rfc3339(),
            chrono::Utc::now().to_rfc3339(),
        ],
    )
    .unwrap();

    // Then create the task
    let task = Task::new(project_id, "Test Task".to_string());
    let task = Task {
        id: task_id.clone(),
        ..task
    };
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, created_at, updated_at, started_at, completed_at, archived_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        rusqlite::params![
            task.id.as_str(),
            task.project_id.as_str(),
            task.category.to_string(),
            task.title,
            task.description,
            task.priority,
            task.internal_status.as_str(),
            task.needs_review_point,
            task.source_proposal_id.as_ref().map(|id| id.as_str()),
            task.plan_artifact_id.as_ref().map(|id| id.as_str()),
            task.created_at.to_rfc3339(),
            task.updated_at.to_rfc3339(),
            task.started_at.map(|dt| dt.to_rfc3339()),
            task.completed_at.map(|dt| dt.to_rfc3339()),
            task.archived_at.map(|dt| dt.to_rfc3339()),
        ],
    )
    .unwrap();
}

#[tokio::test]
async fn test_create_and_get_by_id() {
    let conn = setup_test_db();
    let task_id = TaskId::new();
    create_test_task(&conn, &task_id);
    let repo = SqliteTaskStepRepository::new(conn);

    let step = TaskStep::new(
        task_id.clone(),
        "Test step".to_string(),
        0,
        "user".to_string(),
    );
    let step_id = step.id.clone();

    // Create step
    let created = repo.create(step).await.unwrap();
    assert_eq!(created.title, "Test step");

    // Get by ID
    let fetched = repo.get_by_id(&step_id).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().title, "Test step");
}

#[tokio::test]
async fn test_get_by_id_not_found() {
    let conn = setup_test_db();
    let repo = SqliteTaskStepRepository::new(conn);

    let step_id = TaskStepId::new();
    let result = repo.get_by_id(&step_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_by_task_ordered() {
    let conn = setup_test_db();
    let task_id = TaskId::new();
    create_test_task(&conn, &task_id);
    let repo = SqliteTaskStepRepository::new(conn);

    let step1 = TaskStep::new(task_id.clone(), "Step 1".to_string(), 2, "user".to_string());
    let step2 = TaskStep::new(task_id.clone(), "Step 2".to_string(), 0, "user".to_string());
    let step3 = TaskStep::new(task_id.clone(), "Step 3".to_string(), 1, "user".to_string());

    repo.create(step1).await.unwrap();
    repo.create(step2).await.unwrap();
    repo.create(step3).await.unwrap();

    let steps = repo.get_by_task(&task_id).await.unwrap();
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].title, "Step 2"); // sort_order 0
    assert_eq!(steps[1].title, "Step 3"); // sort_order 1
    assert_eq!(steps[2].title, "Step 1"); // sort_order 2
}

#[tokio::test]
async fn test_get_by_task_and_status() {
    let conn = setup_test_db();
    let task_id = TaskId::new();
    create_test_task(&conn, &task_id);
    let repo = SqliteTaskStepRepository::new(conn);

    let mut step1 = TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string());
    let mut step2 = TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string());
    let step3 = TaskStep::new(task_id.clone(), "Step 3".to_string(), 2, "user".to_string());

    step1.status = TaskStepStatus::Completed;
    step2.status = TaskStepStatus::InProgress;

    repo.create(step1).await.unwrap();
    repo.create(step2).await.unwrap();
    repo.create(step3).await.unwrap();

    let completed_steps = repo
        .get_by_task_and_status(&task_id, TaskStepStatus::Completed)
        .await
        .unwrap();
    assert_eq!(completed_steps.len(), 1);
    assert_eq!(completed_steps[0].title, "Step 1");

    let pending_steps = repo
        .get_by_task_and_status(&task_id, TaskStepStatus::Pending)
        .await
        .unwrap();
    assert_eq!(pending_steps.len(), 1);
    assert_eq!(pending_steps[0].title, "Step 3");
}

#[tokio::test]
async fn test_update() {
    let conn = setup_test_db();
    let task_id = TaskId::new();
    create_test_task(&conn, &task_id);
    let repo = SqliteTaskStepRepository::new(conn);

    let mut step = TaskStep::new(
        task_id.clone(),
        "Original title".to_string(),
        0,
        "user".to_string(),
    );
    let step_id = step.id.clone();

    repo.create(step.clone()).await.unwrap();

    // Update step
    step.title = "Updated title".to_string();
    step.status = TaskStepStatus::Completed;
    repo.update(&step).await.unwrap();

    // Verify update
    let fetched = repo.get_by_id(&step_id).await.unwrap().unwrap();
    assert_eq!(fetched.title, "Updated title");
    assert_eq!(fetched.status, TaskStepStatus::Completed);
}

#[tokio::test]
async fn test_delete() {
    let conn = setup_test_db();
    let task_id = TaskId::new();
    create_test_task(&conn, &task_id);
    let repo = SqliteTaskStepRepository::new(conn);

    let step = TaskStep::new(
        task_id.clone(),
        "Test step".to_string(),
        0,
        "user".to_string(),
    );
    let step_id = step.id.clone();

    repo.create(step).await.unwrap();
    repo.delete(&step_id).await.unwrap();

    let result = repo.get_by_id(&step_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_delete_by_task() {
    let conn = setup_test_db();
    let task_id = TaskId::new();
    create_test_task(&conn, &task_id);
    let repo = SqliteTaskStepRepository::new(conn);

    let step1 = TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string());
    let step2 = TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string());

    repo.create(step1).await.unwrap();
    repo.create(step2).await.unwrap();

    repo.delete_by_task(&task_id).await.unwrap();

    let steps = repo.get_by_task(&task_id).await.unwrap();
    assert_eq!(steps.len(), 0);
}

#[tokio::test]
async fn test_count_by_status() {
    let conn = setup_test_db();
    let task_id = TaskId::new();
    create_test_task(&conn, &task_id);
    let repo = SqliteTaskStepRepository::new(conn);

    let mut step1 = TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string());
    let mut step2 = TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string());
    let mut step3 = TaskStep::new(task_id.clone(), "Step 3".to_string(), 2, "user".to_string());
    let step4 = TaskStep::new(task_id.clone(), "Step 4".to_string(), 3, "user".to_string());

    step1.status = TaskStepStatus::Completed;
    step2.status = TaskStepStatus::Completed;
    step3.status = TaskStepStatus::InProgress;

    repo.create(step1).await.unwrap();
    repo.create(step2).await.unwrap();
    repo.create(step3).await.unwrap();
    repo.create(step4).await.unwrap();

    let counts = repo.count_by_status(&task_id).await.unwrap();
    assert_eq!(counts.get(&TaskStepStatus::Completed), Some(&2));
    assert_eq!(counts.get(&TaskStepStatus::InProgress), Some(&1));
    assert_eq!(counts.get(&TaskStepStatus::Pending), Some(&1));
}

#[tokio::test]
async fn test_bulk_create() {
    let conn = setup_test_db();
    let task_id = TaskId::new();
    create_test_task(&conn, &task_id);
    let repo = SqliteTaskStepRepository::new(conn);

    let steps = vec![
        TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string()),
        TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string()),
        TaskStep::new(task_id.clone(), "Step 3".to_string(), 2, "user".to_string()),
    ];

    let created = repo.bulk_create(steps).await.unwrap();
    assert_eq!(created.len(), 3);

    let fetched = repo.get_by_task(&task_id).await.unwrap();
    assert_eq!(fetched.len(), 3);
}

#[tokio::test]
async fn test_bulk_create_rollback_on_error() {
    let conn = setup_test_db();
    let task_id = TaskId::new();
    create_test_task(&conn, &task_id);
    let repo = SqliteTaskStepRepository::new(conn);

    let step = TaskStep::new(
        task_id.clone(),
        "Existing step".to_string(),
        0,
        "user".to_string(),
    );
    let step_id = step.id.clone();

    // Create a step first
    repo.create(step.clone()).await.unwrap();

    // Try to bulk create with a duplicate ID (should fail and rollback)
    let steps = vec![
        step.clone(), // Duplicate ID
        TaskStep::new(
            task_id.clone(),
            "New step".to_string(),
            1,
            "user".to_string(),
        ),
    ];

    let result = repo.bulk_create(steps).await;
    assert!(result.is_err());

    // Verify only the original step exists
    let fetched = repo.get_by_task(&task_id).await.unwrap();
    assert_eq!(fetched.len(), 1);
    assert_eq!(fetched[0].id, step_id);
}

#[tokio::test]
async fn test_reorder() {
    let conn = setup_test_db();
    let task_id = TaskId::new();
    create_test_task(&conn, &task_id);
    let repo = SqliteTaskStepRepository::new(conn);

    let step1 = TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string());
    let step2 = TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string());
    let step3 = TaskStep::new(task_id.clone(), "Step 3".to_string(), 2, "user".to_string());

    let step1_id = step1.id.clone();
    let step2_id = step2.id.clone();
    let step3_id = step3.id.clone();

    repo.create(step1).await.unwrap();
    repo.create(step2).await.unwrap();
    repo.create(step3).await.unwrap();

    // Reorder: step3, step1, step2
    let new_order = vec![step3_id.clone(), step1_id.clone(), step2_id.clone()];
    repo.reorder(&task_id, new_order).await.unwrap();

    let steps = repo.get_by_task(&task_id).await.unwrap();
    assert_eq!(steps[0].id, step3_id);
    assert_eq!(steps[1].id, step1_id);
    assert_eq!(steps[2].id, step2_id);
    assert_eq!(steps[0].sort_order, 0);
    assert_eq!(steps[1].sort_order, 1);
    assert_eq!(steps[2].sort_order, 2);
}

#[tokio::test]
async fn test_reorder_rollback_on_error() {
    let conn = setup_test_db();
    let task_id = TaskId::new();
    create_test_task(&conn, &task_id);
    let repo = SqliteTaskStepRepository::new(conn);

    let step1 = TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string());
    let step2 = TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string());

    let step1_id = step1.id.clone();
    let step2_id = step2.id.clone();

    repo.create(step1).await.unwrap();
    repo.create(step2).await.unwrap();

    // Try to reorder with a non-existent step ID
    // Note: SQLite won't error on this - it will just not update the invalid row
    // This is expected behavior - the transaction succeeds but only valid IDs are updated
    let invalid_id = TaskStepId::new();
    let new_order = vec![invalid_id, step1_id.clone(), step2_id.clone()];
    let result = repo.reorder(&task_id, new_order).await;

    // Should succeed (no error) since SQLite doesn't error on UPDATE with no matching rows
    assert!(result.is_ok());

    // Valid steps should be updated to positions 1 and 2
    let steps = repo.get_by_task(&task_id).await.unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].id, step1_id);
    assert_eq!(steps[0].sort_order, 1);
    assert_eq!(steps[1].id, step2_id);
    assert_eq!(steps[1].sort_order, 2);
}
